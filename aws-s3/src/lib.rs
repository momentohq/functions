//! Host interfaces for working with AWS S3.
//!
//! This crate provides an [`S3Client`] for putting and getting objects in S3,
//! using Momento's host-provided AWS communication channel.
//!
//! Functions use `wasm32-wasip2` as the target architecture.
//! They use the [WIT](https://component-model.bytecodealliance.org/design/wit.html) [Component Model](https://component-model.bytecodealliance.org/)
//! to describe the ABI.

mod client;

/// Internal module for WIT bindings.
#[doc(hidden)]
pub mod wit;

pub use client::{S3Client, S3GetError, S3PutError};

pub use momento_functions_aws_auth::{
    AuthError, Authorization, Credentials, CredentialsProvider, IamRole, provider,
};
