use momento_functions_host::encoding::Encode;
use momento_functions_host::http;
use momento_functions_wit::function_web::exports::momento::functions::guest_function_web::Response;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};

/// Values returned by a function implemented with the [crate::post!] macro must implement this trait.
pub trait IntoWebResponse {
    fn response(self) -> Response;
}

/// A WebError represents an error result produced by a function execution.
/// Functionally, it is also just an HTTP response - however, this allows for writing
/// functions with a return signature of `WebResult` if you are okay with all errors
/// being converted to 500s and returned in the body.
#[derive(Debug)]
pub struct WebError {
    source: Option<Box<dyn Error>>,
    response: WebResponse,
}

impl WebError {
    pub fn message(message: impl Into<String>) -> Self {
        let message = message.into();
        let response = WebResponse {
            status: 500,
            headers: vec![],
            body: message.as_bytes().to_vec(),
        };
        Self {
            source: None,
            response,
        }
    }
}

impl<E: Error + 'static> From<E> for WebError {
    fn from(e: E) -> Self {
        let body = format!("An error occurred during function invocation: {e}");
        Self {
            source: Some(Box::new(e)),
            response: WebResponse {
                status: 500,
                headers: vec![],
                body: body.into_bytes(),
            },
        }
    }
}

impl Display for WebError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "WebError(Source: {:?})", self.source)
    }
}

/// A Result type for implementing functions. Allows you to use `?` within your function body
/// to return a 500 with the error details.
pub type WebResult<T> = Result<T, WebError>;

impl<R> IntoWebResponse for Result<R, WebError>
where
    R: IntoWebResponse,
{
    fn response(self) -> Response {
        match self {
            Ok(r) => r.response(),
            Err(e) => e.response.response(),
        }
    }
}

impl IntoWebResponse for http::Response {
    fn response(self) -> Response {
        Response {
            status: self.status,
            headers: self.headers.into_iter().map(Into::into).collect(),
            body: self.body,
        }
    }
}

/// This represents a response from a web function.
/// When constructed, it's a 200 response with no headers or body.
/// You can set the status, headers, and body via [WebResponse::with_status], [WebResponse::with_headers],
/// and [WebResponse::with_body] respectfully.
#[derive(Debug)]
pub struct WebResponse {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl Default for WebResponse {
    fn default() -> Self {
        Self {
            status: 200,
            headers: vec![],
            body: vec![],
        }
    }
}

impl WebResponse {
    /// Creates a new default response.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the response status.
    pub fn with_status(mut self, status: u16) -> Self {
        self.status = status;
        self
    }

    /// Adds a header to the response.
    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((key.into(), value.into()));
        self
    }

    /// Overrides the collection of headers for the response.
    pub fn with_headers(mut self, headers: Vec<(String, String)>) -> Self {
        self.headers = headers;
        self
    }

    /// Sets the response body. If encoding the body fails, returns an error.
    pub fn with_body<E: Encode>(mut self, body: E) -> Result<Self, E::Error> {
        let body = body.try_serialize().map(Into::into)?;
        self.body = body;
        Ok(self)
    }
}

impl IntoWebResponse for WebResponse {
    fn response(self) -> Response {
        Response {
            status: self.status,
            headers: self.headers.into_iter().map(Into::into).collect(),
            body: self.body,
        }
    }
}

impl<E: Encode> From<E> for WebResponse {
    fn from(value: E) -> Self {
        WebResponse::default().with_body(value).unwrap_or_else(|e| {
            WebResponse::default()
                .with_status(500)
                .with_body(format!("Failed to encode response body: {e}"))
                .expect("String encoding is infallible")
        })
    }
}
