use momento_functions_guest_web::{WebResponse, WebResult};
use serde_json::json;

/// A request this Function refuses: answered with a 400 shaped like a DynamoDB
/// error, so SDKs surface `code` as the error code and don't retry it.
#[derive(Debug)]
pub struct BadRequest {
    code: &'static str,
    message: String,
}

impl BadRequest {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub fn into_response(self) -> WebResult<WebResponse> {
        log::warn!("rejecting request: {}: {}", self.code, self.message);
        let body = json!({
            "__type": format!("com.momento.functions#{}", self.code),
            "message": self.message,
        });
        Ok(WebResponse::new()
            .with_status(400)
            .header("content-type", "application/x-amz-json-1.0")
            .with_body(body.to_string())?)
    }
}
