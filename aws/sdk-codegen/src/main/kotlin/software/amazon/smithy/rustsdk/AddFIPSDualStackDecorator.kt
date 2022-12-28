/*
 * Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
 * SPDX-License-Identifier: Apache-2.0
 */

package software.amazon.smithy.rustsdk

import software.amazon.smithy.model.Model
import software.amazon.smithy.model.shapes.ServiceShape
import software.amazon.smithy.model.shapes.ShapeId
import software.amazon.smithy.model.shapes.ShapeType
import software.amazon.smithy.model.transform.ModelTransformer
import software.amazon.smithy.rulesengine.language.EndpointRuleSet
import software.amazon.smithy.rulesengine.language.syntax.parameters.Builtins
import software.amazon.smithy.rulesengine.language.syntax.parameters.Parameter
import software.amazon.smithy.rulesengine.language.syntax.parameters.ParameterType
import software.amazon.smithy.rulesengine.traits.ClientContextParamDefinition
import software.amazon.smithy.rulesengine.traits.ClientContextParamsTrait
import software.amazon.smithy.rust.codegen.client.smithy.ClientCodegenContext
import software.amazon.smithy.rust.codegen.client.smithy.customize.ClientCodegenDecorator
import software.amazon.smithy.rust.codegen.client.smithy.endpoint.EndpointRulesetIndex
import software.amazon.smithy.rust.codegen.core.util.getTrait

fun EndpointRuleSet.getBuiltIn(builtIn: Parameter) = parameters.toList().find { it.builtIn == builtIn.builtIn }
fun ClientCodegenContext.getBuiltIn(builtIn: Parameter): Parameter? {
    val idx = EndpointRulesetIndex.of(model)
    val rules = idx.endpointRulesForService(serviceShape) ?: return null
    return rules.getBuiltIn(builtIn)
}

/**
 * For legacy SDKs, there are builtIn parameters that cannot be automatically used as context parameters.
 *
 * However, for the Rust SDK, these parameters can be used directly.
 */
fun Model.promoteBuiltInToContextParam(serviceId: ShapeId, builtInSrc: Parameter): Model {
    val model = this
    val idx = EndpointRulesetIndex.of(model)
    val service = model.expectShape(serviceId, ServiceShape::class.java)
    val rules = idx.endpointRulesForService(service) ?: return model
    // load the builtIn with a matching name from the ruleset allowing for any docs updates
    val builtIn = rules.getBuiltIn(builtInSrc) ?: return model

    return ModelTransformer.create().mapShapes(model) { shape ->
        if (shape !is ServiceShape || shape.id != service.id) {
            shape
        } else {
            val traitBuilder = shape.getTrait<ClientContextParamsTrait>()
                // there is a bug in the return type of the toBuilder method
                ?.let { ClientContextParamsTrait.builder().parameters(it.parameters) }
                ?: ClientContextParamsTrait.builder()
            val contextParamsTrait =
                traitBuilder.putParameter(
                    builtIn.name.asString(),
                    ClientContextParamDefinition.builder().documentation(builtIn.documentation.get()).type(
                        when (builtIn.type!!) {
                            ParameterType.STRING -> ShapeType.STRING
                            ParameterType.BOOLEAN -> ShapeType.BOOLEAN
                        },
                    ).build(),
                ).build()
            shape.toBuilder().removeTrait(ClientContextParamsTrait.ID).addTrait(contextParamsTrait).build()
        }
    }
}

class AddFIPSDualStackDecorator : ClientCodegenDecorator {
    override val name: String = "AddFipsDualStack"
    override val order: Byte = 0

    override fun transformModel(service: ServiceShape, model: Model): Model {
        return model
            .promoteBuiltInToContextParam(service.id, Builtins.FIPS)
            .promoteBuiltInToContextParam(service.id, Builtins.DUALSTACK)
    }
}