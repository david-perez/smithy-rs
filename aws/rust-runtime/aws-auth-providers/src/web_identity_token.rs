/*
 * Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
 * SPDX-License-Identifier: Apache-2.0.
 */

//! Web Identity Token Credential Provider

use aws_hyper::{DynConnector, StandardClient};
use aws_sdk_sts::Region;
use aws_types::os_shim_internal::{Env, Fs};
use aws_types::region::ProvideRegion;

use crate::{must_have_connector, sts_util};
use aws_auth::provider::{AsyncProvideCredentials, BoxFuture, CredentialsError, CredentialsResult};

const ENV_VAR_TOKEN_FILE: &str = "AWS_WEB_IDENTITY_TOKEN_FILE";
const ENV_VAR_ROLE_ARN: &str = "AWS_IAM_ROLE_ARN";
const ENV_VAR_SESSION_NAME: &str = "AWS_IAM_ROLE_SESSION_NAME";

pub struct WebIdentityTokenCredentialProvider {
    env: Env,
    fs: Fs,
    client: StandardClient,
    region: Option<Region>,
}

impl AsyncProvideCredentials for WebIdentityTokenCredentialProvider {
    fn provide_credentials<'a>(&'a self) -> BoxFuture<'a, CredentialsResult>
    where
        Self: 'a,
    {
        Box::pin(self.credentials())
    }
}

impl WebIdentityTokenCredentialProvider {
    async fn credentials(&self) -> CredentialsResult {
        let token_file = self
            .env
            .get(ENV_VAR_TOKEN_FILE)
            .map_err(|_| CredentialsError::CredentialsNotLoaded)?;
        let role_arn = self.env.get(ENV_VAR_ROLE_ARN).map_err(|_| {
            CredentialsError::InvalidConfiguration(
                "AWS_IAM_ROLE_ARN environment variable must be set".into(),
            )
        })?;
        let token = self
            .fs
            .read_to_end(token_file)
            .map_err(|err| CredentialsError::ProviderError(err.into()))?;
        let token = String::from_utf8(token).map_err(|_utf_8_error| {
            CredentialsError::Unhandled("WebIdentityToken was not valid UTF-8".into())
        })?;
        let session_name = self
            .env
            .get(ENV_VAR_SESSION_NAME)
            .unwrap_or_else(|_| sts_util::default_session_name("web-identity-token"));
        let conf = aws_sdk_sts::Config::builder()
            .region(self.region.clone())
            .build();
        let operation = aws_sdk_sts::operation::AssumeRoleWithWebIdentity::builder()
            .role_arn(role_arn)
            .role_session_name(session_name)
            .web_identity_token(token)
            .build()
            .expect("valid operation")
            .make_operation(&conf)
            .expect("valid operation");
        let resp = self
            .client
            .call(operation)
            .await
            .map_err(|sdk_error| CredentialsError::ProviderError(sdk_error.into()))?;
        sts_util::into_credentials(resp.credentials, "WebIdentityToken")
    }
}

#[derive(Default)]
pub struct Builder {
    env: Env,
    fs: Fs,
    connector: Option<DynConnector>,
    region: Option<Region>,
}

impl Builder {
    pub fn fs(mut self, fs: Fs) -> Self {
        self.fs = fs;
        self
    }

    pub fn set_fs(&mut self, fs: Fs) -> &mut Self {
        self.fs = fs;
        self
    }

    pub fn env(mut self, env: Env) -> Self {
        self.env = env;
        self
    }

    pub fn set_env(&mut self, env: Env) -> &mut Self {
        self.env = env;
        self
    }

    pub fn connector(mut self, connector: DynConnector) -> Self {
        self.connector = Some(connector);
        self
    }

    pub fn set_connector(&mut self, connector: Option<DynConnector>) -> &mut Self {
        self.connector = connector;
        self
    }

    pub fn region(mut self, region: &dyn ProvideRegion) -> Self {
        self.region = region.region();
        self
    }

    pub fn set_region(&mut self, region: Option<Region>) -> &mut Self {
        self.region = region;
        self
    }

    pub fn build(self) -> WebIdentityTokenCredentialProvider {
        let connector = self.connector.unwrap_or_else(must_have_connector);
        let client = aws_hyper::Builder::<()>::new()
            .map_connector(|_| connector)
            .build();
        WebIdentityTokenCredentialProvider {
            env: self.env,
            fs: self.fs,
            client,
            region: self.region,
        }
    }
}

#[cfg(test)]
mod test {
    use crate::web_identity_token::{
        Builder, ENV_VAR_ROLE_ARN, ENV_VAR_SESSION_NAME, ENV_VAR_TOKEN_FILE,
    };
    use aws_auth::provider::CredentialsError;
    use aws_hyper::DynConnector;
    use aws_sdk_sts::Region;
    use aws_types::os_shim_internal::{Env, Fs};
    use smithy_client::dvr;
    use smithy_client::dvr::NetworkTraffic;
    use std::collections::HashMap;
    use std::error::Error;
    use std::time::{Duration, UNIX_EPOCH};

