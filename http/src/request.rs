use momento_functions_bytes::Data;

use crate::wit::momento::http::http;

/// SigV4 credentials for signing an AWS request.
pub struct AwsSigV4Secret {
    /// AWS access key ID.
    pub access_key_id: String,
    /// AWS secret access key.
    pub secret_access_key: String,
    /// AWS region.
    pub region: String,
    /// AWS service name.
    pub service: String,
}

impl From<AwsSigV4Secret> for http::AwsSigv4Secret {
    fn from(s: AwsSigV4Secret) -> Self {
        http::AwsSigv4Secret {
            access_key_id: s.access_key_id,
            secret_access_key: s.secret_access_key,
            region: s.region,
            service: s.service,
        }
    }
}

/// An IAM role for Momento to federate into when making the request.
pub struct IamRole {
    /// The ARN of the IAM role.
    pub role_arn: String,
    /// The AWS service name.
    pub service: String,
}

impl From<IamRole> for http::IamRole {
    fn from(r: IamRole) -> Self {
        http::IamRole {
            role_arn: r.role_arn,
            service: r.service,
        }
    }
}

/// Authorization strategy for an HTTP request.
pub enum Authorization {
    /// No special authorization. You can still include an `Authorization` header manually.
    None,
    /// Sign the request with AWS SigV4 using explicit credentials.
    AwsSigV4Secret(AwsSigV4Secret),
    /// Federate into an IAM role for the request.
    Federated(IamRole),
}

impl From<Authorization> for http::Authorization {
    fn from(a: Authorization) -> Self {
        match a {
            Authorization::None => http::Authorization::None,
            Authorization::AwsSigV4Secret(s) => http::Authorization::AwsSigv4Secret(s.into()),
            Authorization::Federated(r) => http::Authorization::Federated(r.into()),
        }
    }
}

/// An HTTP request.
pub struct Request {
    /// The target URL.
    pub url: String,
    /// The HTTP verb (e.g. `"GET"`, `"POST"`).
    pub verb: String,
    /// Request headers as name-value pairs.
    pub headers: Vec<(String, String)>,
    /// The request body.
    pub body: Data,
    /// The authorization strategy.
    pub authorization: Authorization,
}

impl Request {
    /// Create a new request builder with no body and no authorization.
    ///
    /// # Examples
    /// ________
    /// Build a GET request:
    /// ```rust,no_run
    /// use momento_functions_http::{Request, Authorization};
    ///
    /// let request = Request::new("https://example.com/api", "GET");
    /// ```
    pub fn new(url: impl Into<String>, verb: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            verb: verb.into(),
            headers: Vec::new(),
            body: Data::from(vec![]),
            authorization: Authorization::None,
        }
    }

    /// Add a header to the request.
    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    /// Set the request body.
    pub fn with_body(mut self, body: impl Into<Data>) -> Self {
        self.body = body.into();
        self
    }

    /// Set the authorization strategy.
    pub fn with_authorization(mut self, authorization: Authorization) -> Self {
        self.authorization = authorization;
        self
    }
}

impl From<Request> for http::Request {
    fn from(r: Request) -> Self {
        http::Request {
            url: r.url,
            verb: r.verb,
            headers: r.headers,
            body: r.body.into(),
            authorization: r.authorization.into(),
        }
    }
}
