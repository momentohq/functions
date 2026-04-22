use momento_functions_bytes::Data;
use momento_functions_guest_web::invoke;

invoke!(ping);
fn ping(_payload: Data) -> &'static str {
    "pong"
}
