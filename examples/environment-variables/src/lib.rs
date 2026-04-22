use itertools::Itertools;
use momento_functions_bytes::Data;
use momento_functions_guest_web::invoke;

invoke!(env);
fn env(_payload: Data) -> String {
    std::env::vars().map(|(k, v)| format!("{k}={v}")).join("\n")
}
