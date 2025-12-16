//! Host interface utilities for HTTP

use momento_functions_wit::host::momento::host::http;
use thiserror::Error;

use crate::encoding::EncodeError;
use crate::{
    aws,
    encoding::{Encode, Extract},
};

/// HTTP Get response
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct Response {
    /// HTTP status code
    pub status: u16,
    /// HTTP response headers
    pub headers: Vec<(String, String)>,
    /// HTTP response body
    pub body: Vec<u8>,
}
impl Response {
    /// Take the payload of the response and decode it.
    ///
    /// This consumes the payload; if you call it again, it will return an Error.
    ///
    /// ```rust
    /// # use momento_functions_host::http;
    /// use momento_functions_host::encoding::Json;
    ///
    /// # fn f() -> Result<(), serde_json::error::Error> {
    /// #[derive(serde::Serialize)]
    /// struct Request {
    ///     message: String
    /// }
    /// #[derive(serde::Deserialize)]
    /// struct Reply {
    ///     message: String
    /// }
    ///
    /// let Json(reply): Json<Reply> = http::post(
    ///     "https://gomomento.com",
    ///     [
    ///         ("authorization".to_string(), "abc123".to_string()),
    ///     ],
    ///     Json(Request { message: "hello".to_string() })
    /// )?
    /// .extract()?;
    /// # Ok(()) }
    /// ```
    pub fn extract<E: Extract>(&mut self) -> Result<E, E::Error> {
        E::extract(std::mem::take(&mut self.body))
    }
}

