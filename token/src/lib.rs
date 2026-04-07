//! Host interfaces for generating Momento disposable tokens.
//!
//! This crate provides token generation with scoped permissions for
//! cache, topic, and function access control.

mod generate;
mod permissions;

/// Internal module for WIT bindings.
#[doc(hidden)]
pub mod wit;

pub use generate::{
    GenerateDisposableTokenError, GenerateDisposableTokenResponse, generate_disposable_token,
};
pub use permissions::{
    CacheItemSelector, CachePermissions, CacheRole, CacheSelector, ExplicitPermissions,
    FunctionPermissions, FunctionRole, FunctionSelector, Permissions, PermissionsType,
    TopicPermissions, TopicRole, TopicSelector,
};
