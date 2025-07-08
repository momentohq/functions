/// Create a handler for a momento::host::spawn::spawn_function.
///
/// You can use raw bytes, or json-marshalled types.
///
/// **Raw:**
/// ```rust
/// momento_functions::spawn!(triggered);
/// fn triggered(payload: Vec<u8>) {
///     ()
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
/// fn greet(request: Request) -> () {
///     Ok(())
/// }
/// ```
#[macro_export]
macro_rules! spawn {
    ($spawn_handler: ident) => {
        struct SpawnFunction;
        momento_functions_wit::function_spawn::export_spawn_function!(SpawnFunction);

        #[automatically_derived]
        impl momento_functions_wit::function_spawn::exports::momento::functions::guest_function_spawn::Guest for SpawnFunction {
            fn spawned(payload: Vec<u8>) {
                $spawn_handler(payload)
            }
        }
    };

    ($post_handler: ident, $request: ident) => {
        struct SpawnFunction;
        momento_functions_wit::function_spawn::export_spawn_function!(SpawnFunction);

        #[automatically_derived]
        impl momento_functions_wit::function_spawn::exports::momento::functions::guest_function_spawn::Guest for SpawnFunction {
            fn spawned(payload: Vec<u8>) {
                let payload: $request = serde_json::from_slice(&payload).expect("payload is not valid json");
                $post_handler(payload)
            }
        }
    }
}