/// An error occurred while calling an HTTP Get method.
#[derive(Debug, Error)]
pub enum HttpGetError {
    /// An error occurred while calling the host http function.
    #[error(transparent)]
    HttpError(#[from] http::Error),
}

/// HTTP GET
///
/// ```rust
/// # use momento_functions_host::http;
///
/// # fn f() -> Result<(), http::HttpGetError> {
/// http::get("https://gomomento.com", [])?;
/// http::get(
///     "https://gomomento.com",
///     [
///         ("authorization".to_string(), "abc123".to_string()),
///     ]
/// )?;
/// # Ok(()) }
/// ```
pub fn get(
    url: impl Into<String>,
    headers: impl IntoIterator<Item = (String, String)>,
) -> Result<Response, HttpGetError> {
    let http::Response {
        status,
        headers,
        body,
    } = http::get(&http::Request {
        url: url.into(),
        headers: headers.into_iter().collect(),
        body: Default::default(),
        authorization: http::Authorization::None,
    })?;
    Ok(Response {
        status,
        headers,
        body,
    })
}

/// An error occurred while calling an HTTP Put method.
#[derive(Debug, Error)]
pub enum HttpPutError<E: EncodeError> {
    /// An error occurred while calling the host http function.
    #[error(transparent)]
    HttpError(#[from] http::Error),
    /// An error occurred while encoding the provided body.
    #[error("Failed to encode body.")]
    EncodeFailed {
        /// The underlying encoding error.
        cause: E,
    },
}

/// HTTP PUT
///
/// ```rust
/// # use momento_functions_host::http;
/// # use momento_functions_host::http::HttpPutError;
/// # fn f() -> Result<(), HttpPutError<&'static str>> {
/// http::put("https://gomomento.com", [], b"hello".as_ref())?;
/// # Ok(())}
///
/// use momento_functions_host::encoding::Json;
/// #[derive(serde::Serialize)]
/// struct MyStruct {
///     message: String
/// }
///
/// # fn g() -> Result<(), HttpPutError<Json<MyStruct>>> {
/// http::put(
///     "https://gomomento.com",
///     [
///         ("authorization".to_string(), "abc123".to_string()),
///     ],
///     Json(MyStruct { message: "hello".to_string() })
/// )?;
/// # Ok(())}
/// ```
pub fn put<E: Encode>(
    url: impl Into<String>,
    headers: impl IntoIterator<Item = (String, String)>,
    body: E,
) -> Result<Response, HttpPutError<E::Error>> {
    let http::Response {
        status,
        headers,
        body,
    } = http::put(&http::Request {
        url: url.into(),
        headers: headers.into_iter().collect(),
        body: body
            .try_serialize()
            .map_err(|e| HttpPutError::EncodeFailed { cause: e })?
            .into(),
        authorization: http::Authorization::None,
    })?;
    Ok(Response {
        status,
        headers,
        body,
    })
}

/// An error occurred while calling an HTTP Post method.
#[derive(Debug, Error)]
pub enum HttpPostError<E: EncodeError> {
    /// An error occurred while calling the host http function.
    #[error(transparent)]
    HttpError(#[from] http::Error),
    /// An error occurred while encoding the provided body.
    #[error("Failed to encode body.")]
    EncodeFailed {
        /// The underlying encoding error.
        cause: E,
    },
}

/// HTTP POST
///
/// ```rust
/// # use momento_functions_host::http;
/// # use momento_functions_host::http::HttpPostError;
///
/// # fn f() -> Result<(), HttpPostError<&'static str>> {
/// http::post("https://gomomento.com", [], b"hello".as_ref())?;
/// # Ok(())}
///
/// use momento_functions_host::encoding::Json;
///
/// #[derive(serde::Serialize)]
/// struct Request {
///     message: String
/// }
/// #[derive(serde::Deserialize)]
/// struct Reply {
///     message: String
/// }
/// # fn g() -> Result<(), HttpPostError<Json<Request>>> {
///
/// let Json(reply): Json<Reply> = http::post(
///     "https://gomomento.com",
///     [
///         ("authorization".to_string(), "abc123".to_string()),
///     ],
///     Json(Request { message: "hello".to_string() })
/// )?
/// .extract()?;
/// # Ok(()) }
/// ```
pub fn post<E: Encode>(
    url: impl Into<String>,
    headers: impl IntoIterator<Item = (String, String)>,
    body: E,
) -> Result<Response, HttpPostError<E::Error>> {
    let http::Response {
        status,
        headers,
        body,
    } = http::post(&http::Request {
        url: url.into(),
        headers: headers.into_iter().collect(),
        body: body
            .try_serialize()
            .map_err(|e| HttpPostError::EncodeFailed { cause: e })?
            .into(),
        authorization: http::Authorization::None,
    })?;
    Ok(Response {
        status,
        headers,
        body,
    })
}

/// An error occurred while calling an HTTP Delete method.
#[derive(Debug, Error)]
pub enum HttpDeleteError {
    /// An error occurred while calling the host http function.
    #[error(transparent)]
    HttpError(#[from] http::Error),
}

/// HTTP DELETE
///
/// ```rust
/// # use momento_functions_host::http;
/// # use momento_functions_host::http::HttpDeleteError;
///
/// fn f() -> Result<(), HttpDeleteError> {
/// http::delete("https://gomomento.com", [])?;
/// http::delete(
///     "https://gomomento.com",
///     [
///         ("authorization".to_string(), "abc123".to_string()),
///     ]
/// )?;
/// # Ok(()) }
/// ```
pub fn delete(
    url: impl Into<String>,
    headers: impl IntoIterator<Item = (String, String)>,
) -> Result<Response, HttpDeleteError> {
    let http::Response {
        status,
        headers,
        body,
    } = http::delete(&http::Request {
        url: url.into(),
        headers: headers.into_iter().collect(),
        body: Default::default(),
        authorization: http::Authorization::None,
    })?;
    Ok(Response {
        status,
        headers,
        body,
    })
}

impl aws::auth::Credentials {
    fn into_http(
        self,
        region: impl Into<String>,
        service: impl Into<String>,
    ) -> http::Authorization {
        match self {
            aws::auth::Credentials::Hardcoded {
                access_key_id,
                secret_access_key,
            } => http::Authorization::AwsSigv4Secret(http::AwsSigv4Secret {
                access_key_id,
                secret_access_key,
                region: region.into(),
                service: service.into(),
            }),
            aws::auth::Credentials::Federated { role_arn } => {
                http::Authorization::Federated(http::IamRole {
                    role_arn,
                    service: service.into(),
                })
            }
        }
    }
}

/// HTTP GET with AWS SigV4 signing provided by the host
///
/// ```rust
/// # use momento_functions_host::http;
/// use momento_functions_host::build_environment_aws_credentials;
///
/// # fn f() -> Result<(), http::HttpGetError> {
/// http::get_aws_sigv4(
///     "https://bedrock-runtime.us-west-2.amazonaws.com/model/us.amazon.nova-pro-v1:0/invoke",
///     [],
///     build_environment_aws_credentials!(),
///     "us-west-2",
///     "bedrock",
/// )?;
/// http::get_aws_sigv4(
///     "https://bedrock-runtime.us-west-2.amazonaws.com/model/us.amazon.nova-pro-v1:0/invoke",
///     [
///         ("other_header".to_string(), "abc123".to_string()),
///     ],
///     build_environment_aws_credentials!(),
///     "us-west-2",
///     "bedrock",
/// )?;
/// # Ok(()) }
/// ```
pub fn get_aws_sigv4(
    url: impl Into<String>,
    headers: impl IntoIterator<Item = (String, String)>,
    aws_credentials: aws::auth::Credentials,
    region: impl Into<String>,
    service: impl Into<String>,
) -> Result<Response, HttpGetError> {
    let http::Response {
        status,
        headers,
        body,
    } = http::get(&http::Request {
        url: url.into(),
        headers: headers.into_iter().collect(),
        body: Default::default(),
        authorization: aws_credentials.into_http(region, service),
    })?;
    Ok(Response {
        status,
        headers,
        body,
    })
}

/// HTTP PUT with AWS SigV4 signing provided by the host
///
/// ```rust
/// # use momento_functions_host::http;
/// use momento_functions_host::encoding::Json;
/// use momento_functions_host::build_environment_aws_credentials;
///
/// #[derive(serde::Serialize)]
/// struct MyStruct {
///     message: String
/// }
/// # fn f() -> Result<(), http::HttpPutError<Json<MyStruct>>> {
///
/// http::put_aws_sigv4(
///     "https://gomomento.com",
///     [
///         ("authorization".to_string(), "abc123".to_string()),
///     ],
///     build_environment_aws_credentials!(),
///     "us-west-2",
///     "bedrock",
///     Json(MyStruct { message: "hello".to_string() })
/// )?;
/// # Ok(()) }
/// ```
pub fn put_aws_sigv4<E: Encode>(
    url: impl Into<String>,
    headers: impl IntoIterator<Item = (String, String)>,
    aws_credentials: aws::auth::Credentials,
    region: impl Into<String>,
    service: impl Into<String>,
    body: E,
) -> Result<Response, HttpPutError<E::Error>> {
    let http::Response {
        status,
        headers,
        body,
    } = http::put(&http::Request {
        url: url.into(),
        headers: headers.into_iter().collect(),
        body: body
            .try_serialize()
            .map_err(|e| HttpPutError::EncodeFailed { cause: e })?
            .into(),
        authorization: aws_credentials.into_http(region, service),
    })?;
    Ok(Response {
        status,
        headers,
        body,
    })
}

/// HTTP POST with AWS SigV4 signing provided by the host
///
/// ```rust
/// # use momento_functions_host::http;
/// use momento_functions_host::encoding::Json;
/// use momento_functions_host::build_environment_aws_credentials;
///
/// #[derive(serde::Serialize)]
/// struct MyStruct {
///     message: String
/// }
/// # fn f() -> Result<(), http::HttpPostError<Json<MyStruct>>> {
///
/// http::post_aws_sigv4(
///     "https://gomomento.com",
///     [
///         ("authorization".to_string(), "abc123".to_string()),
///     ],
///     build_environment_aws_credentials!(),
///     "us-west-2",
///     "bedrock",
///     Json(MyStruct { message: "hello".to_string() })
/// )?;
/// # Ok(()) }
/// ```
pub fn post_aws_sigv4<E: Encode>(
    url: impl Into<String>,
    headers: impl IntoIterator<Item = (String, String)>,
    aws_credentials: aws::auth::Credentials,
    region: impl Into<String>,
    service: impl Into<String>,
    body: E,
) -> Result<Response, HttpPostError<E::Error>> {
    let http::Response {
        status,
        headers,
        body,
    } = http::post(&http::Request {
        url: url.into(),
        headers: headers.into_iter().collect(),
        body: body
            .try_serialize()
            .map_err(|e| HttpPostError::EncodeFailed { cause: e })?
            .into(),
        authorization: aws_credentials.into_http(region, service),
    })?;
    Ok(Response {
        status,
        headers,
        body,
    })
}

/// HTTP DELETE with AWS SigV4 signing provided by the host
///
/// ```rust
/// # use momento_functions_host::http;
/// use momento_functions_host::build_environment_aws_credentials;
///
/// # fn f() -> Result<(), http::HttpDeleteError> {
/// http::delete_aws_sigv4(
///     "https://bedrock-runtime.us-west-2.amazonaws.com/model/us.amazon.nova-pro-v1:0/invoke",
///     [],
///     build_environment_aws_credentials!(),
///     "us-west-2",
///     "bedrock",
/// )?;
/// http::delete_aws_sigv4(
///     "https://bedrock-runtime.us-west-2.amazonaws.com/model/us.amazon.nova-pro-v1:0/invoke",
///     [
///         ("other_header".to_string(), "abc123".to_string()),
///     ],
///     build_environment_aws_credentials!(),
///     "us-west-2",
///     "bedrock",
/// )?;
/// # Ok(()) }
/// ```
pub fn delete_aws_sigv4(
    url: impl Into<String>,
    headers: impl IntoIterator<Item = (String, String)>,
    aws_credentials: aws::auth::Credentials,
    region: impl Into<String>,
    service: impl Into<String>,
) -> Result<Response, HttpDeleteError> {
    let http::Response {
        status,
        headers,
        body,
    } = http::delete(&http::Request {
        url: url.into(),
        headers: headers.into_iter().collect(),
        body: Default::default(),
        authorization: aws_credentials.into_http(region, service),
    })?;
    Ok(Response {
        status,
        headers,
        body,
    })
}
