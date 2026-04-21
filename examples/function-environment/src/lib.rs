use momento_functions_bytes::Data;
use momento_functions_guest_web::{WebEnvironment, invoke};

invoke!(function_environment);
fn function_environment(_payload: Data) -> String {
    let env = WebEnvironment::load();
    format!(
        r#"Cache: {},
Function: {},
Invocation ID: {},
Query parameters: {},
HTTP method: {}
HTTP path: {}
"#,
        env.cache_name(),
        env.function_name(),
        env.invocation_id(),
        env.query_parameters()
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join(", "),
        env.http_method(),
        env.http_path()
    )
}
