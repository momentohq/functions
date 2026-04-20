//! Host interfaces for working with AWS Secrets Manager.
//!
//! This crate provides a [`SecretsManagerClient`] for retrieving secrets from
//! AWS Secrets Manager, using Momento's host-provided AWS communication channel.
//!
//! Functions use `wasm32-wasip2` as the target architecture.
//! They use the [WIT](https://component-model.bytecodealliance.org/design/wit.html) [Component Model](https://component-model.bytecodealliance.org/)
//! to describe the ABI.

mod client;

/// Internal module for WIT bindings.
#[doc(hidden)]
pub mod wit;

pub use client::{GetSecretValueRequest, SecretsManagerClient, SecretsManagerGetSecretValueError};

pub use momento_functions_aws_auth::{
    AuthError, Authorization, Credentials, CredentialsProvider, IamRole, provider,
};
