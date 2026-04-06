//! Host interfaces for AWS authentication in Momento Functions.
//!
//! This crate provides AWS credential types used to authenticate with
//! AWS services (S3, DynamoDB, Lambda, etc.) via Momento's host interfaces.

/// Internal module for WIT bindings.
#[doc(hidden)]
pub mod wit;

pub use wit::momento::aws_auth::aws_auth::{
    AuthError, Authorization, Credentials, CredentialsProvider, IamRole, provider,
};
