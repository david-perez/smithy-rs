/*
 * Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
 * SPDX-License-Identifier: Apache-2.0.
 */

package software.amazon.smithy.rust.codegen.server.smithy.generators

import software.amazon.smithy.model.Model
import software.amazon.smithy.model.shapes.OperationShape
import software.amazon.smithy.rust.codegen.rustlang.RustWriter
import software.amazon.smithy.rust.codegen.rustlang.rust
import software.amazon.smithy.rust.codegen.smithy.RustSymbolProvider

/**
 * OperationRegistryGenerator
 */
class OperationRegistryGenerator(
    private val model: Model,
    private val symbolProvider: RustSymbolProvider,
    private val operations: List<OperationShape>,
) {
    fun render(writer: RustWriter) {
        writer.rust("// Will this work?")
    }
}
