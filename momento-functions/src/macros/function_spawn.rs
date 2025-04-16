/// Create a handler for a momento::host::spawn::spawn_function.
///
/// You can use raw bytes, or json-marshalled types.
///
/// **Raw:**
/// ```rust
/// momento_functions::spawn!(triggered);
/// fn triggered(payload: Vec<u8>) -> FunctionResult<()> {
///     Ok(())
/// }
/// ```
///
/// **Typed JSON:**
/// ```rust
/// #[derive(serde::Deserialize)]
/// struct Request {
///     name: String,
/// }
///
/// momento_functions::spawn!(greet, Request);
/// fn greet(request: Request) -> FunctionResult<()> {
///     Ok(())
/// }
/// ```
#[macro_export]
macro_rules! spawn {
    ($spawn_handler: ident) => {
        use momento_functions_host::FunctionResult;
        struct SpawnFunction;
        momento_functions_wit::function_spawn::export_spawn_function!(SpawnFunction);

        #[automatically_derived]
        impl momento_functions_wit::function_spawn::exports::momento::functions::guest_function_spawn::Guest for SpawnFunction {
            fn spawned(payload: Vec<u8>) -> Result<(), momento_functions_wit::function_spawn::momento::functions::types::InvocationError> {
                $spawn_handler(payload).map_err(Into::into)
            }
        }
    };

    ($post_handler: ident, $request: ident) => {
        use momento_functions_host::FunctionResult;
        struct SpawnFunction;
        momento_functions_wit::function_spawn::export_spawn_function!(SpawnFunction);

        #[automatically_derived]
        impl momento_functions_wit::function_spawn::exports::momento::functions::guest_function_spawn::Guest for SpawnFunction {
            fn spawned(payload: Vec<u8>) -> Result<(), momento_functions_wit::function_spawn::momento::functions::types::InvocationError> {
                let payload: $request = serde_json::from_slice(&payload)
                    .map_err(|e| momento_functions_wit::function_spawn::momento::functions::types::InvocationError::RequestError(format!("could not deserialize json: {e:?}")))?;
                $post_handler(payload).map_err(Into::into)
            }
        }
    }
}
