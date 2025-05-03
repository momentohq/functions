use momento_functions_host::{FunctionResult, encoding::Payload};

/// A response from a web function invocation
pub trait WebResponse {
    /// Status code of the response
    fn get_status_code(&self) -> u16;

    /// Response headers
    ///
    /// Called only once, this should consume the internal vector
    fn take_headers(&mut self) -> Vec<(String, String)>;

    /// Take the payload of the response
    ///
    /// Called only once, this should consume the internal payload
    fn take_payload(self) -> impl Payload;
}

/// Just treat a present payload as a 200
impl<T> WebResponse for T
where
    T: Payload,
{
    fn get_status_code(&self) -> u16 {
        200
    }

    fn take_headers(&mut self) -> Vec<(String, String)> {
        vec![]
    }

    fn take_payload(self) -> impl Payload {
        self
    }
}

/// A builder for a web response
///
/// ________
/// Bytes:
/// ```rust
/// momento_functions::WebResponseBuilder::new()
///     .status_code(200)
///     .headers(vec![("Content-Type".to_string(), "application/json".to_string())])
///     .payload(b"{\"some\": \"json\"}".to_vec());
/// ```
///
/// ________
/// Typed JSON:
/// ```rust
/// use momento_functions::WebResponseBuilder;
/// use momento_functions_host::encoding::Json;
///
/// #[derive(serde::Serialize)]
/// struct MyStruct {
///     hello: String
/// }
///
/// WebResponseBuilder::new()
///     .status_code(200)
///     .headers(vec![("Content-Type".to_string(), "application/json".to_string())])
///     .payload(Json(MyStruct { hello: "hello".to_string() }));
/// ```
#[derive(Debug)]
pub struct WebResponseBuilder {
    status_code: u16,
    headers: Option<Vec<(String, String)>>,
    payload: Option<Vec<u8>>,
}
impl WebResponseBuilder {
    /// Create a new web response builder
    #[allow(clippy::new_without_default)]
    pub fn new() -> WebResponseBuilder {
        WebResponseBuilder {
            status_code: 200,
            headers: None,
            payload: None,
        }
    }

    /// Set the status code of the response
    pub fn status_code(mut self, status_code: u16) -> Self {
        self.status_code = status_code;
        self
    }

    /// Set the headers of the response
    pub fn headers(mut self, headers: Vec<(String, String)>) -> Self {
        self.headers = Some(headers);
        self
    }

    /// Set the payload of the response
    pub fn payload(mut self, payload: impl Payload) -> FunctionResult<Self> {
        self.payload = payload.try_serialize()?.map(Into::into);
        Ok(self)
    }
}

impl WebResponse for WebResponseBuilder {
    fn get_status_code(&self) -> u16 {
        self.status_code
    }

    fn take_headers(&mut self) -> Vec<(String, String)> {
        self.headers.take().unwrap_or_default()
    }

    fn take_payload(self) -> impl Payload {
        self.payload.unwrap_or_default()
    }
}
