/*
 * Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
 * SPDX-License-Identifier: Apache-2.0
 */

package software.amazon.smithy.rust.codegen.server.smithy.generators

import software.amazon.smithy.codegen.core.Symbol
import software.amazon.smithy.codegen.core.SymbolProvider
import software.amazon.smithy.model.shapes.CollectionShape
import software.amazon.smithy.model.shapes.StructureShape
import software.amazon.smithy.model.shapes.UnionShape
import software.amazon.smithy.model.traits.LengthTrait
import software.amazon.smithy.model.traits.Trait
import software.amazon.smithy.model.traits.UniqueItemsTrait
import software.amazon.smithy.rust.codegen.core.rustlang.RustWriter
import software.amazon.smithy.rust.codegen.core.rustlang.Visibility
import software.amazon.smithy.rust.codegen.core.rustlang.docs
import software.amazon.smithy.rust.codegen.core.rustlang.documentShape
import software.amazon.smithy.rust.codegen.core.rustlang.join
import software.amazon.smithy.rust.codegen.core.rustlang.rust
import software.amazon.smithy.rust.codegen.core.rustlang.rustBlock
import software.amazon.smithy.rust.codegen.core.rustlang.rustTemplate
import software.amazon.smithy.rust.codegen.core.smithy.RuntimeType
import software.amazon.smithy.rust.codegen.core.smithy.expectRustMetadata
import software.amazon.smithy.rust.codegen.core.util.PANIC
import software.amazon.smithy.rust.codegen.core.util.orNull
import software.amazon.smithy.rust.codegen.server.smithy.PubCrateConstraintViolationSymbolProvider
import software.amazon.smithy.rust.codegen.server.smithy.ServerCodegenContext
import software.amazon.smithy.rust.codegen.server.smithy.canReachConstrainedShape
import software.amazon.smithy.rust.codegen.server.smithy.supportedCollectionConstraintTraits
import software.amazon.smithy.rust.codegen.server.smithy.validationErrorMessage

/**
 * [ConstrainedCollectionGenerator] generates a wrapper tuple newtype holding a constrained `std::vec::Vec`.
 * This type can be built from unconstrained values, yielding a `ConstraintViolation` when the input does not satisfy
 * the constraints.
 *
 * The [`length`] and [`uniqueItems`] traits are the only constraint traits applicable to list shapes.
 * TODO(https://github.com/awslabs/smithy-rs/issues/1401):
 * The [`uniqueItems`] trait has not been implemented yet.
 *
 * If [unconstrainedSymbol] is provided, the `MaybeConstrained` trait is implemented for the constrained type, using the
 * [unconstrainedSymbol]'s associated type as the associated type for the trait.
 *
 * [`length`]: https://smithy.io/2.0/spec/constraint-traits.html#length-trait
 * [`uniqueItems`]: https://smithy.io/2.0/spec/constraint-traits.html#smithy-api-uniqueitems-trait
 */
