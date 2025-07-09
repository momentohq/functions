use momento_functions::WebResponse;
use momento_functions_host::encoding::Json;

#[derive(serde::Deserialize)]
struct Request {
    name: String,
}

#[derive(serde::Serialize)]
struct Response {
    message: String,
}

momento_functions::post!(greet);
fn greet(Json(request): Json<Request>) -> WebResponse {
    Json(Response {
        message: format!("Hello, {}!", request.name),
    })
    .into()
}
