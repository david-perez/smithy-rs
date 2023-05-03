/*
 * Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
 * SPDX-License-Identifier: Apache-2.0
 */

package software.amazon.smithy.rust.codegen.simple.smithy

import software.amazon.smithy.build.PluginContext
import software.amazon.smithy.build.SmithyBuildPlugin
import software.amazon.smithy.codegen.core.SymbolProvider
import software.amazon.smithy.rust.codegen.core.rustlang.RustModule
import software.amazon.smithy.rust.codegen.core.rustlang.Writable
import software.amazon.smithy.rust.codegen.core.rustlang.docs
import software.amazon.smithy.rust.codegen.core.rustlang.escape
import software.amazon.smithy.rust.codegen.core.rustlang.rust
import software.amazon.smithy.rust.codegen.core.rustlang.rustTemplate
import software.amazon.smithy.rust.codegen.core.rustlang.writable
import software.amazon.smithy.rust.codegen.core.smithy.CoreRustSettings
import software.amazon.smithy.rust.codegen.core.smithy.ModuleDocProvider
import software.amazon.smithy.rust.codegen.core.smithy.RuntimeType
import software.amazon.smithy.rust.codegen.core.smithy.RustCrate
import software.amazon.smithy.rust.codegen.core.util.CommandFailed
import software.amazon.smithy.rust.codegen.core.util.runCommand
import java.util.logging.Logger

/**
 * A simple plugin that generates a Rust crate.
 */
class RustSimpleCodegenPlugin : SmithyBuildPlugin {
    private val logger = Logger.getLogger(javaClass.name)

    override fun getName(): String = "rust-simple-codegen"

    override fun execute(context: PluginContext) {
        logger.info("My simple plugin started")

        val symbolProvider = SymbolProvider {
            // This allows you to map shapes in the Smithy model into Rust "symbols",
            // which eventually get rendered as types.
            TODO("We don't need said mapping for such a simple smithy-rs plugin")
        }

        val listStructuresModule = RustModule.public("list_structures")

        val moduleDocProvider = object : ModuleDocProvider {
            override fun docsWriter(module: RustModule.LeafModule): Writable? {
                // This allows you to define Rust module docs for each module you create.
                val strDoc: (String) -> Writable = { str -> writable { docs(escape(str)) } }
                return when (module) {
                   listStructuresModule -> strDoc("This module hosts a function to list the structures in a Smithy model")
                   else -> TODO("Document this module: $module")
                }
            }
        }

        val coreRustSettings = CoreRustSettings.from(context.model, context.settings)

        val rustCrate = RustCrate(
            context.fileManifest,
            symbolProvider,
            coreRustSettings.codegenConfig,
            moduleDocProvider,
        )

        // Write to `lib.rs`.
        rustCrate.lib {
            rust(
                """
                /// Prints `"Hello world!"` to standard output.
                pub fn say_hello() {
                    println!("Hello world!");
                }
                """
            )
        }

        // Write to the `list_structures` module, which will be written to the `lib_structures.rs` file.
        rustCrate.withModule(listStructuresModule) {
            val structureList = context.model.structureShapes.map { it.id }.joinToString(", ").replace("#", "##")

            rust(
                """
                /// Prints all structure shapes in the Smithy model.
                pub fn list_structures() {
                    println!("This model contains the following structure shapes: $structureList");
                }
                """
            )

            rustTemplate(
                """
                /// A function that takes in an `regex::Regex`.
                pub fn process_regex(_req: #{regex}::Regex) {
                    todo!()
                }
                """,
                "regex" to RuntimeType.Regex,
            )
        }

        // Flush our Rust crate to disk.
        rustCrate.finalize(
            coreRustSettings,
            context.model,
            manifestCustomizations = emptyMap(),
            libRsCustomizations = emptyList(),
        )

        // Shell out to `cargo fmt` to format the output crate. This requires that `cargo fmt` is installed in the host.
        // This step is obviously optional.
        try {
            "cargo fmt".runCommand(context.fileManifest.baseDir, timeout = coreRustSettings.codegenConfig.formatTimeoutSeconds.toLong())
        } catch (err: CommandFailed) {
            logger.warning("Failed to run `cargo fmt`: ${err.output}")
        }
    }
}
