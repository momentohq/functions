use std::collections::HashMap;

use itertools::Itertools;
use momento_functions::{WebResponse, WebResult};
use momento_functions_host::{encoding::Json, web_extensions::headers};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Request {
    headers_to_send_back: HashMap<String, String>,
}

momento_functions::post!(headers_example);
fn headers_example(Json(request): Json<Request>) -> WebResult<WebResponse> {
    // Get the headers passed in when function was invoked
    let headers = headers();
    // Don't expose secrets!
    let headers_to_return = headers
        .iter()
        .filter(|(k, _)| *k != "authorization")
        .map(|(k, v)| format!("'{k}'='{v}'"))
        .join(" | ");
    Ok(WebResponse::new()
        .with_status(200)
        // Collect and send back the headers passed in
        .with_headers(
            request
                .headers_to_send_back
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        )
        .with_body(format!("You sent this with headers: {headers_to_return}"))?)
}
