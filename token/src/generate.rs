use crate::permissions::Permissions;
use crate::wit::momento::token::token;

/// An error occurred when generating a disposable token.
#[derive(thiserror::Error, Debug)]
pub enum GenerateDisposableTokenError {
    /// An error occurred when calling the host generate function.
    #[error(transparent)]
    TokenError(#[from] token::TokenError),
}

/// Response from generating a disposable token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerateDisposableTokenResponse {
    /// The new, disposable token with scoped permissions.
    pub api_key: String,
    /// The endpoint to be used when calling Momento with this token.
    pub endpoint: String,
    /// How long the token is valid until, in epoch seconds.
    pub valid_until: u64,
}

impl From<token::GenerateDisposableTokenResponse> for GenerateDisposableTokenResponse {
    fn from(response: token::GenerateDisposableTokenResponse) -> Self {
        GenerateDisposableTokenResponse {
            api_key: response.api_key,
            endpoint: response.endpoint,
            valid_until: response.valid_until,
        }
    }
}

/// Generate a disposable token with scoped permissions.
///
/// # Arguments
/// * `valid_for_seconds` - How many seconds the token should be valid for.
/// * `permissions` - The permissions to scope the token with.
/// * `token_id` - Optional string to be included inside the token.
///
/// # Examples
/// ________
/// Generate a token with read-write cache access:
/// ```rust,no_run
/// use momento_functions_token::{
///     generate_disposable_token, Permissions, CachePermissions,
/// };
///
/// match generate_disposable_token(
///     300,
///     Permissions::new().with_cache(CachePermissions::read_write()),
///     None,
/// ) {
///     Ok(response) => println!("Token: {}, Endpoint: {}", response.api_key, response.endpoint),
///     Err(e) => log::error!("failed to generate token: {e}"),
/// }
/// ```
pub fn generate_disposable_token(
    valid_for_seconds: u32,
    permissions: Permissions,
    token_id: Option<String>,
) -> Result<GenerateDisposableTokenResponse, GenerateDisposableTokenError> {
    let stringified_permissions = serde_json::to_string(&permissions).map_err(|e| {
        GenerateDisposableTokenError::TokenError(token::TokenError::InvalidArgument(format!(
            "Invalid permissions object passed in: {e:?}"
        )))
    })?;
    token::generate_disposable_token(
        token::Expires { valid_for_seconds },
        &stringified_permissions,
        token_id.as_deref(),
    )
    .map_err(Into::into)
    .map(Into::into)
}
