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
    Json(Response {
        message: format!("Hello, {}!", request.name),
    })
}
