/*
 * Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
 * SPDX-License-Identifier: Apache-2.0
 */

import org.gradle.api.tasks.testing.logging.TestExceptionFormat

plugins {
    kotlin("jvm")
    `maven-publish`
}

description = "Generates Rust code from Smithy models"

extra["displayName"] = "Smithy :: Rust :: Codegen :: Simple"

extra["moduleName"] = "software.amazon.smithy.rust.codegen.simple"

group = "software.amazon.smithy.rust.codegen.simple.smithy"

version = "0.1.0"

val smithyVersion: String by project

dependencies {
    implementation(project(":codegen-core"))
}

tasks.compileKotlin { kotlinOptions.jvmTarget = "1.8" }

// Reusable license copySpec
val licenseSpec = copySpec {
    from("${project.rootDir}/LICENSE")
    from("${project.rootDir}/NOTICE")
}

// Configure jars to include license related info
tasks.jar {
    metaInf.with(licenseSpec)
    inputs.property("moduleName", project.name)
    manifest { attributes["Automatic-Module-Name"] = project.name }
}

val sourcesJar by tasks.creating(Jar::class) {
    group = "publishing"
    description = "Assembles Kotlin sources jar"
    archiveClassifier.set("sources")
    from(sourceSets.getByName("main").allSource)
}

publishing {
    publications {
        create<MavenPublication>("default") {
            from(components["java"])
            artifact(sourcesJar)
        }
    }
    repositories { maven { url = uri("$buildDir/repository") } }
}
