//! Host interfaces for working with AWS credentials

use momento_functions_wit::host::momento::host::aws_auth;
use momento_functions_wit::host::momento::host::aws_auth::AuthError;

/// Reads AWS credentials from the environment variables
/// `AWS_ACCESS_KEY_ID` and `AWS_SECRET_ACCESS_KEY` at build time.
///
/// This is not the best way to encode AWS access, but it is included as a simple
/// way to get started with calling AWS.
///
/// The credentials must be set in the environment at build time, something like
/// ```bash
/// AWS_ACCESS_KEY_ID=$DEVELOPER_KEY_ID \
/// AWS_SECRET_ACCESS_KEY=$DEVELOPER_SECRET_KEY \
/// cargo build --target wasm32-wasip2 --release
/// ```
///
/// If your build environment does not have the variables set, the key and secret
/// will be `UNSET`. It is highly unlikely that this will work for calling AWS.
///
/// **Examples:**
/// ________
/// Read from the normal environment variables
/// ```rust
/// # use momento_functions_host::build_environment_aws_credentials;
/// let credentials = build_environment_aws_credentials!();
/// ```
/// ________
/// Read from foo_AWS_ACCESS_KEY_ID and foo_AWS_SECRET_ACCESS_KEY
/// ```rust
/// # use momento_functions_host::build_environment_aws_credentials;
/// let credentials = build_environment_aws_credentials!("foo_");
/// ```
#[macro_export]
macro_rules! build_environment_aws_credentials {
    ($prefix:literal) => {
        momento_functions_host::aws::auth::Credentials::Hardcoded {
            access_key_id: option_env!(concat!($prefix, "AWS_ACCESS_KEY_ID"))
                .unwrap_or("UNSET")
                .to_string(),
            secret_access_key: option_env!(concat!($prefix, "AWS_SECRET_ACCESS_KEY"))
                .unwrap_or("UNSET")
                .to_string(),
        }
    };
    () => {
        build_environment_aws_credentials!("")
    };
}

/// The authorization strategy to use when connecting to AWS services.
pub enum Credentials {
    /// Credentials that are hardcoded in the application.
    /// You should use a different strategy if you can.
    ///
    /// Compiled wasm archives are irretrievable from Momento. The only way to leak
    /// hardcoded credentials after uploading to Momento is for you to write code to
    /// exfiltrate them.
    Hardcoded {
        /// The AWS access key ID for the IAM user you wish to use
        access_key_id: String,
        /// The AWS secret access key for the IAM user you wish to use
        secret_access_key: String,
    },
}

/// A configured AWS credentials provider. This can be used to connect to AWS services.
pub struct AwsCredentialsProvider {
    resource: aws_auth::CredentialsProvider,
}

impl AwsCredentialsProvider {
    /// The credentials to use when connecting to AWS services.
    ///
    /// **Examples:**
    /// ```rust,no_run
    /// # // Not run because docs.rs does not run a Momento WIT host environment, of course!
    /// # // But it does at least compile, to make sure the example is correct.
    /// # use momento_functions_host::{build_environment_aws_credentials, aws::auth::{AwsCredentialsProvider}};
    /// let provider = AwsCredentialsProvider::new("us-east-1", build_environment_aws_credentials!())?;
    /// ```
    pub fn new(
        region: impl AsRef<str>,
        credentials: Credentials,
    ) -> Result<AwsCredentialsProvider, AuthError> {
        let wit_authorization = match credentials {
            Credentials::Hardcoded {
                access_key_id,
                secret_access_key,
            } => aws_auth::Authorization::Hardcoded(aws_auth::Credentials {
                access_key_id,
                secret_access_key,
            }),
        };

        let resource = aws_auth::provider(&wit_authorization, region.as_ref())?;

        momento_functions_wit::host::momento::host::aws_ddb::Client::new(&resource);

        Ok(AwsCredentialsProvider { resource })
    }

    /// Returns the underlying WIT resource.
    pub(crate) fn resource(&self) -> &aws_auth::CredentialsProvider {
        &self.resource
    }
}
