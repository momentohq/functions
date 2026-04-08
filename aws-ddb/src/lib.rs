//! Host interfaces for working with AWS DynamoDB.
//!
//! This crate provides a [`DynamoDBClient`] for putting and getting items in DynamoDB,
//! using Momento's host-provided AWS communication channel.

mod client;
mod types;

/// Internal module for WIT bindings.
#[doc(hidden)]
pub mod wit;

pub use client::{DynamoDBClient, DynamoDBError, GetItemError};
pub use types::{
    AttributeValue, BinaryConversionError, ConversionError, Item, Key, KeyValue,
    NumericConversionError,
};

pub use momento_functions_aws_auth::{
    AuthError, Authorization, Credentials, CredentialsProvider, IamRole, provider,
};
