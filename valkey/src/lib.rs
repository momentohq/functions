//! Host interfaces for interacting with managed Valkey clusters.
//!
//! This crate provides a client for executing commands against Valkey clusters
//! that are managed by Momento.

mod client;

/// Internal module for WIT bindings.
#[doc(hidden)]
pub mod wit;

pub use client::{ClusterClient, ValkeyError, Value, get_managed_cluster_client};
