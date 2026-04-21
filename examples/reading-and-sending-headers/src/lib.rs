use std::collections::HashMap;

use itertools::Itertools;
use momento_functions_bytes::encoding::Json;
use momento_functions_guest_web::{WebEnvironment, WebResponse, WebResult, invoke};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Request {
    headers_to_send_back: HashMap<String, String>,
}

invoke!(headers_example);
fn headers_example(Json(request): Json<Request>) -> WebResult<WebResponse> {
    let headers = WebEnvironment::load().headers();
    // Don't expose secrets!
    let headers_to_return = headers
        .iter()
        .filter(|(k, _)| *k != "authorization")
        .map(|(k, v)| format!("'{k}'='{v}'"))
        .join(" | ");
    Ok(WebResponse::new()
        .with_status(200)
        .with_headers(request.headers_to_send_back.into_iter().collect())
        .with_body(format!("You sent this with headers: {headers_to_return}"))?)
}