class ConstrainedCollectionGenerator(
    val codegenContext: ServerCodegenContext,
    val writer: RustWriter,
    val shape: CollectionShape,
    private val constraintsInfo: List<TraitInfo>,
    private val unconstrainedSymbol: Symbol? = null,
) {
    private val model = codegenContext.model
    private val constrainedShapeSymbolProvider = codegenContext.constrainedShapeSymbolProvider
    private val publicConstrainedTypes = codegenContext.settings.codegenConfig.publicConstrainedTypes
    private val constraintViolationSymbolProvider =
        with(codegenContext.constraintViolationSymbolProvider) {
            if (publicConstrainedTypes) {
                this
            } else {
                PubCrateConstraintViolationSymbolProvider(this)
            }
        }
    private val symbolProvider = codegenContext.symbolProvider

    fun render() {
        check(constraintsInfo.isNotEmpty()) {
            "`ConstrainedCollectionGenerator` can only be invoked for constrained collections, but this shape was not constrained"
        }

        val name = constrainedShapeSymbolProvider.toSymbol(shape).name
        val inner = "std::vec::Vec<#{ValueSymbol}>"
        val constraintViolation = constraintViolationSymbolProvider.toSymbol(shape)
        val constrainedSymbol = symbolProvider.toSymbol(shape)

        val codegenScope = arrayOf(
            "ValueSymbol" to constrainedShapeSymbolProvider.toSymbol(model.expectShape(shape.member.target)),
            "From" to RuntimeType.From,
            "TryFrom" to RuntimeType.TryFrom,
            "ConstraintViolation" to constraintViolation,
        )

        writer.documentShape(shape, model)
        writer.docs(rustDocsConstrainedTypeEpilogue(name))
        val metadata = constrainedSymbol.expectRustMetadata()
        metadata.render(writer)
        writer.rustTemplate(
            """
            struct $name(pub(crate) $inner);
            """,
            *codegenScope,
        )

        writer.rustBlock("impl $name") {
            if (metadata.visibility == Visibility.PUBLIC) {
                writer.rustTemplate(
                    """
                    /// ${rustDocsInnerMethod(inner)}
                    pub fn inner(&self) -> &$inner {
                        &self.0
                    }
                    """,
                    *codegenScope,
                )
            }
            writer.rustTemplate(
                """
                /// ${rustDocsIntoInnerMethod(inner)}
                pub fn into_inner(self) -> $inner {
                    self.0
                }

                #{ValidationFunctions:W}
                """,
                *codegenScope,
                "ValidationFunctions" to constraintsInfo.map { it.validationFunctionDefinition(constraintViolation, inner) }.join("\n"),
            )
        }

        writer.rustTemplate(
            """
            impl #{TryFrom}<$inner> for $name {
                type Error = #{ConstraintViolation};

                /// ${rustDocsTryFromMethod(name, inner)}
                fn try_from(value: $inner) -> Result<Self, Self::Error> {
                    #{ConstraintChecks:W}

                    Ok(Self(value))
                }
            }

            impl #{From}<$name> for $inner {
                fn from(value: $name) -> Self {
                    value.into_inner()
                }
            }
            """,
            *codegenScope,
            "ConstraintChecks" to constraintsInfo.map { it.tryFromCheck }.join("\n"),
        )

        val innerShape = model.expectShape(shape.member.target)
        if (!publicConstrainedTypes &&
            innerShape.canReachConstrainedShape(model, symbolProvider) &&
            innerShape !is StructureShape &&
            innerShape !is UnionShape
        ) {
            writer.rustTemplate(
                """
                impl #{From}<$name> for #{FullyUnconstrainedSymbol} {
                    fn from(value: $name) -> Self {
                        value
                            .into_inner()
                            .into_iter()
                            .map(|v| v.into())
                            .collect()
                    }
                }
                """,
                *codegenScope,
                "FullyUnconstrainedSymbol" to symbolProvider.toSymbol(shape),
            )
        }

        if (unconstrainedSymbol != null) {
            writer.rustTemplate(
                """
                impl #{ConstrainedTrait} for $name {
                    type Unconstrained = #{UnconstrainedSymbol};
                }
                """,
                "ConstrainedTrait" to RuntimeType.ConstrainedTrait,
                "UnconstrainedSymbol" to unconstrainedSymbol,
            )
        }
    }
}

