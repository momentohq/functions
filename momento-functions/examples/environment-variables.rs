use itertools::Itertools;

momento_functions::post!(env);
fn env(_payload: Vec<u8>) -> String {
    std::env::vars().map(|(k, v)| format!("{k}={v}")).join("\n")
}
