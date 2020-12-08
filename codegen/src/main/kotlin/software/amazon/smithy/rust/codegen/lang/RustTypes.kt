/*
 * Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
 * SPDX-License-Identifier: Apache-2.0.
 */

package software.amazon.smithy.rust.codegen.lang

import software.amazon.smithy.rust.codegen.smithy.RuntimeType

interface Container {
    val value: RustType
}

/**
 * A hierarchy of types handled by Smithy codegen
 */
sealed class RustType {

    /*
     * Name refers to the top-level type for import purposes
     */
    abstract val name: kotlin.String

    object Bool : RustType() {
        override val name: kotlin.String = "bool"
    }

    object String : RustType() {
        override val name: kotlin.String = "String"
    }

    data class Float(val precision: Int) : RustType() {
        override val name: kotlin.String = "f$precision"
    }

    data class Integer(val precision: Int) : RustType() {
        override val name: kotlin.String = "i$precision"
    }

    data class Vec(val member: RustType) : RustType() {
        override val name: kotlin.String = "Vec"
    }

    data class Slice(val member: RustType) : RustType() {
        override val name: kotlin.String
            get() = ""
    }

    data class HashMap(val key: RustType, val value: RustType) : RustType() {
        // TODO: assert that underneath, the member is a String
        override val name: kotlin.String = "HashMap"
    }

    data class HashSet(val member: RustType) : RustType() {
        // TODO: assert that underneath, the member is a String
        override val name: kotlin.String = SetType
    }

    data class Reference(val lifetime: kotlin.String?, override val value: RustType) : RustType(), Container {
        override val name: kotlin.String = value.name
    }

    data class Option(override val value: RustType) : RustType(), Container {
        override val name: kotlin.String = "Option"
    }

    data class Box(override val value: RustType) : RustType(), Container {
        override val name: kotlin.String = "Box"
    }

    data class Opaque(override val name: kotlin.String) : RustType()

    companion object {
        val SetType = "BTreeSet"
    }
}

fun RustType.render(): String = when (this) {
    is RustType.Bool -> this.name
    is RustType.Float -> this.name
    is RustType.Integer -> this.name
    is RustType.String -> this.name
    is RustType.Vec -> "${this.name}<${this.member.render()}>"
    is RustType.Slice -> "[${this.member.render()}]"
    is RustType.HashMap -> "${this.name}<${this.key.render()}, ${this.value.render()}>"
    is RustType.HashSet -> "${this.name}<${this.member.render()}>"
    is RustType.Reference -> "&${this.lifetime?.let { "'$it" } ?: ""} ${this.value.render()}"
    is RustType.Option -> "${this.name}<${this.value.render()}>"
    is RustType.Box -> "${this.name}<${this.value.render()}>"
    is RustType.Opaque -> this.name
}

/**
 * Returns true if [this] contains [t] anywhere within it's tree. For example,
 * Option<Instant>.contains(Instant) would return true.
 * Option<Instant>.contains(Blob) would return false.
 */
fun <T : RustType> RustType.contains(t: T): Boolean {
    if (t == this) {
        return true
    }

    return when (this) {
        is RustType.Vec -> this.member.contains(t)
        is RustType.HashSet -> this.member.contains(t)
        is RustType.Reference -> this.value.contains(t)
        is RustType.Option -> this.value.contains(t)
        is RustType.Box -> this.value.contains(t)
        else -> false
    }
}

inline fun <reified T : Container> RustType.stripOuter(): RustType {
    return when (this) {
        is T -> this.value
        else -> this
    }
}

/**
 * Meta information about a Rust construction (field, struct, or enum)
 */
data class RustMetadata(
    val derives: Derives = Derives.Empty,
    val additionalAttributes: List<Attribute> = listOf(),
    val public: Boolean
) {
    fun withDerives(vararg newDerive: RuntimeType): RustMetadata =
        this.copy(derives = derives.copy(derives = derives.derives + newDerive))

    fun attributes(): List<Attribute> = additionalAttributes + derives
    fun renderAttributes(writer: RustWriter): RustMetadata {
        attributes().forEach {
            it.render(writer)
        }
        return this
    }

    fun renderVisibility(writer: RustWriter): RustMetadata {
        if (public) {
            writer.writeInline("pub ")
        }
        return this
    }

    fun render(writer: RustWriter) {
        renderAttributes(writer)
        renderVisibility(writer)
    }
}

/**
 * [Attributes](https://doc.rust-lang.org/reference/attributes.html) are general free form metadata
 * that are interpreted by the compiler.
 *
 * For example:
 * ```rust
 *
 * #[derive(Clone, PartialEq, Serialize)] // <-- this is an attribute
 * #[serde(serialize_with = "abc")] // <-- this is an attribute
 * struct Abc {
 *   a: i64
 * }
 */
sealed class Attribute {
    abstract fun render(writer: RustWriter)

    companion object {
        /**
         * [non_exhaustive](https://doc.rust-lang.org/reference/attributes/type_system.html#the-non_exhaustive-attribute)
         * indicates that more fields may be added in the future
         */
        val NonExhaustive = Custom("non_exhaustive")
    }
}

data class Derives(val derives: Set<RuntimeType>) : Attribute() {
    override fun render(writer: RustWriter) {
        if (derives.isEmpty()) {
            return
        }
        writer.raw("#[derive(")
        derives.sortedBy { it.name }.forEach { derive ->
            writer.writeInline("#T, ", derive)
        }
        writer.write(")]")
    }

    companion object {
        val Empty = Derives(setOf())
    }
}

data class Custom(val annot: String, val symbols: List<RuntimeType> = listOf()) : Attribute() {
    override fun render(writer: RustWriter) {
        writer.raw("#[")
        writer.writeInline(annot)
        writer.write("]")

        symbols.forEach {
            writer.addDependency(it.dependency)
        }
    }
}