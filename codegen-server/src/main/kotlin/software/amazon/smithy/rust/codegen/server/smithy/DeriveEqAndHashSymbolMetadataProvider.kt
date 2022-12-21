package software.amazon.smithy.rust.codegen.server.smithy

import software.amazon.smithy.model.Model
import software.amazon.smithy.model.neighbor.Walker
import software.amazon.smithy.model.shapes.DocumentShape
import software.amazon.smithy.model.shapes.DoubleShape
import software.amazon.smithy.model.shapes.FloatShape
import software.amazon.smithy.model.shapes.ListShape
import software.amazon.smithy.model.shapes.MapShape
import software.amazon.smithy.model.shapes.MemberShape
import software.amazon.smithy.model.shapes.NumberShape
import software.amazon.smithy.model.shapes.Shape
import software.amazon.smithy.model.shapes.StringShape
import software.amazon.smithy.model.shapes.StructureShape
import software.amazon.smithy.model.shapes.UnionShape
import software.amazon.smithy.rust.codegen.core.rustlang.RustMetadata
import software.amazon.smithy.rust.codegen.core.smithy.RuntimeType
import software.amazon.smithy.rust.codegen.core.smithy.RustSymbolProvider
import software.amazon.smithy.rust.codegen.core.smithy.SymbolMetadataProvider
import software.amazon.smithy.rust.codegen.core.smithy.expectRustMetadata

// TODO Docs
// TODO Test
class DeriveEqAndHashSymbolMetadataProvider(
    private val base: RustSymbolProvider,
    val model: Model,
): SymbolMetadataProvider(base) {
    private val walker = Walker(model)

    private fun addDeriveEqAndHashIfPossible(shape: Shape): RustMetadata {
        check(shape !is MemberShape)
        val baseMetadata = base.toSymbol(shape).expectRustMetadata()
        return if (walker.walkShapes(shape)
                .any { it is FloatShape || it is DoubleShape || it is DocumentShape }
        ) {
            baseMetadata
        } else {
            baseMetadata.withDerives(RuntimeType.Eq, RuntimeType.Hash)
        }
    }

    override fun memberMeta(memberShape: MemberShape) = base.toSymbol(memberShape).expectRustMetadata()

    override fun structureMeta(structureShape: StructureShape) = addDeriveEqAndHashIfPossible(structureShape)
    override fun unionMeta(unionShape: UnionShape) = addDeriveEqAndHashIfPossible(unionShape)
    override fun enumMeta(stringShape: StringShape) = addDeriveEqAndHashIfPossible(stringShape)

    override fun listMeta(listShape: ListShape): RustMetadata = addDeriveEqAndHashIfPossible(listShape)
    override fun mapMeta(mapShape: MapShape): RustMetadata = addDeriveEqAndHashIfPossible(mapShape)
    override fun stringMeta(stringShape: StringShape): RustMetadata = addDeriveEqAndHashIfPossible(stringShape)
    override fun numberMeta(numberShape: NumberShape): RustMetadata = addDeriveEqAndHashIfPossible(numberShape)
}
