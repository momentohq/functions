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
    /// The request did not complete within the allotted time. Often transient.
    #[error("request timed out: {0}")]
    Timeout(String),
    /// Failed to establish a connection to the server (DNS resolution or TCP
    /// connect failed). Often transient.
    #[error("connection failed: {0}")]
    ConnectionFailed(String),
    /// The connection was established but dropped mid-request (e.g. connection
    /// reset by peer, broken pipe, unexpected EOF). Often transient.
    #[error("connection interrupted: {0}")]
    ConnectionInterrupted(String),
    /// The TLS handshake or certificate validation failed. Not retryable.
    #[error("tls error: {0}")]
    TlsError(String),
    /// The redirect limit was exceeded.
    #[error("too many redirects: {0}")]
    TooManyRedirects(String),
    /// A response was received but its body could not be decoded. Not retryable.
    #[error("malformed response: {0}")]
    MalformedResponse(String),
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
            http::Error::Timeout(s) => HttpError::Timeout(s),
            http::Error::ConnectionFailed(s) => HttpError::ConnectionFailed(s),
            http::Error::ConnectionInterrupted(s) => HttpError::ConnectionInterrupted(s),
            http::Error::TlsError(s) => HttpError::TlsError(s),
            http::Error::TooManyRedirects(s) => HttpError::TooManyRedirects(s),
            http::Error::MalformedResponse(s) => HttpError::MalformedResponse(s),
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
///     Err(e) => eprintln!("request failed: {e}"),
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
///     Err(e) => eprintln!("request failed: {e}"),
/// }
/// ```
pub fn invoke(request: Request) -> Result<Response, HttpError> {
    match request.request_timeout() {
        // Saturate rather than wrap: a duration beyond u64::MAX ms is still an
        // effectively-unbounded timeout, so clamping to the max is the safe read.
        Some(timeout) => {
            let timeout_milliseconds = u64::try_from(timeout.as_millis()).unwrap_or(u64::MAX);
            http::invoke_with_timeout(request.into(), timeout_milliseconds)
        }
        None => http::invoke(request.into()),
    }
    .map(Into::into)
    .map_err(Into::into)
}
