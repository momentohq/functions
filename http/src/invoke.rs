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

/// Marker the Momento host appends to transient transport errors that are safe to retry. Kept in
/// sync with the host's HTTP error classification.
const RETRYABLE_MARKER: &str = "(retryable)";

impl HttpError {
    /// Returns `true` when this error represents a transient, transport-level failure that is
    /// safe to retry for an **idempotent** request.
    ///
    /// The most common cause is a keep-alive connection being recycled by the remote peer (or an
    /// intervening load balancer) in between requests: the connection is torn down mid-exchange,
    /// so the request typically never reaches the destination's application layer. These surface
    /// as a [`HttpError::RequestError`], which is otherwise indistinguishable from a terminal
    /// failure — this method lets you branch on it instead of inspecting the message yourself.
    ///
    /// Only retry when your request is idempotent — a `GET`, or a write guarded by an idempotency
    /// key. For a non-idempotent request (e.g. a plain `POST` that is not safe to repeat), prefer
    /// surfacing the error to your caller.
    ///
    /// # Examples
    /// ```rust,no_run
    /// use momento_functions_http::{invoke, HttpError, Request};
    ///
    /// fn get_with_retry(url: &str) -> Result<(), HttpError> {
    ///     let mut attempts = 0;
    ///     loop {
    ///         attempts += 1;
    ///         match invoke(Request::new(url, "GET")) {
    ///             Ok(_response) => return Ok(()),
    ///             Err(e) if e.is_retryable() && attempts < 3 => continue,
    ///             Err(e) => return Err(e),
    ///         }
    ///     }
    /// }
    /// ```
    pub fn is_retryable(&self) -> bool {
        match self {
            // Primary signal: the host tags retryable transport errors with `RETRYABLE_MARKER`.
            // We also detect the underlying connection-recycled messages directly as a fallback,
            // so this still works against hosts that predate the marker (the SDK and the host
            // ship on independent release cadences).
            HttpError::RequestError(message) => {
                message.contains(RETRYABLE_MARKER) || message_indicates_connection_closed(message)
            }
            HttpError::InternalError
            | HttpError::InvalidUrl { .. }
            | HttpError::InvalidHeaderName { .. }
            | HttpError::InvalidHeaderValue { .. } => false,
        }
    }
}

/// Compatibility fallback for hosts that have not yet adopted [`RETRYABLE_MARKER`]: detect the
/// "connection recycled mid-exchange" error class directly from the underlying transport message.
fn message_indicates_connection_closed(message: &str) -> bool {
    const NEEDLES: [&str; 5] = [
        "connection closed before message completed",
        "unexpected end of file",
        "IncompleteMessage",
        "connection reset",
        "broken pipe",
    ];
    NEEDLES.iter().any(|needle| message.contains(needle))
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
    http::invoke(request.into())
        .map(Into::into)
        .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::HttpError;

    #[test]
    fn retryable_when_host_tags_the_marker() {
        let err = HttpError::RequestError(
            "connection closed by peer before the request/response completed (retryable) \
             (https://search.example.com): error decoding response body"
                .to_string(),
        );
        assert!(err.is_retryable());
    }

    #[test]
    fn retryable_via_fallback_for_untagged_connection_errors() {
        // An older host that has not adopted the marker still surfaces the underlying message.
        for message in [
            "response decode error: error decoding response body: connection closed before message completed",
            "request error: connection reset by peer (os error 104)",
        ] {
            let err = HttpError::RequestError(message.to_string());
            assert!(err.is_retryable(), "expected retryable for: {message}");
        }
    }

    #[test]
    fn not_retryable_for_terminal_errors() {
        assert!(!HttpError::InternalError.is_retryable());
        assert!(
            !HttpError::InvalidUrl {
                url: "not a url".to_string(),
                error: "invalid".to_string(),
            }
            .is_retryable()
        );
        // A genuine response decode failure (not a recycled connection) is not retryable.
        assert!(
            !HttpError::RequestError(
                "response decode error: error decoding response body".to_string()
            )
            .is_retryable()
        );
    }
}
