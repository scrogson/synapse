//! IAM Service Library
//!
//! This crate provides Identity and Access Management services:
//! - Organizations
//! - Teams
//! - Users
//!
//! Re-exports the generated gRPC services, storage implementations,
//! and GraphQL types for use by gateways.

mod generated;

pub use generated::iam;
