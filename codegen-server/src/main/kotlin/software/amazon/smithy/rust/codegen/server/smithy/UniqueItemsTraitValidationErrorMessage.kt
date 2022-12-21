/*
 * Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
 * SPDX-License-Identifier: Apache-2.0
 */

package software.amazon.smithy.rust.codegen.server.smithy

import software.amazon.smithy.model.traits.UniqueItemsTrait

// TODO Using the `Debug` representation for the duplicate items for now; adjust according to what smithy-typescript
//  prints.
fun UniqueItemsTrait.validationErrorMessage() =
    "Value with repeated values {:?} at '{}' failed to satisfy constraint: Member must have unique values"
