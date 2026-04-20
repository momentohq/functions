use momento_functions_bytes::Data;
use thiserror::Error;

use crate::{request::Request, wit::momento::http::http};

/// An error returned by an HTTP request.
#[derive(Debug, Error)]
pub enum HttpError {
    /// An internal error occurred within Momento.
    #[error("internal error")]
    InternalError,
    /// An error occurred while making the request.
    #[error("request error: {0}")]
    RequestError(String),
    /// The provided URL was not valid.
    #[error("invalid url '{url}': {error}")]
    InvalidUrl { url: String, error: String },
    /// A provided header name was not valid.
    #[error("invalid header name '{header}': {error}")]
    InvalidHeaderName { header: String, error: String },
    /// A provided header value was not valid.
    #[error("invalid header value '{value}': {error}")]
    InvalidHeaderValue { value: String, error: String },
}

impl From<http::Error> for HttpError {
    fn from(e: http::Error) -> Self {
        match e {
            http::Error::InternalError => HttpError::InternalError,
            http::Error::RequestError(s) => HttpError::RequestError(s),
            http::Error::InvalidUrl(u) => HttpError::InvalidUrl {
                url: u.url,
                error: u.error,
            },
            http::Error::InvalidHeaderName(h) => HttpError::InvalidHeaderName {
                header: h.header,
                error: h.error,
            },
            http::Error::InvalidHeaderValue(v) => HttpError::InvalidHeaderValue {
                value: v.value,
                error: v.error,
            },
        }
    }
}

/// A response from an HTTP request.
///
/// The `body` is exposed as [`Data`] so bytes are only read when you need them.
/// Call [`Data::into_bytes`] to fully materialize the body, or read it in
/// chunks via the underlying buffer resource.
pub struct Response {
    /// The HTTP status code.
    pub status: u16,
    /// Response headers as name-value pairs.
    pub headers: Vec<(String, String)>,
    /// The response body. Read on demand to avoid unnecessary allocation.
    pub body: Data,
}

impl From<http::Response> for Response {
    fn from(r: http::Response) -> Self {
        Response {
            status: r.status,
            headers: r.headers,
            body: Data::from(r.body),
        }
    }
}

/// Send an HTTP request.
///
/// # Arguments
/// * `request` - The request to send.
///
/// # Examples
/// ________
/// Send a GET request:
/// ```rust,no_run
/// use momento_functions_http::{invoke, Request};
///
/// match invoke(Request::new("https://example.com/api", "GET")) {
///     Ok(response) => println!("status: {}", response.status),
///     Err(e) => log::error!("request failed: {e}"),
/// }
/// ```
///
/// Send a POST request with a JSON body:
/// ```rust,no_run
/// use momento_functions_http::{invoke, Request};
///
/// match invoke(
///     Request::new("https://example.com/api", "POST")
///         .with_header("Content-Type", "application/json")
///         .with_body(b"{\"key\": \"value\"}".to_vec()),
/// ) {
///     Ok(response) => println!("status: {}", response.status),
///     Err(e) => log::error!("request failed: {e}"),
/// }
/// ```
pub fn invoke(request: Request) -> Result<Response, HttpError> {
    http::invoke(request.into())
        .map(Into::into)
        .map_err(Into::into)
}
