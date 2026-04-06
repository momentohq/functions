use crate::wit::exports::momento::spawn_function::guest_function_spawn::{
    self as guest_function_spawn, SpawnFailure, SpawnSuccess,
};
use momento_functions_bytes::encoding::Extract;

/// Values returned by a function implemented with the [`spawn!`] macro must implement this trait.
pub trait IntoSpawnResult {
    fn into_spawn_result(self) -> Result<SpawnSuccess, SpawnFailure>;
}

impl IntoSpawnResult for () {
    fn into_spawn_result(self) -> Result<SpawnSuccess, SpawnFailure> {
        Ok(SpawnSuccess::Ok)
    }
}

impl<E: std::fmt::Display> IntoSpawnResult for Result<(), E> {
    fn into_spawn_result(self) -> Result<SpawnSuccess, SpawnFailure> {
        self.map(|_| SpawnSuccess::Ok)
            .map_err(|e| SpawnFailure::FailedNote(e.to_string()))
    }
}

impl IntoSpawnResult for Result<SpawnSuccess, SpawnFailure> {
    fn into_spawn_result(self) -> Result<SpawnSuccess, SpawnFailure> {
        self
    }
}

/// Create a handler for a Momento Spawn Function.
///
/// Your handler receives any type for which [`Extract`](momento_functions_bytes::encoding::Extract) is implemented.
/// It may return `()`, `Result<(), E>`, or `Result<SpawnSuccess, SpawnFailure>`.
/// On payload extraction failure, returns [`SpawnFailure::FailedNote`] with the error message.
///
/// **Raw bytes:**
/// ```rust
/// use momento_functions_bytes::Data;
/// use momento_functions_guest_spawn::spawn;
///
/// spawn!(triggered);
/// fn triggered(_payload: Data) {}
/// ```
///
/// **Typed JSON with result:**
/// ```rust
/// use momento_functions_bytes::encoding::Json;
/// use momento_functions_guest_spawn::spawn;
///
/// #[derive(serde::Deserialize)]
/// struct Request { name: String }
///
/// spawn!(triggered);
/// fn triggered(Json(_req): Json<Request>) -> Result<(), String> {
///     Ok(())
/// }
/// ```
#[macro_export]
macro_rules! spawn {
    ($handler: ident) => {
        struct SpawnFunction;
        momento_functions_guest_spawn::wit::export_spawn_function!(SpawnFunction);

        #[automatically_derived]
        impl momento_functions_guest_spawn::wit::exports::momento::spawn_function::guest_function_spawn::Guest for SpawnFunction {
            fn spawned(
                payload: momento_functions_guest_spawn::wit::exports::momento::spawn_function::guest_function_spawn::Data,
            ) -> Result<
                momento_functions_guest_spawn::SpawnSuccess,
                momento_functions_guest_spawn::SpawnFailure,
            > {
                momento_functions_guest_spawn::spawned_template(payload, $handler)
            }
        }
    };
}

/// Internal helper for the [`spawn!`] macro.
#[doc(hidden)]
pub fn spawned_template<TExtract, TResult: IntoSpawnResult>(
    payload: guest_function_spawn::Data,
    handler: fn(TExtract) -> TResult,
) -> Result<SpawnSuccess, SpawnFailure>
where
    TExtract: Extract,
{
    let payload: momento_functions_bytes::Data = payload.into();
    match TExtract::extract(payload) {
        Ok(request) => handler(request).into_spawn_result(),
        Err(error) => Err(SpawnFailure::FailedNote(format!(
            "Failed to extract spawn payload: {error}"
        ))),
    }
}
