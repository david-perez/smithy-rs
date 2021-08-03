/*
 * Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
 * SPDX-License-Identifier: Apache-2.0.
 */

//! Named Provider Factory
//!
//! During profile file execution, providers can be loaded given a name like `Ec2InstanceProvider`

use aws_auth::provider::env::EnvironmentVariableCredentialsProvider;
use aws_auth::provider::AsyncProvideCredentials;
use aws_types::os_shim_internal::{Env, Fs};
use std::borrow::Cow;
use std::collections::HashMap;

pub struct NamedProviderFactory {
    inner: HashMap<Cow<'static, str>, Box<dyn AsyncProvideCredentials>>,
}

impl NamedProviderFactory {
    fn provider(&self, name: &str) -> Option<&dyn AsyncProvideCredentials> {
        self.inner.get(name).map(|item| item.as_ref())
    }
}

pub struct Builder {
    env: Option<Env>,
    fs: Option<Fs>,
    extras: HashMap<Cow<'static, str>, Box<dyn AsyncProvideCredentials>>,
}

impl Builder {
    pub fn build(self) -> NamedProviderFactory {
        let env = self.env.unwrap_or(Env::real());
        let _ = self.fs.unwrap_or(Fs::real());
        let mut provider = NamedProviderFactory { inner: self.extras };
        provider.inner.insert(
            Cow::Borrowed("Environment"),
            Box::new(EnvironmentVariableCredentialsProvider::new_with_env(
                env.clone(),
            )),
        );
        provider
    }
    pub fn env(mut self, env: Env) -> Self {
        self.env = Some(env);
        self
    }

    pub fn fs(mut self, fs: Fs) -> Self {
        self.fs = Some(fs);
        self
    }

    pub fn provider(
        mut self,
        name: impl Into<Cow<'static, str>>,
        provider: impl AsyncProvideCredentials + 'static,
    ) -> Self {
        self.extras.insert(name.into(), Box::new(provider));
        self
    }
}
