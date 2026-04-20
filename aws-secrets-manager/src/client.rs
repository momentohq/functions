use std::time::Duration;

use crate::wit::momento::aws_secrets::aws_secrets::{self as aws_secrets};
use momento_functions_aws_auth::CredentialsProvider;
use momento_functions_bytes::encoding::{Extract, ExtractError};

/// Secrets Manager client for host interfaces.
///
/// This client uses Momento's host-provided AWS communication channel, which
/// is kept hot at all times. When your Function has not run in several days or more,
/// the channel is still hot and ready, keeping your Function invocations predictable
/// even when your demand is unpredictable.
pub struct SecretsManagerClient {
    client: aws_secrets::Client,
}

/// Builder for a Secrets Manager `GetSecretValue` request.
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
    /// * `secret_id` - The name or ARN of the secret to retrieve.
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

/// An error occurred while retrieving a secret from Secrets Manager.
#[derive(Debug, thiserror::Error)]
pub enum SecretsManagerGetSecretValueError<E>
where
    E: ExtractError,
{
    /// The value could not be extracted with the provided implementation.
    #[error("Failed to extract value.")]
    ExtractFailed {
        /// The underlying extract error.
        cause: E,
    },
    /// An error occurred when calling the host Secrets Manager interface.
    #[error(transparent)]
    SecretsManagerError(#[from] aws_secrets::SecretsError),
}

impl SecretsManagerClient {
    /// Create a new Secrets Manager client.
    pub fn new(credentials: &CredentialsProvider) -> Self {
        Self {
            client: aws_secrets::Client::new(credentials),
        }
    }

    /// Get a secret value from AWS Secrets Manager.
    ///
    /// If you would like to avoid repeated calls to AWS Secrets Manager to save on latency,
    /// pass a non-zero `allowed_staleness`. The secret is securely cached within the
    /// Function's context and is only accessible by the Function itself. Once the cached
    /// secret is older than `allowed_staleness`, the next call will go back to Secrets Manager.
    ///
    /// Pass `Duration::from_secs(0)` to always make the call to AWS.
    ///
    /// # Arguments
    /// * `request` - The request to send to Secrets Manager.
    /// * `allowed_staleness` - How long the cached secret may be reused before refetching.
    pub fn get_secret_value<T: Extract>(
        &self,
        request: GetSecretValueRequest,
        allowed_staleness: Duration,
    ) -> Result<T, SecretsManagerGetSecretValueError<T::Error>> {
        self.do_get_secret_value(request, allowed_staleness)
    }

    /// Like [`get_secret_value`](Self::get_secret_value), but always makes a request to
    /// Secrets Manager for the latest value.
    pub fn get_latest_secret_value<T: Extract>(
        &self,
        request: GetSecretValueRequest,
    ) -> Result<T, SecretsManagerGetSecretValueError<T::Error>> {
        self.do_get_secret_value(request, Duration::from_secs(0))
    }

    fn do_get_secret_value<T: Extract>(
        &self,
        request: GetSecretValueRequest,
        allowed_staleness: Duration,
    ) -> Result<T, SecretsManagerGetSecretValueError<T::Error>> {
        let response = self
            .client
            .get_secret_value(&aws_secrets::GetSecretValueRequest {
                secret_id: request.secret_id,
                version_id: request.version_id,
                version_stage: request.version_stage,
                allowed_staleness_seconds: allowed_staleness.as_secs(),
            })?;

        // Normalize the secret to bytes regardless of which variant was returned.
        let secret_bytes = match response.secret {
            aws_secrets::SecretValue::SecretBytes(bytes) => bytes,
            aws_secrets::SecretValue::SecretString(s) => s.into_bytes(),
        };

        T::extract(secret_bytes.into())
            .map_err(|cause| SecretsManagerGetSecretValueError::ExtractFailed { cause })
    }
}
