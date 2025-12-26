//! Blog Service Library
//!
//! This crate provides Blog services:
//! - Users
//! - Posts
//!
//! Re-exports the generated gRPC services, storage implementations,
//! and GraphQL types for use by gateways.

mod generated;

pub use generated::blog;
