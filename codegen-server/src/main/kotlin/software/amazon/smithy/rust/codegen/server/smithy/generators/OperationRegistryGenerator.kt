/*
 * Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
 * SPDX-License-Identifier: Apache-2.0.
 */

package software.amazon.smithy.rust.codegen.server.smithy.generators

import software.amazon.smithy.model.shapes.OperationShape
import software.amazon.smithy.rust.codegen.rustlang.*
import software.amazon.smithy.rust.codegen.smithy.CodegenContext
import software.amazon.smithy.rust.codegen.smithy.RuntimeType
import software.amazon.smithy.rust.codegen.smithy.protocols.HttpBindingResolver
import software.amazon.smithy.rust.codegen.smithy.protocols.HttpTraitHttpBindingResolver
import software.amazon.smithy.rust.codegen.smithy.protocols.ProtocolContentTypes
import software.amazon.smithy.rust.codegen.util.inputShape
import software.amazon.smithy.rust.codegen.util.outputShape
import software.amazon.smithy.rust.codegen.util.toSnakeCase

/**
 * OperationRegistryGenerator
 */
class OperationRegistryGenerator(
    codegenContext: CodegenContext,
    private val operations: List<OperationShape>,
) {
    private val serverCrate = "aws_smithy_http_server"
    private val service = codegenContext.serviceShape
    private val operationNames = operations
        .map { operation -> service.getContextualName(operation).toSnakeCase() }
    private val model = codegenContext.model
    private val symbolProvider = codegenContext.symbolProvider
    private val runtimeConfig = codegenContext.runtimeConfig

    // TODO Is using this like this fine?
    private val httpBindingResolver: HttpBindingResolver =
        HttpTraitHttpBindingResolver(codegenContext.model, ProtocolContentTypes.consistent("application/json"))

    fun render(writer: RustWriter) {
        fromRequestIntoResponseWorkaround(writer)

        Attribute.Derives(setOf(RuntimeType.Debug, RuntimeType.DeriveBuilder)).render(writer)
        // TODO: Should I use `raw` here? I would not have to escape the `#`.
        writer.rust("##[builder(pattern = \"owned\")]")
        // TODO: is this the correct way of getting the service name?
        val operationRegistryName = "${service.getContextualName(service)}OperationRegistry<${operationsGenericArguments()}>"
        writer.rustBlock("""
            pub struct $operationRegistryName
            where
                ${operationsTraitBounds()}
            """.trimIndent()) {
            val members = operationNames
                .mapIndexed { i, operationName -> "$operationName: Fun$i" }
                .joinToString(separator = ",\n")
            rust(members)
        }

        writer.rustBlockTemplate("""
            impl<${operationsGenericArguments()}> From<$operationRegistryName> for #{router}
            where
                ${operationsTraitBounds()}
            """.trimIndent(),
            "router" to RuntimeType.Router(runtimeConfig)) {
            rustBlock("fn from(registry: ${operationRegistryName}) -> Self") {
                val requestSpecsVarNames = operationNames.map { "${it}_request_spec" }
                val requestSpecs = requestSpecsVarNames.zip(operations) { requestSpecVarName, operation ->
                    "let $requestSpecVarName = ${operation.requestSpec()};"
                }.joinToString(separator = "\n")
                val routes = requestSpecsVarNames.zip(operationNames) { requestSpecVarName, operationName ->
                    // TODO Same question as in `OperationShape.requestSpec()`
                    ".route($requestSpecVarName, $serverCrate::routing::operation_handler::operation(registry.$operationName))"
                }.joinToString(separator = "\n")

                rustTemplate("""
                    $requestSpecs
                    #{router}::new()
                        $routes
                    """.trimIndent(), "router" to RuntimeType.Router(runtimeConfig))
            }
        }
    }

    private fun operationsGenericArguments(): String = operations.mapIndexed { i, _ -> "Fun$i, Fut$i"}.joinToString()

    private fun operationsTraitBounds(): String = operations
        .mapIndexed { i, operation -> """
            Fun$i: FnOnce(${symbolProvider.toSymbol(operation.inputShape(model))}) -> Fut$i + Clone + Send + Sync + 'static,
            Fut$i: std::future::Future<Output = ${symbolProvider.toSymbol(operation.outputShape(model))}> + Send
        """.trimIndent()}.joinToString(separator = ",\n")

    // TODO Workaround to generate empty impls for `axum`'s `FromRequest`/`IntoResponse` so that my code compiles.
    //  Will go away once Matteo's PR lands.
    private fun fromRequestIntoResponseWorkaround(writer: RustWriter) {
        writer.rustTemplate(operations.map { it.fromRequestIntoResponseWorkaround() }.joinToString("\n"),
            "FromRequest" to RuntimeType.FromRequest,
            "IntoResponse" to RuntimeType.IntoResponse,
            "Body" to RuntimeType.AxumBody,
            "HttpBody" to RuntimeType.AxumHttpBody,
            "async_trait" to RuntimeType.AsyncTrait,
            "RequestParts" to RuntimeType.RequestParts
        )
    }

    private fun OperationShape.fromRequestIntoResponseWorkaround(): String {
        val input = symbolProvider.toSymbol(this.inputShape(model))
        val output = symbolProvider.toSymbol(this.outputShape(model))

        return """
           impl #{IntoResponse} for $output {
                type Body = #{Body};
                type BodyError = <Self::Body as #{HttpBody}>::Error;

                fn into_response(self) -> http::Response<Self::Body> {
                    todo!();
                }
            }

            ##[#{async_trait}]
            impl<B> #{FromRequest}<B> for $input
            where
                B: Send, // required by `async_trait`
            {
                type Rejection = http::StatusCode;

                async fn from_request(
                    _req: &mut #{RequestParts}<B>,
                ) -> Result<Self, Self::Rejection> {
                    todo!()
                }
            }
        """.trimIndent()
    }

    private fun OperationShape.requestSpec(): String {
        val httpTrait = httpBindingResolver.httpTrait(this)
        val namespace = "$serverCrate::routing::request_spec"

        // TODO Support the `endpoint` trait: https://awslabs.github.io/smithy/1.0/spec/core/endpoint-traits.html#endpoint-trait

        val pathSegments = httpTrait.uri.segments.map {
            "$namespace::PathSegment::" +
                if (it.isGreedyLabel) "Greedy"
                else if (it.isLabel) "Label"
                else "Literal(String::from(\"${it.content}\"))"
        }
        val querySegments = httpTrait.uri.queryLiterals.map {
            "$namespace::QuerySegment::" +
                if (it.value == "") "Key(String::from(\"${it.key}\"))"
                else "KeyValue(String::from(\"${it.key}\"), String::from(\"${it.value}\"))"
        }

        // TODO Note I'm writing out the fully namespaced names of the types, which one would do reaching for `RuntimeType`.
        //  I don't foresee that we will use these types anywhere else in the codebase, so is it really worth adding them
        //  to `RuntimeType.kt`, or creating them here and converting them to strings using `fullyQualifiedName()`,
        //  as opposed to just interpolating strings like I'm doing here? I get the value of using `RuntimeType` when it's
        //  an ubiquitous type used throughout the codebase, since it means it's definition is centralized, making it easy
        //  to modify, but if the use of a type is localized like here, is there any benefit? Interpolating strings reads
        //  clearer in my opinion.
        //  I guess there has to be at least one usage of `RuntimeType`; otherwise the dependency won't appear in `Cargo.toml`.
        //  Is there a way of forcing the dependency to appear in `Cargo.toml` without using `RuntimeType`?
        return """
            $namespace::RequestSpec::new(
                http::Method::${httpTrait.method},
                $namespace::UriSpec {
                    host_prefix: None,
                    path_and_query: $namespace::PathAndQuerySpec {
                        path_segments: $namespace::PathSpec(vec![${pathSegments.joinToString()}]),
                        query_segments: vec![${querySegments.joinToString()}],
                    }
                }
            )""".trimIndent()
    }
}