//! Host interface extensions for Web Functions
//!
//! These interfaces don't do anything on other kinds of Functions.

use std::{collections::HashMap, env, sync::OnceLock};

use momento_functions_wit::function_web::momento::functions::web_function_support;

static NOT_FOUND: &str = "<not found>";
// Some of the Momento host interfaces will take ownership of the returned value, returning
// a `None` or empty-like object upon repeated calls. These `OnceLocks` allow for repeated
// calls since the data is not intended to be mutated anyway.
static GET_ENVIRONMENT_ONCE: OnceLock<FunctionEnvironment> = OnceLock::new();
static GET_HEADERS_ONCE: OnceLock<Vec<(String, String)>> = OnceLock::new();
static GET_QUERY_PARAMETERS_ONCE: OnceLock<Vec<(String, String)>> = OnceLock::new();
static GET_TOKEN_METADATA_ONCE: OnceLock<Option<String>> = OnceLock::new();

/// Data structure containing easy-to-access information regarding the current invocation's
/// environment. Momento will populate this information as necessary, either through provided
/// environment variables normally accessible via `std::env::var()`, or interfaces across the WASI
/// bridge. For best practices/avoiding variable typos, a constructor has been provided:
/// ```rust,no_run
/// use momento_functions_host::web_extensions::FunctionEnvironment;
/// let function_environment = FunctionEnvironment::get_function_environment();
///
/// // Examples
/// log::info!("Cache: {}", function_environment::cache());
/// log::info!("Invocation ID: {}", function_environment::invocation_id());
///
/// let joined_query_parameters = function_environment::query_parameters()
///     .iter()
///     .map(|(k, v)| format!("{k}={v}"))
///     .collect::<Vec<_>>()
///     .join(", ");
/// log::info!("Called with query parameters: {joined_query_parameters}");
/// ```
pub struct FunctionEnvironment {
    cache_name: String,
    invocation_id: String,
    headers: HashMap<String, String>,
    query_parameters: HashMap<String, String>,
    token_metadata: Option<String>,
}

impl FunctionEnvironment {
    /// Returns a singleton object containing useful information regarding the current function invocation's
    /// environment. This is safe to call multiple times. It is recommended to use this object when trying to
    /// access the provided
    pub fn get_function_environment() -> &'static FunctionEnvironment {
        GET_ENVIRONMENT_ONCE.get_or_init(|| {
            let cache_name = env::var("__CACHE_NAME").unwrap_or(NOT_FOUND.to_string());
            let invocation_id = env::var("__INVOCATION_ID").unwrap_or(NOT_FOUND.to_string());
            FunctionEnvironment {
                cache_name,
                invocation_id,
                headers: HashMap::from_iter(headers()),
                query_parameters: HashMap::from_iter(query_parameters()),
                token_metadata: token_metadata(),
            }
        })
    }

    /// The name of the cache this function belongs to. You can also access this by calling:
    /// ```rust
    /// let cache_name = std:env::var("__CACHE_NAME").unwrap_or_default());
    /// ```
    pub fn cache_name(&self) -> &String {
        &self.cache_name
    }

    /// The ID of the currently executing invocation. You can also access this by calling:
    /// ```rust
    /// let invocation_id = std:env::var("__INVOCATION_ID").unwrap_or_default());
    /// ```
    pub fn invocation_id(&self) -> &String {
        &self.invocation_id
    }

    /// A map of the headers used in the request when the function was invoked. If you would prefer
    /// the raw `Vec<(String, String)>` object, you can access it via:
    /// ```rust,no_run
    /// use momento_functions_host::web_extensions::headers;
    /// let headers = headers();
    /// ```
    pub fn headers(&self) -> &HashMap<String, String> {
        &self.headers
    }

    /// A map of the query parameters used in the request when the function was invoked. If you would prefer
    /// the raw `Vec<(String, String)>` object, you can access it via:
    /// ```rust,no_run
    /// use momento_functions_host::web_extensions::query_parameters;
    /// let query_parameters = query_parameters();
    /// ```
    pub fn query_parameters(&self) -> &HashMap<String, String> {
        &self.query_parameters
    }

    /// The metadata within the caller's token, if present. If you would prefer
    /// the raw `Option<String>`` object, you can access it via:
    /// ```rust,no_run
    /// use momento_functions_host::web_extensions::token_metadata;
    /// let token_metadata = token_metadata();
    /// ```
    pub fn token_metadata(&self) -> &Option<String> {
        &self.token_metadata
    }
}

/// Returns the headers for the web function, if any are present.
pub fn headers() -> Vec<(String, String)> {
    GET_HEADERS_ONCE
        .get_or_init(|| {
            web_function_support::headers()
                .into_iter()
                .map(|web_function_support::Header { name, value }| (name, value))
                .collect()
        })
        .to_owned()
}

/// Returns the query parameters for the web function, if any are present.
pub fn query_parameters() -> Vec<(String, String)> {
    GET_QUERY_PARAMETERS_ONCE
        .get_or_init(|| {
            web_function_support::query_parameters()
                .into_iter()
                .map(|web_function_support::QueryParameter { name, value }| (name, value))
                .collect()
        })
        .to_owned()
}

/// Returns the metadata within the caller's token, if present.
pub fn token_metadata() -> Option<String> {
    GET_TOKEN_METADATA_ONCE
        .get_or_init(web_function_support::token_metadata)
        .to_owned()
}

/// Returns the invocation ID of the currently invoked function. This may be helpful to you
/// if you want to connect a request ID to callers with the invocation that was used at that time.
#[deprecated(since = "0.7.0", note = "Use `FunctionEnvironment` instead")]
pub fn invocation_id() -> String {
    web_function_support::invocation_id()
}
