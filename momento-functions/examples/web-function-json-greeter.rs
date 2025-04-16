#[derive(serde::Deserialize)]
struct Request {
    name: String,
}

#[derive(serde::Serialize)]
struct Response {
    message: String,
}

momento_functions::post!(greet, Request, Response);
fn greet(request: Request) -> FunctionResult<Response> {
    Ok(Response {
        message: format!("Hello, {}!", request.name),
    })
}
