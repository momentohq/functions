use itertools::Itertools;
use momento_functions::WebResponse;

momento_functions::post!(env);
fn env(_payload: Vec<u8>) -> WebResponse {
    std::env::vars()
        .map(|(k, v)| format!("{k}={v}"))
        .join("\n")
        .into()
}
