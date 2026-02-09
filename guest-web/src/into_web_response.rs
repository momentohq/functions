use crate::wit::exports::momento::web_function::guest_function_web::Response;
use momento_functions_bytes::{
    Data,
    encoding::{Encode, Json},
};
use serde::Serialize;

macro_rules! content_type {
    ($content_type:expr) => {
        vec![("content-type".to_string(), $content_type.to_string())]
            .into_iter()
            .map(Into::into)
            .collect()
    };
}

/// Values returned by a function implemented with the [crate::invoke!] macro must implement this trait.
pub trait IntoWebResponse {
    fn response(self) -> Response;
}

impl IntoWebResponse for Vec<u8> {
    fn response(self) -> Response {
        Response {
            status: 200,
            headers: content_type!("application/octet-stream"),
            body: Data::from(self).into(),
        }
    }
}

impl IntoWebResponse for &[u8] {
    fn response(self) -> Response {
        Response {
            status: 200,
            headers: content_type!("application/octet-stream"),
            body: Data::from(self.to_vec()).into(),
        }
    }
}

impl IntoWebResponse for String {
    fn response(self) -> Response {
        Response {
            status: 200,
            headers: content_type!("text/plain; charset=utf-8"),
            body: Data::from(self.into_bytes()).into(),
        }
    }
}

impl IntoWebResponse for &str {
    fn response(self) -> Response {
        Response {
            status: 200,
            headers: content_type!("text/plain; charset=utf-8"),
            body: Data::from(self.to_string().into_bytes()).into(),
        }
    }
}

impl IntoWebResponse for () {
    fn response(self) -> Response {
        Response {
            status: 204,
            headers: vec![],
            body: Data::from(Vec::new()).into(),
        }
    }
}

impl IntoWebResponse for Option<Vec<u8>> {
    fn response(self) -> Response {
        Response {
            status: 200,
            headers: content_type!("application/octet-stream"),
            body: Data::from(self.unwrap_or_default()).into(),
        }
    }
}

impl IntoWebResponse for Option<String> {
    fn response(self) -> Response {
        Response {
            status: 200,
            headers: content_type!("text/plain; charset=utf-8"),
            body: Data::from(self.unwrap_or_default().into_bytes()).into(),
        }
    }
}

impl IntoWebResponse for serde_json::Value {
    fn response(self) -> Response {
        match serde_json::to_vec(&self) {
            Ok(body) => Response {
                status: 200,
                headers: content_type!("application/json; charset=utf-8"),
                body: Data::from(body).into(),
            },
            Err(e) => Response {
                status: 500,
                headers: content_type!("text/plain; charset=utf-8"),
                body: Data::from(format!("Failed to encode response: {e}").into_bytes()).into(),
            },
        }
    }
}

impl<T: Serialize> IntoWebResponse for Json<T> {
    fn response(self) -> Response {
        match self.try_serialize() {
            Ok(body) => Response {
                status: 200,
                headers: content_type!("application/json; charset=utf-8"),
                body: body.into(),
            },
            Err(e) => Response {
                status: 500,
                headers: content_type!("text/plain; charset=utf-8"),
                body: Data::from(format!("Failed to encode response: {e}").into_bytes()).into(),
            },
        }
    }
}

impl IntoWebResponse for momento_functions_bytes::Data {
    fn response(self) -> Response {
        Response {
            status: 200,
            headers: content_type!("application/octet-stream"),
            body: self.into(),
        }
    }
}
