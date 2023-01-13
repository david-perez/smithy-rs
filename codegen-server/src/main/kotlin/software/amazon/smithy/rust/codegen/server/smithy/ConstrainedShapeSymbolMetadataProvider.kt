package software.amazon.smithy.rust.codegen.server.smithy

import software.amazon.smithy.model.Model
import software.amazon.smithy.model.shapes.ListShape
import software.amazon.smithy.model.shapes.MapShape
import software.amazon.smithy.model.shapes.MemberShape
import software.amazon.smithy.model.shapes.NumberShape
import software.amazon.smithy.model.shapes.Shape
import software.amazon.smithy.model.shapes.StringShape
import software.amazon.smithy.model.shapes.StructureShape
import software.amazon.smithy.model.shapes.UnionShape
import software.amazon.smithy.rust.codegen.core.rustlang.RustMetadata
import software.amazon.smithy.rust.codegen.core.rustlang.Visibility
import software.amazon.smithy.rust.codegen.core.smithy.RustSymbolProvider
import software.amazon.smithy.rust.codegen.core.smithy.SymbolMetadataProvider
import software.amazon.smithy.rust.codegen.core.smithy.containerDefaultMetadata
import software.amazon.smithy.rust.codegen.core.smithy.expectRustMetadata

// TODO Docs
class ConstrainedShapeSymbolMetadataProvider(
    private val base: RustSymbolProvider,
    private val model: Model,
    private val constrainedTypes: Boolean,
) : SymbolMetadataProvider(base) {

    override fun memberMeta(memberShape: MemberShape) = base.toSymbol(memberShape).expectRustMetadata()
    override fun structureMeta(structureShape: StructureShape) = base.toSymbol(structureShape).expectRustMetadata()
    override fun unionMeta(unionShape: UnionShape) = base.toSymbol(unionShape).expectRustMetadata()
    override fun enumMeta(stringShape: StringShape) = base.toSymbol(stringShape).expectRustMetadata()

    private fun addDerivesAndAdjustVisibilityIfConstrained(shape: Shape): RustMetadata {
        check(shape is ListShape || shape is MapShape || shape is StringShape || shape is NumberShape)

        val baseMetadata = base.toSymbol(shape).expectRustMetadata()
        var derives = baseMetadata.derives
        val additionalAttributes = baseMetadata.additionalAttributes.toMutableList()

        if (shape.canReachConstrainedShape(model, base)) {
            derives += containerDefaultMetadata(shape, model).derives
        }

        val visibility = Visibility.publicIf(constrainedTypes, Visibility.PUBCRATE)
        return RustMetadata(derives, additionalAttributes, visibility)
    }

    override fun listMeta(listShape: ListShape): RustMetadata = addDerivesAndAdjustVisibilityIfConstrained(listShape)
    override fun mapMeta(mapShape: MapShape): RustMetadata = addDerivesAndAdjustVisibilityIfConstrained(mapShape)
    override fun stringMeta(stringShape: StringShape): RustMetadata = addDerivesAndAdjustVisibilityIfConstrained(stringShape)
    override fun numberMeta(numberShape: NumberShape): RustMetadata = addDerivesAndAdjustVisibilityIfConstrained(numberShape)
}