    #[tokio::test]
    async fn e2e_test() -> Result<(), Box<dyn Error>> {
        let env = Env::from_slice(&[
            (ENV_VAR_TOKEN_FILE, "/token.jwt"),
            (ENV_VAR_ROLE_ARN, "arn:aws:iam::123456789123:role/test-role"),
            (ENV_VAR_SESSION_NAME, "test-session"),
        ]);
        let fs = Fs::from_test_dir("test-data/web-identity-token", "/");
        let traffic: NetworkTraffic = serde_json::from_str(&std::fs::read_to_string(
            "test-data/web-identity-token/http-traffic.json",
        )?)?;
        let connector = dvr::ReplayingConnection::new(traffic.events().clone());
        let provider = Builder::default()
            .region(&Region::new("us-east-1"))
            .fs(fs)
            .env(env)
            .connector(DynConnector::new(connector.clone()))
            .build();
        let creds = provider.credentials().await?;
        assert_eq!(creds.access_key_id(), "AKIDTEST");
        assert_eq!(creds.secret_access_key(), "SECRETKEYTEST");
        assert_eq!(creds.session_token(), Some("SESSIONTOKEN_TEST"));
        assert_eq!(
            creds.expiry(),
            Some(UNIX_EPOCH + Duration::from_secs(1629147173))
        );
        let reqs = connector.take_requests();
        assert_eq!(reqs.len(), 1);
        Ok(())
    }

    #[tokio::test]
    async fn unloaded_provider() {
        // empty environment
        let env = Env::from_slice(&[]);
        let provider = Builder::default()
            .region(&Region::new("us-east-1"))
            .env(env)
            .build();
        let err = provider
            .credentials()
            .await
            .expect_err("should fail, provider not loaded");
        match err {
            CredentialsError::CredentialsNotLoaded => { /* ok */ }
            _ => panic!("incorrect error variant"),
        }
    }

    #[tokio::test]
    async fn missing_env_var() {
        let env = Env::from_slice(&[(ENV_VAR_TOKEN_FILE, "/token.jwt")]);
        let provider = Builder::default()
            .region(&Region::new("us-east-1"))
            .env(env)
            .build();
        let err = provider
            .credentials()
            .await
            .expect_err("should fail, provider not loaded");
        assert!(
            format!("{}", err).contains("AWS_IAM_ROLE_ARN"),
            "`{}` did not contain expected string",
            err
        );
        match err {
            CredentialsError::InvalidConfiguration(_) => { /* ok */ }
            _ => panic!("incorrect error variant"),
        }
    }

    #[tokio::test]
    async fn fs_missing_file() {
        let env = Env::from_slice(&[
            (ENV_VAR_TOKEN_FILE, "/token.jwt"),
            (ENV_VAR_ROLE_ARN, "arn:aws:iam::123456789123:role/test-role"),
            (ENV_VAR_SESSION_NAME, "test-session"),
        ]);
        let fs = Fs::from_map(HashMap::new());
        let provider = Builder::default()
            .region(&Region::new("us-east-1"))
            .fs(fs)
            .env(env)
            .build();
        let err = provider.credentials().await.expect_err("no JWT token");
        match err {
            CredentialsError::ProviderError(_) => { /* ok */ }
            _ => panic!("incorrect error variant"),
        }
    }

    #[tokio::test]
    async fn invalid_token() -> Result<(), Box<dyn Error>> {
        let env = Env::from_slice(&[
            (ENV_VAR_TOKEN_FILE, "/token.jwt"),
            (ENV_VAR_ROLE_ARN, "arn:aws:iam::123456789123:role/test-role"),
            (ENV_VAR_SESSION_NAME, "test-session"),
        ]);
        let fs = Fs::from_test_dir("test-data/web-identity-token", "/");
        let traffic: NetworkTraffic = serde_json::from_str(&std::fs::read_to_string(
            "test-data/web-identity-token-invalid-token/http-traffic.json",
        )?)?;
        let connector = dvr::ReplayingConnection::new(traffic.events().clone());
        let provider = Builder::default()
            .region(&Region::new("us-east-1"))
            .fs(fs)
            .env(env)
            .connector(DynConnector::new(connector.clone()))
            .build();
        let err = provider.credentials().await.expect_err("no JWT token");
        assert!(format!("{}", err).contains("Error normalizing issuer"));
        match err {
            CredentialsError::ProviderError(_) => { /* ok */ }
            _ => panic!("incorrect error variant"),
        }
        Ok(())
    }
}
