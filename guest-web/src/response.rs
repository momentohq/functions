use crate::IntoWebResponse;
use crate::wit::exports::momento::web_function::guest_function_web::Response;
use momento_functions_bytes::Data;
use momento_functions_bytes::encoding::Encode;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};

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
            body: message.as_bytes().to_vec().into(),
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
                body: body.into_bytes().into(),
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

/// This represents a response from a web function.
/// When constructed, it's a 200 response with no headers or body.
/// You can set the status, headers, and body via [WebResponse::with_status], [WebResponse::with_headers],
/// and [WebResponse::with_body] respectfully.
#[derive(Debug)]
pub struct WebResponse {
    status: u16,
    headers: Vec<(String, String)>,
    body: Data,
}

impl Default for WebResponse {
    fn default() -> Self {
        Self {
            status: 200,
            headers: vec![],
            body: vec![].into(),
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
        let body = body.try_serialize()?;
        self.body = body;
        Ok(self)
    }
}

impl IntoWebResponse for WebResponse {
    fn response(self) -> Response {
        Response {
            status: self.status,
            headers: self.headers.into_iter().map(Into::into).collect(),
            body: self.body.into(),
        }
    }
}

impl From<(String, String)> for crate::wit::momento::web_function::web_function_support::Header {
    fn from((name, value): (String, String)) -> Self {
        Self { name, value }
    }
}
