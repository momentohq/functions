//! An example Function that receives a JSON payload and returns a JSON response.
//!
//! This is a typed hello world.
//!
//! Invoke this Function with a JSON body like `{"name": "kvc"}` and it will respond with `{"message": "Hello, kvc!"}`.

use core::str;

use momento_functions_bytes::encoding::Json;
use momento_functions_guest_web::invoke;

#[derive(serde::Deserialize)]
struct Request {
    name: String,
}

#[derive(serde::Serialize)]
struct Response {
    message: String,
}

invoke!(greet);
fn greet(Json(request): Json<Request>) -> Json<Response> {
    let Request { name } = request;
    let message = format!("Hello, {}!", name);
    Json(Response { message })
}
