use crate::IntoWebResponse;
use momento_functions_host::encoding::{Encode, Json};
use momento_functions_wit::function_web::exports::momento::functions::guest_function_web::Response;
use serde::Serialize;

macro_rules! content_type {
    ($content_type:expr) => {
        vec![("content-type".to_string(), $content_type.to_string())]
            .into_iter()
            .map(Into::into)
            .collect()
    };
}

impl IntoWebResponse for Vec<u8> {
    fn response(self) -> Response {
        Response {
            status: 200,
            headers: content_type!("application/octet-stream"),
            body: self,
        }
    }
}

impl IntoWebResponse for &[u8] {
    fn response(self) -> Response {
        Response {
            status: 200,
            headers: content_type!("application/octet-stream"),
            body: self.to_vec(),
        }
    }
}

impl IntoWebResponse for String {
    fn response(self) -> Response {
        Response {
            status: 200,
            headers: content_type!("text/plain; charset=utf-8"),
            body: self.into_bytes(),
        }
    }
}

impl IntoWebResponse for &str {
    fn response(self) -> Response {
        Response {
            status: 200,
            headers: content_type!("text/plain; charset=utf-8"),
            body: self.to_string().into_bytes(),
        }
    }
}

impl IntoWebResponse for () {
    fn response(self) -> Response {
        Response {
            status: 204,
            headers: vec![],
            body: vec![],
        }
    }
}

impl IntoWebResponse for Option<Vec<u8>> {
    fn response(self) -> Response {
        Response {
            status: 200,
            headers: content_type!("application/octet-stream"),
            body: self.unwrap_or_default(),
        }
    }
}

impl IntoWebResponse for Option<String> {
    fn response(self) -> Response {
        Response {
            status: 200,
            headers: content_type!("text/plain; charset=utf-8"),
            body: self.unwrap_or_default().into_bytes(),
        }
    }
}

impl IntoWebResponse for serde_json::Value {
    fn response(self) -> Response {
        match serde_json::to_vec(&self) {
            Ok(body) => Response {
                status: 200,
                headers: content_type!("application/json; charset=utf-8"),
                body,
            },
            Err(e) => Response {
                status: 500,
                headers: vec![],
                body: format!("Failed to encode response: {e}").into(),
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
                headers: vec![],
                body: format!("Failed to encode response: {e}").into(),
            },
        }
    }
}
