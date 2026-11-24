use momento_functions_guest_web::{WebResponse, WebResult};
use serde::{Deserialize, Serialize};

/// A DynamoDB response, stored in the cache and returned to the caller unchanged.
#[derive(Serialize, Deserialize)]
pub struct CachedResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl CachedResponse {
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }

    pub fn is_client_error(&self) -> bool {
        (400..500).contains(&self.status)
    }

    pub fn into_web_response(self) -> WebResult<WebResponse> {
        Ok(WebResponse::new()
            .with_status(self.status)
            .with_headers(self.headers)
            .with_body(self.body)?)
    }
}
