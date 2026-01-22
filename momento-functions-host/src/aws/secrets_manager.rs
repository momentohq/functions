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
///
/// Additionally, you can leverage Momento's caching system to securely store your secret without
/// making repeated calls to AWS Secrets Manager.
pub struct SecretsManagerClient {
    client: host::aws_secrets::Client,
    cache_ttl: Option<Duration>,
}

/// Helpful struct to easily make a request to AWS Secrets Manager.
///
/// Use `GetSecretValueRequest::new()` to construct what you need.
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
    /// Create a new Secrets Manager client without caching.
    ///
    /// Secrets retrieved through this client will always be fetched directly from AWS Secrets Manager
    /// without being cached.
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
            cache_ttl: None,
        }
    }

    /// Create a new Secrets Manager client with caching enabled.
    ///
    /// Secrets retrieved through this client will be cached in Momento with the specified TTL.
    /// Subsequent requests for the same secret will return the cached value until the TTL expires.
    ///
    /// # Arguments
    /// * `credentials` - AWS credentials provider for authenticating with Secrets Manager
    /// * `cache_ttl` - Duration to cache secrets
    ///
    /// # Examples
    ///
    /// ```rust,no-run
    /// use std::time::Duration;
    /// use momento_functions_host::aws::auth::AwsCredentialsProvider;
    /// use momento_functions_host::aws::secrets_manager::SecretsManagerClient;
    /// use momento_functions_host::build_environment_aws_credentials;
    /// use momento_functions_wit::host::momento::host::aws_auth::AuthError;
    ///
    /// # fn f() -> Result<(), AuthError> {
    /// // Cache secrets for 5 minutes
    /// let client = SecretsManagerClient::new_with_cache(
    ///     &AwsCredentialsProvider::new(
    ///         "us-east-1",
    ///         build_environment_aws_credentials!()
    ///     )?,
    ///     Duration::from_secs(300)
    /// );
    /// # Ok(())
    /// # }
    /// ```
    pub fn new_with_cache(credentials: &auth::AwsCredentialsProvider, cache_ttl: Duration) -> Self {
        Self {
            client: host::aws_secrets::Client::new(credentials.resource()),
            cache_ttl: Some(cache_ttl),
        }
    }

    /// Get a secret value from AWS Secrets Manager.
    ///
    /// If you construct this client only with `new()`, the secret is fetched directly from AWS Secrets Manager
    /// without storing the secret within your cache.
    ///
    /// If you would like to avoid repeated calls to AWS Secrets Manager (and hit limits), you can instead construct
    /// this with `new_with_cache` and a TTL to store your secret within your cache. This way, repeated functions can
    /// read from the cache instead.
    ///
    /// # Arguments
    /// * `request` - The request to send to Secrets Manager
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
    /// let secret: Vec<u8> = client.get_secret_value(GetSecretValueRequest::new("my-secret"))?;
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
    ///         .version_stage("AWSPENDING")
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
    /// let client = SecretsManagerClient::new_with_cache(&credentials, Duration::from_secs(300));
    /// let secret: Vec<u8> = client.get_secret_value(GetSecretValueRequest::new("my-secret"))?;
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
    /// let client = SecretsManagerClient::new_with_cache(&credentials, Duration::from_secs(300));
    /// let secret: MyPersistedSecret = client.get_secret_value(GetSecretValueRequest::new("my-secret"))?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_secret_value<T: crate::encoding::Extract>(
        &self,
        request: GetSecretValueRequest,
    ) -> Result<T, SecretsManagerGetSecretValueError<T::Error>> {
        let cache_key = request.secret_id.clone();

        // If caching is enabled, check the cache first
        if let Some(ttl) = self.cache_ttl {
            log::debug!("caching enabled for secrets, checking if secret exists in cache");
            if let Some(cached_value) = crate::cache::get::<T>(&cache_key)
                .map_err(|e| {
                    Some(match e {
                        crate::cache::CacheGetError::ExtractFailed { cause } => {
                            SecretsManagerGetSecretValueError::ExtractFailed { cause }
                        }
                        crate::cache::CacheGetError::CacheError(_) => {
                            return None;
                        }
                    })
                })
                .unwrap_or(None)
            {
                return Ok(cached_value);
            }

            log::debug!("cache miss, retrieving secret from AWS Secrets Manager");

            let response =
                self.client
                    .get_secret_value(&host::aws_secrets::GetSecretValueRequest {
                        secret_id: request.secret_id,
                        version_id: request.version_id,
                        version_stage: request.version_stage,
                    })?;

            // Extract the secret bytes based on the variant so it is properly encoded upon cache storage
            let secret_bytes = match response.secret {
                host::aws_secrets::SecretValue::SecretBytes(bytes) => bytes,
                host::aws_secrets::SecretValue::SecretString(s) => s.into_bytes(),
            };

            log::debug!("storing secret in cache");
            if let Err(e) = crate::cache::set(&cache_key, secret_bytes.clone(), ttl) {
                log::debug!(
                    "failed to cache secret, will return secret since it was retrieved: {e:?}"
                );
            }

            // Extract and return the value
            T::extract(secret_bytes)
                .map_err(|cause| SecretsManagerGetSecretValueError::ExtractFailed { cause })
        } else {
            log::debug!("retrieving secret from AWS Secrets Manager");
            let response =
                self.client
                    .get_secret_value(&host::aws_secrets::GetSecretValueRequest {
                        secret_id: request.secret_id,
                        version_id: request.version_id,
                        version_stage: request.version_stage,
                    })?;

            let secret_bytes = match response.secret {
                host::aws_secrets::SecretValue::SecretBytes(bytes) => bytes,
                host::aws_secrets::SecretValue::SecretString(s) => s.into_bytes(),
            };

            T::extract(secret_bytes)
                .map_err(|cause| SecretsManagerGetSecretValueError::ExtractFailed { cause })
        }
    }
}