internal sealed class CollectionTraitInfo {
    data class UniqueItems(val uniqueItemsTrait: UniqueItemsTrait, val memberSymbol: Symbol) : CollectionTraitInfo() {
        override fun toTraitInfo(): TraitInfo =
            TraitInfo(
                tryFromCheck = {
                    rust("let value = Self::check_unique_items(value)?;")
                },
                constraintViolationVariant = {
                    docs("Constraint violation error when the list does not contain unique items")
                    // TODO Rationale.
                    // TODO Rename to `duplicate_indices`.
                    rustTemplate(
                        """
                        UniqueItems {
                            /// A vector of indices into `original` pointing to all duplicate items. This vector has
                            /// at least two elements.
                            /// More specifically, for every element `idx_1` in `duplicates`, there exists another
                            /// distinct element `idx_2` such that `original[idx_1] == original[idx_2]` is `true`.
                            /// Nothing is guaranteed about the order of the indices.
                            duplicates: #{Vec}<usize>,
                            /// The original vector, that contains duplicate items.
                            original: #{Vec}<#{MemberSymbol}>,
                        }
                        """,
                        "Vec" to RuntimeType.Vec,
                        "String" to RuntimeType.String,
                        "MemberSymbol" to memberSymbol,
                    )
                },
                asValidationExceptionField = {
                    // smithy-typescript echoes back one instance of each repeated element after sorting them [0]:
                    //
                    // I think we shouldn't do this for several reasons:
                    //
                    // 1. In Rust we can't sort the elements to provide a stable message because they may not be `Ord`.
                    // 2. If we return the elements' serialized representation, we'd have to generate serializers
                    //    for all shapes in the closure of `@uniqueItems` lists just for this use case. These are
                    //    typically shapes for which we only generate deserializers.
                    // 3. Getting back one instance of each repeated element is not helpful:
                    //        - The elements may be big (complex structures); smithy-typescript truncates them [1]. The
                    //          caller might thus may not even get to see the repeated elements.
                    //        - The caller does not know how many times each duplicate element occurred.
                    //        - The caller does not know in which positions each duplicate element occurred.
                    //
                    // I think a better error message is to just return the indices of the duplicate elements: so, the
                    // list:
                    //
                    //     ["a", "b", "a", "b", "c"]
                    //
                    // Would return:
                    //
                    //     [0, 1, 2, 3]
                    //
                    // An even better representation would be to return _groups_ of indices (with size >= 2), one per
                    // equivalence class:
                    //
                    //     [[0, 2], [1, 3]]
                    //
                    // However, this latter representation comes at a non-negligible (?) performance cost, namely, the
                    // allocation of one vector per equivalence class. Judging by how many clients are really interested
                    // in parsing these (none?), perhaps it's sufficient to just return the "flattened" indices.
                    //
                    // [0]: https://github.com/awslabs/smithy-typescript/blob/517c85f8baccf0e5334b4e66d8786bdb5791c595/smithy-typescript-ssdk-libs/server-common/src/validation/validators.ts#L310
                    // [1]: https://github.com/awslabs/smithy-typescript/blob/517c85f8baccf0e5334b4e66d8786bdb5791c595/smithy-typescript-ssdk-libs/server-common/src/validation/index.ts#L106-L111
                    rust(
                        """
                        Self::UniqueItems { duplicates, .. } => 
                            crate::model::ValidationExceptionField {
                                message: format!("${uniqueItemsTrait.validationErrorMessage()}", &duplicates, &path),
                                path,
                            },
                        """,
                    )
                },
                validationFunctionDefinition = { constraintViolation, _ ->
                    {
                        // TODO Explain why we can use `HashMap`.
                        // TODO Unit tests.
                        // TODO Explain we could easily get groups.
                        // TODO Document algorithm, extra Vec, and complexity.
                        rustTemplate(
                            """
                            fn check_unique_items(items: #{Vec}<#{MemberSymbol}>) -> Result<#{Vec}<#{MemberSymbol}>, #{ConstraintViolation}> {
                                let mut seen = #{HashMap}::new();
                                let mut duplicates = #{Vec}::new();
                                for idx in 0..items.len() {
                                    if let Some(prev_idx) = seen.insert(&items[idx], idx) {
                                        duplicates.push(prev_idx);
                                    }
                                }

                                let mut last_duplicates = #{Vec}::new();
                                for idx in &duplicates {
                                    if let Some(prev_idx) = seen.remove(&items[*idx]) {
                                        last_duplicates.push(prev_idx);
                                    }
                                }
                                duplicates.extend(last_duplicates);

                                if !duplicates.is_empty() {
                                    debug_assert!(duplicates.len() >= 2);
                                    Err(#{ConstraintViolation}::UniqueItems { duplicates, original: items })
                                } else {
                                    Ok(items)
                                }
                            }
                            """,
                            "Vec" to RuntimeType.Vec,
                            "HashMap" to RuntimeType.HashMap,
                            "MemberSymbol" to memberSymbol,
                            "ConstraintViolation" to constraintViolation,
                        )
                    }
                },
            )
    }

    data class Length(val lengthTrait: LengthTrait) : CollectionTraitInfo() {
        override fun toTraitInfo(): TraitInfo =
            TraitInfo(
                tryFromCheck = {
                    rust("Self::check_length(value.len())?;")
                },
                constraintViolationVariant = {
                    docs("Constraint violation error when the list doesn't have the required length")
                    rust("Length(usize)")
                },
                asValidationExceptionField = {
                    rust(
                        """
                        Self::Length(length) => crate::model::ValidationExceptionField {
                            message: format!("${lengthTrait.validationErrorMessage()}", length, &path),
                            path,
                        },
                        """,
                    )
                },
                validationFunctionDefinition = { constraintViolation, _ ->
                    {
                        rustTemplate(
                            """
                            fn check_length(length: usize) -> Result<(), #{ConstraintViolation}> {
                                if ${lengthTrait.rustCondition("length")} {
                                    Ok(())
                                } else {
                                    Err(#{ConstraintViolation}::Length(length))
                                }
                            }
                            """,
                            "ConstraintViolation" to constraintViolation,
                        )
                    }
                },
            )
    }

    companion object {
        private fun fromTrait(trait: Trait, shape: CollectionShape, symbolProvider: SymbolProvider): CollectionTraitInfo {
            check(shape.hasTrait(trait.toShapeId()))
            return when (trait) {
                is LengthTrait -> {
                    Length(trait)
                }
                is UniqueItemsTrait -> {
                    UniqueItems(trait, symbolProvider.toSymbol(shape.member))
                }
                else -> {
                    PANIC("CollectionTraitInfo.fromTrait called with unsupported trait $trait")
                }
            }
        }

        fun fromShape(shape: CollectionShape, symbolProvider: SymbolProvider): List<TraitInfo> =
            supportedCollectionConstraintTraits
                .mapNotNull { shape.getTrait(it).orNull() }
                .map { trait -> fromTrait(trait, shape, symbolProvider) }
                .map(CollectionTraitInfo::toTraitInfo)
    }

    abstract fun toTraitInfo(): TraitInfo
}
