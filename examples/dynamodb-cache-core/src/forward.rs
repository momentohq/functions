use momento_functions_guest_web::WebResult;
use momento_functions_http::{Authorization, Request, invoke};

use crate::{CachedResponse, headers::forwardable};

/// POST `body` to DynamoDB at `url` and capture the full response. Only
/// [`forwardable`] headers are sent.
pub fn forward(
    url: &str,
    headers: &[(String, String)],
    body: Vec<u8>,
    authorization: Authorization,
) -> WebResult<CachedResponse> {
    let request = Request::new(url, "POST")
        .with_headers(forwardable(headers))
        .with_body(body)
        .with_authorization(authorization);
    let response = invoke(request)?;
    Ok(CachedResponse {
        status: response.status,
        headers: response.headers,
        body: response.body.into_bytes(),
    })
}
