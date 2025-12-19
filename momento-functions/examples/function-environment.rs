use momento_functions_host::web_extensions::FunctionEnvironment;

momento_functions::post!(function_environment);
fn function_environment(_payload: Vec<u8>) -> String {
    let function_env = FunctionEnvironment::get_function_environment();
    format!(
        r#"Cache: {}
Invocation ID: {},
Query parameters: {},
"#,
        function_env.cache_name(),
        function_env.invocation_id(),
        function_env
            .query_parameters()
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join(", ")
    )
}
