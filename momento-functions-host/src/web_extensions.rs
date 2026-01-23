//! Host interface extensions for Web Functions
//!
//! These interfaces don't do anything on other kinds of Functions.

use std::{collections::HashMap, env, sync::LazyLock};

use momento_functions_wit::function_web::momento::functions::web_function_support;

static NOT_FOUND: &str = "<not found>";
// Some of the Momento host interfaces will take ownership of the returned value, returning
// a `None` or empty-like object upon repeated calls. These `OnceLocks` allow for repeated
// calls since the data is not intended to be mutated anyway.
static GET_ENVIRONMENT_ONCE: LazyLock<FunctionEnvironment> = LazyLock::new(|| {
    let cache_name = env::var("__CACHE_NAME").unwrap_or(NOT_FOUND.to_string());
    let function_name = env::var("__FUNCTION_NAME").unwrap_or(NOT_FOUND.to_string());
    let invocation_id = env::var("__INVOCATION_ID").unwrap_or(NOT_FOUND.to_string());
    FunctionEnvironment {
        cache_name,
        function_name,
        invocation_id,
    }
});
static GET_HEADERS_ONCE: LazyLock<HashMap<String, String>> = LazyLock::new(|| {
    web_function_support::headers()
        .into_iter()
        .map(|web_function_support::Header { name, value }| (name, value))
        .collect()
});
// Yes, this is a hashmap, but query parameters can be repeated. Usually people don't do that though.
static GET_QUERY_PARAMETERS_ONCE: LazyLock<HashMap<String, String>> = LazyLock::new(|| {
    web_function_support::query_parameters()
        .into_iter()
        .map(|web_function_support::QueryParameter { name, value }| (name, value))
        .collect()
});
static GET_TOKEN_METADATA_ONCE: LazyLock<Option<String>> =
    LazyLock::new(web_function_support::token_metadata);
static GET_HTTP_METHOD_ONCE: LazyLock<String> = LazyLock::new(web_function_support::http_method);
static GET_HTTP_PATH_ONCE: LazyLock<String> =
    LazyLock::new(|| web_function_support::invocation_path().unwrap_or_default());

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
    function_name: String,
    invocation_id: String,
}

impl FunctionEnvironment {
    /// Returns a singleton object containing useful information regarding the current function invocation's
    /// environment. This is safe to call multiple times. It is recommended to use this object when trying to
    /// access the environment variables populated upon function creation.
    pub fn get_function_environment() -> &'static FunctionEnvironment {
        &GET_ENVIRONMENT_ONCE
    }

    /// The name of the cache this function belongs to. You can also access this via:
    /// ```rust
    /// let cache_name = std:env::var("__CACHE_NAME").unwrap_or_default());
    /// ```
    pub fn cache_name(&self) -> &String {
        &self.cache_name
    }

    /// The name of the function. You can also access this via:
    /// ```rust
    /// let function_name = std:env::var("__FUNCTION_NAME").unwrap_or_default());
    /// ```
    pub fn function_name(&self) -> &String {
        &self.function_name
    }

    /// The ID of the currently executing invocation. You can also access this via:
    /// ```rust
    /// let invocation_id = std:env::var("__INVOCATION_ID").unwrap_or_default());
    /// ```
    pub fn invocation_id(&self) -> &String {
        &self.invocation_id
    }

    /// A map of the headers used in the request when the function was invoked. You can also access this via:
    /// ```rust,no_run
    /// use momento_functions_host::web_extensions::headers;
    /// let headers = headers();
    /// ```
    pub fn headers(&self) -> &HashMap<String, String> {
        headers()
    }

    /// A map of the query parameters used in the request when the function was invoked. You can also access this via:
    /// ```rust,no_run
    /// use momento_functions_host::web_extensions::query_parameters;
    /// let query_parameters = query_parameters();
    /// ```
    pub fn query_parameters(&self) -> &HashMap<String, String> {
        query_parameters()
    }

    /// The metadata within the caller's token, if present. You can also access this via:
    /// ```rust,no_run
    /// use momento_functions_host::web_extensions::token_metadata;
    /// let token_metadata = token_metadata();
    /// ```
    pub fn token_metadata(&self) -> &Option<String> {
        token_metadata()
    }

    /// The HTTP method used in the request when the function was invoked.
    /// "GET", "POST", etc.
    pub fn http_method(&self) -> &str {
        &GET_HTTP_METHOD_ONCE
    }

    /// The HTTP path used in the request when the function was invoked.
    ///
    /// This is the path relative to the function. If your function is deployed at
    /// `https://gomomento.com/my-function`, and you call
    /// `https://gomomento.com/my-function/search/me`, this will return `/search/me`.
    pub fn http_path(&self) -> &str {
        &GET_HTTP_PATH_ONCE
    }
}

/// Returns the headers for the web function, if any are present.
pub fn headers() -> &'static HashMap<String, String> {
    &GET_HEADERS_ONCE
}

/// Returns the query parameters for the web function, if any are present.
pub fn query_parameters() -> &'static HashMap<String, String> {
    &GET_QUERY_PARAMETERS_ONCE
}

/// Returns the metadata within the caller's token, if present.
pub fn token_metadata() -> &'static Option<String> {
    &GET_TOKEN_METADATA_ONCE
}

/// Returns the invocation ID of the currently invoked function. This may be helpful to you
/// if you want to connect a request ID to callers with the invocation that was used at that time.
#[deprecated(since = "0.7.0", note = "Use `FunctionEnvironment` instead")]
pub fn invocation_id() -> String {
    web_function_support::invocation_id()
}
