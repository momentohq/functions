//! Host interfaces for spawning Momento Functions.
//!
//! This crate provides a function for invoking a Spawn Function by name,
//! fire-and-forget.

mod spawn;

/// Internal module for WIT bindings.
#[doc(hidden)]
pub mod wit;

pub use spawn::{SpawnError, spawn};
