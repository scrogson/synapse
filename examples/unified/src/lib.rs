//! Synapse Unified Example
//!
//! A unified schema example demonstrating:
//! - IAM service (Users, Organizations, Teams)
//! - Blog service (Authors, Posts)
//! - Cross-service relations (Author -> User)
//!
//! # Features
//!
//! - `full` (default) - Everything in one binary
//! - `gateway` - Just the GraphQL gateway
//! - `iam-service` - IAM gRPC service
//! - `blog-service` - Blog gRPC service
//! - `storage` - SeaORM storage implementations

#![allow(missing_docs)]

mod generated;

pub use generated::*;
