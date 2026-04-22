use momento_functions_bytes::encoding::Json;
use momento_functions_guest_spawn::spawn;

#[derive(serde::Deserialize)]
struct Request {
    _name: String,
}

spawn!(spawned);
fn spawned(Json(_payload): Json<Request>) {}
