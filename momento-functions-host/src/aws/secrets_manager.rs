//! Host interfaces for working with AWS Secrets Manager
use std::time::Duration;

use crate::aws::secrets_manager::host::aws_secrets::SecretsError;
use momento_functions_wit::host::momento::host;

use crate::encoding::ExtractError;

use super::auth;

/// Secrets Manager client for host interfaces.
///
/// This client uses Momento's host-provided AWS communication channel, which
/// is kept hot at all times. When your Function has not run in several days or more,
/// the channel is still hot and ready, keeping your Function invocations predictable
/// even when your demand is unpredictable.
pub struct SecretsManagerClient {
    client: host::aws_secrets::Client,
}

/// Helpful struct to easily make a request to AWS Secrets Manager.
///
/// Use `GetSecretValueRequest::new("your secret ID")` to construct what you need.
pub struct GetSecretValueRequest {
    secret_id: String,
    version_id: Option<String>,
    version_stage: Option<String>,
}

impl GetSecretValueRequest {
    /// Create a new request builder for the specified secret.
    ///
    /// # Arguments
    /// * `secret_id` - The name or ARN of the secret to retrieve
    pub fn new(secret_id: impl Into<String>) -> Self {
        Self {
            secret_id: secret_id.into(),
            version_id: None,
            version_stage: None,
        }
    }

    /// Set the specific version of the secret to retrieve.
    pub fn version_id(mut self, version_id: impl Into<String>) -> Self {
        self.version_id = Some(version_id.into());
        self
    }

    /// Set the version stage of the secret to retrieve (e.g., "AWSCURRENT", "AWSPENDING").
    pub fn version_stage(mut self, version_stage: impl Into<String>) -> Self {
        self.version_stage = Some(version_stage.into());
        self
    }
}

