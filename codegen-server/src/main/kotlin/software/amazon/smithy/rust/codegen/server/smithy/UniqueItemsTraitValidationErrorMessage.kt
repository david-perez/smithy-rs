/*
 * Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
 * SPDX-License-Identifier: Apache-2.0
 */

package software.amazon.smithy.rust.codegen.server.smithy

import software.amazon.smithy.model.traits.UniqueItemsTrait

// TODO Using the `Debug` representation for the duplicate items for now; adjust according to what smithy-typescript
//  prints.
fun UniqueItemsTrait.validationErrorMessage() =
    // We're using the `Debug` representation of `Vec<usize>` here e.g. `[0, 2, 3]`.
    "Value with repeated values at indices {:?} at '{}' failed to satisfy constraint: Member must have unique values"
