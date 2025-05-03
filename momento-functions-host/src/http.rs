//! Host interface utilities for HTTP

use momento_functions_wit::host::momento::host::http;

use crate::{
    FunctionResult, aws,
    encoding::{Extract, Payload},
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
    /// # use momento_functions_host::FunctionResult;
    /// # use momento_functions_host::http;
    /// use momento_functions_host::encoding::Json;
    ///
    /// # fn f() -> FunctionResult<()> {
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
    pub fn extract<E: Extract>(&mut self) -> FunctionResult<E> {
        E::extract(std::mem::take(&mut self.body)).map_err(|e| {
            crate::Error::MessageError(format!(
                "status: {status} failed to deserialize json: {e}",
                status = self.status
            ))
        })
    }
}

/// HTTP GET
///
/// ```rust
/// # use momento_functions_host::FunctionResult;
/// # use momento_functions_host::http;
///
/// # fn f() -> FunctionResult<()> {
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
) -> FunctionResult<Response> {
    let http::Response {
        status,
        headers,
        body,
    } = http::get(&http::Request {
        url: url.into(),
        headers: headers.into_iter().collect(),
        body: Default::default(),
        authorization: http::Authorization::None,
    });
    Ok(Response {
        status,
        headers,
        body,
    })
}

/// HTTP PUT
///
/// ```rust
/// # use momento_functions_host::FunctionResult;
/// # use momento_functions_host::http;
///
/// # fn f() -> FunctionResult<()> {
/// http::put("https://gomomento.com", [], b"hello".as_ref())?;
///
/// use momento_functions_host::encoding::Json;
///
/// #[derive(serde::Serialize)]
/// struct MyStruct {
///     message: String
/// }
///
/// http::put(
///     "https://gomomento.com",
///     [
///         ("authorization".to_string(), "abc123".to_string()),
///     ],
///     Json(MyStruct { message: "hello".to_string() })
/// )?;
/// # Ok(()) }
/// ```
pub fn put(
    url: impl Into<String>,
    headers: impl IntoIterator<Item = (String, String)>,
    body: impl Payload,
) -> FunctionResult<Response> {
    let http::Response {
        status,
        headers,
        body,
    } = http::put(&http::Request {
        url: url.into(),
        headers: headers.into_iter().collect(),
        body: body.try_serialize()?.map(Into::into).unwrap_or_default(),
        authorization: http::Authorization::None,
    });
    Ok(Response {
        status,
        headers,
        body,
    })
}

/// HTTP POST
///
/// ```rust
/// # use momento_functions_host::FunctionResult;
/// # use momento_functions_host::http;
///
/// # fn f() -> FunctionResult<()> {
/// http::post("https://gomomento.com", [], b"hello".as_ref())?;
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
pub fn post(
    url: impl Into<String>,
    headers: impl IntoIterator<Item = (String, String)>,
    body: impl Payload,
) -> FunctionResult<Response> {
    let http::Response {
        status,
        headers,
        body,
    } = http::post(&http::Request {
        url: url.into(),
        headers: headers.into_iter().collect(),
        body: body.try_serialize()?.map(Into::into).unwrap_or_default(),
        authorization: http::Authorization::None,
    });
    Ok(Response {
        status,
        headers,
        body,
    })
}

/// HTTP DELETE
///
/// ```rust
/// # use momento_functions_host::FunctionResult;
/// # use momento_functions_host::http;
///
/// # fn f() -> FunctionResult<()> {
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
) -> FunctionResult<Response> {
    let http::Response {
        status,
        headers,
        body,
    } = http::delete(&http::Request {
        url: url.into(),
        headers: headers.into_iter().collect(),
        body: Default::default(),
        authorization: http::Authorization::None,
    });
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
        }
    }
}

/// HTTP GET with AWS SigV4 signing provided by the host
///
/// ```rust
/// # use momento_functions_host::FunctionResult;
/// # use momento_functions_host::http;
/// use momento_functions_host::build_environment_aws_credentials;
///
/// # fn f() -> FunctionResult<()> {
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
) -> FunctionResult<Response> {
    let http::Response {
        status,
        headers,
        body,
    } = http::get(&http::Request {
        url: url.into(),
        headers: headers.into_iter().collect(),
        body: Default::default(),
        authorization: aws_credentials.into_http(region, service),
    });
    Ok(Response {
        status,
        headers,
        body,
    })
}

/// HTTP PUT with AWS SigV4 signing provided by the host
///
/// ```rust
/// # use momento_functions_host::FunctionResult;
/// # use momento_functions_host::http;
/// use momento_functions_host::encoding::Json;
/// use momento_functions_host::build_environment_aws_credentials;
/// # fn f() -> FunctionResult<()> {
/// #[derive(serde::Serialize)]
/// struct MyStruct {
///     message: String
/// }
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
pub fn put_aws_sigv4(
    url: impl Into<String>,
    headers: impl IntoIterator<Item = (String, String)>,
    aws_credentials: aws::auth::Credentials,
    region: impl Into<String>,
    service: impl Into<String>,
    body: impl Payload,
) -> FunctionResult<Response> {
    let http::Response {
        status,
        headers,
        body,
    } = http::put(&http::Request {
        url: url.into(),
        headers: headers.into_iter().collect(),
        body: body.try_serialize()?.map(Into::into).unwrap_or_default(),
        authorization: aws_credentials.into_http(region, service),
    });
    Ok(Response {
        status,
        headers,
        body,
    })
}

/// HTTP POST with AWS SigV4 signing provided by the host
///
/// ```rust
/// # use momento_functions_host::FunctionResult;
/// # use momento_functions_host::http;
/// use momento_functions_host::encoding::Json;
/// use momento_functions_host::build_environment_aws_credentials;
/// # fn f() -> FunctionResult<()> {
/// #[derive(serde::Serialize)]
/// struct MyStruct {
///     message: String
/// }
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
pub fn post_aws_sigv4(
    url: impl Into<String>,
    headers: impl IntoIterator<Item = (String, String)>,
    aws_credentials: aws::auth::Credentials,
    region: impl Into<String>,
    service: impl Into<String>,
    body: impl Payload,
) -> FunctionResult<Response> {
    let http::Response {
        status,
        headers,
        body,
    } = http::post(&http::Request {
        url: url.into(),
        headers: headers.into_iter().collect(),
        body: body.try_serialize()?.map(Into::into).unwrap_or_default(),
        authorization: aws_credentials.into_http(region, service),
    });
    Ok(Response {
        status,
        headers,
        body,
    })
}

/// HTTP DELETE with AWS SigV4 signing provided by the host
///
/// ```rust
/// # use momento_functions_host::FunctionResult;
/// # use momento_functions_host::http;
/// use momento_functions_host::build_environment_aws_credentials;
///
/// # fn f() -> FunctionResult<()> {
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
) -> FunctionResult<Response> {
    let http::Response {
        status,
        headers,
        body,
    } = http::delete(&http::Request {
        url: url.into(),
        headers: headers.into_iter().collect(),
        body: Default::default(),
        authorization: match aws_credentials {
            aws::auth::Credentials::Hardcoded {
                access_key_id,
                secret_access_key,
            } => http::Authorization::AwsSigv4Secret(http::AwsSigv4Secret {
                access_key_id,
                secret_access_key,
                region: region.into(),
                service: service.into(),
            }),
        },
    });
    Ok(Response {
        status,
        headers,
        body,
    })
}