/// An error occurred while retrieving a secret from Secrets Manager
#[derive(Debug, thiserror::Error)]
pub enum SecretsManagerGetSecretValueError<E>
where
    E: ExtractError,
{
    /// The value could not be extracted with the provided implementation.
    #[error("Failed to extract value.")]
    ExtractFailed {
        /// The underlying encode error.
        cause: E,
    },
    /// An error occurred when calling the host secrets manager interface.
    #[error(transparent)]
    SecretsManagerError(#[from] SecretsError),
}

impl SecretsManagerClient {
    /// Create a new Secrets Manager client.
    ///
    ///
    /// ```rust,no-run
    /// use momento_functions_host::aws::auth::AwsCredentialsProvider;
    /// use momento_functions_host::aws::secrets_manager::SecretsManagerClient;
    /// use momento_functions_host::build_environment_aws_credentials;
    /// use momento_functions_wit::host::momento::host::aws_auth::AuthError;
    ///
    /// # fn f() -> Result<(), AuthError> {
    /// let client = SecretsManagerClient::new(
    ///     &AwsCredentialsProvider::new(
    ///         "us-east-1",
    ///         build_environment_aws_credentials!()
    ///     )?
    /// );
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(credentials: &auth::AwsCredentialsProvider) -> Self {
        Self {
            client: host::aws_secrets::Client::new(credentials.resource()),
        }
    }

    /// Get a secret value from AWS Secrets Manager.
    ///
    /// If you would like to avoid repeated calls to AWS Secrets Manager to save on latency, you can
    /// pass in a specified `Duration` for allowed staleness in your request. The secret is securely cached
    /// within the function's context and is only accessible by the function itself. No other resource can access
    /// this secret. Once the secret has been detected as stale, the request will be sent directly to AWS Secrets
    /// Manager once more.
    ///
    /// Staleness refers to how long the secret has been cached within the function's context before another call
    /// to Secrets Manager is made. This is compareda against the first time the secret is cached, regardless of
    /// invocation. You can use this to ensure your solution has a window of allowing stale credentials when a
    /// secret has been rotated.
    ///
    /// You can set it to `Duration::from_secs(0)` to always make the call to AWS.
    ///
    /// # Arguments
    /// * `request` - The request to send to Secrets Manager
    /// * `allowed_staleness` - How long to cache the secret within your function's context.
    ///
    /// # Examples
    ///
    /// Simple fetch:
    /// ```rust,no_run
    /// use momento_functions_host::aws::auth::AwsCredentialsProvider;
    /// use momento_functions_host::aws::secrets_manager::{SecretsManagerClient, GetSecretValueRequest};
    /// use momento_functions_host::build_environment_aws_credentials;
    /// use momento_functions_wit::host::momento::host::aws_auth::AuthError;
    /// # fn f() -> Result<(), Box<dyn std::error::Error>> {
    /// let credentials = AwsCredentialsProvider::new("us-east-1", build_environment_aws_credentials!())?;
    /// let client = SecretsManagerClient::new(&credentials);
    /// let secret: Vec<u8> = client.get_secret_value(GetSecretValueRequest::new("my-secret"), Duration::from_secs(0))?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// Fetch with version:
    /// ```rust,no_run
    /// use momento_functions_host::aws::auth::AwsCredentialsProvider;
    /// use momento_functions_host::aws::secrets_manager::{SecretsManagerClient, GetSecretValueRequest};
    /// use momento_functions_host::build_environment_aws_credentials;
    /// use momento_functions_wit::host::momento::host::aws_auth::AuthError;
    /// # fn f() -> Result<(), Box<dyn std::error::Error>> {
    /// let credentials = AwsCredentialsProvider::new("us-east-1", build_environment_aws_credentials!())?;
    /// let client = SecretsManagerClient::new(&credentials);
    /// let secret: Vec<u8> = client.get_secret_value(
    ///     GetSecretValueRequest::new("my-secret")
    ///         .version_stage("AWSPENDING"),
    ///     Duration::from_secs(0),
    /// )?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// Fetch with caching:
    /// ```rust,no_run
    /// use std::time::Duration;
    /// use momento_functions_host::aws::auth::AwsCredentialsProvider;
    /// use momento_functions_host::aws::secrets_manager::{SecretsManagerClient, GetSecretValueRequest};
    /// use momento_functions_host::build_environment_aws_credentials;
    /// use momento_functions_wit::host::momento::host::aws_auth::AuthError;
    /// # fn f() -> Result<(), Box<dyn std::error::Error>> {
    /// let credentials = AwsCredentialsProvider::new("us-east-1", build_environment_aws_credentials!())?;
    /// let client = SecretsManagerClient::new(&credentials, Duration::from_secs(300));
    /// let allowed_staleness = Duration::from_mins(5);
    /// let secret: Vec<u8> = client.get_secret_value(GetSecretValueRequest::new("my-secret"), allowed_staleness)?;
    /// # Ok(())
    /// # }
    ///
    /// ```
    /// Fetch with a JSON struct:
    /// ```rust,no_run
    ///  use std::time::Duration;
    ///  use momento_functions_host::aws::auth::AwsCredentialsProvider;
    ///  use momento_functions_host::aws::secrets_manager::{SecretsManagerClient, GetSecretValueRequest};
    ///  use momento_functions_host::build_environment_aws_credentials;
    ///  use momento_functions_wit::host::momento::host::aws_auth::AuthError;
    /// # fn f() -> Result<(), Box<dyn std::error::Error>> {
    /// struct MyPersistedSecret {
    ///     pub key: String,
    ///     pub nonce: String,
    ///     pub signing_key: Vec<u8>,
    /// }
    ///
    /// let credentials = AwsCredentialsProvider::new("us-east-1", build_environment_aws_credentials!())?;
    /// let client = SecretsManagerClient::new(&credentials, Duration::from_secs(300));
    /// let allowed_staleness = Duration::from_mins(5);
    /// let secret: MyPersistedSecret = client.get_secret_value(GetSecretValueRequest::new("my-secret"), allowed_staleness)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_secret_value<T: crate::encoding::Extract>(
        &self,
        request: GetSecretValueRequest,
        allowed_staleness: Duration,
    ) -> Result<T, SecretsManagerGetSecretValueError<T::Error>> {
        let response = self
            .client
            .get_secret_value(&host::aws_secrets::GetSecretValueRequest {
                secret_id: request.secret_id,
                version_id: request.version_id,
                version_stage: request.version_stage,
                allowed_staleness_seconds: allowed_staleness.as_secs(),
            })?;

        // Extract the secret bytes based on the variant so it is properly encoded upon cache storage
        let secret_bytes = match response.secret {
            host::aws_secrets::SecretValue::SecretBytes(bytes) => bytes,
            host::aws_secrets::SecretValue::SecretString(s) => s.into_bytes(),
        };

        // Extract and return the value
        T::extract(secret_bytes)
            .map_err(|cause| SecretsManagerGetSecretValueError::ExtractFailed { cause })
    }
}
