/*
 * Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
 * SPDX-License-Identifier: Apache-2.0.
 */

//! Profile File Based Providers
//!
//! Profile file based providers combine two pieces:
//!
//! 1. Parsing and resolution of the assume role chain
//! 2. A user-modifiable hashmap of provider name to provider.
//!
//! Profile file based providers first determine the chain of providers that will be used to load
//! credentials. After determining and validating this chain, a `Vec` of providers will be created.
//!
//! Each subsequent provider will provide boostrap providers to the next provider in order to load
//! the final credentials.
//!
//! This module contains two sub modules:
//! - [`repr`](repr) which contains an abstract representation of a provider chain and the logic to build it
//! - [`exec`](exec) which contains a chain representation of providers to implement passing bootstrapped credentials
//! through a series of providers.
use aws_types::profile::ProfileParseError;
use std::borrow::Cow;

mod exec;
mod repr;

#[derive(Debug)]
#[non_exhaustive]
pub enum ProfileFileError {
    CouldNotParseProfile(ProfileParseError),
    CredentialLoop {
        profiles: Vec<String>,
        next: String,
    },
    MissingCredentialSource {
        profile: String,
        message: Cow<'static, str>,
    },
    InvalidCredentialSource {
        profile: String,
        message: Cow<'static, str>,
    },
    MissingProfile {
        profile: String,
        message: Cow<'static, str>,
    },
    UnknownCredentialSource {
        name: Cow<'static, str>,
    },
}
