//! Guest bindings for Momento Spawn Functions.
//!
//! This crate provides the [`spawn!`] macro for implementing a Momento Spawn Function.
//! The spawned function receives a payload and returns a result that the host can use
//! to track success or failure.
//!
//! Functions use `wasm32-wasip2` as the target architecture.
//! They use the [WIT](https://component-model.bytecodealliance.org/design/wit.html) [Component Model](https://component-model.bytecodealliance.org/)
//! to describe the ABI.

mod function_spawn;

/// Internal module for WIT bindings.
#[doc(hidden)]
pub mod wit;

pub use function_spawn::{IntoSpawnResult, spawned_template};
pub use wit::exports::momento::spawn_function::guest_function_spawn::{SpawnFailure, SpawnSuccess};
