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

/// Current authenticated user context
///
/// This struct is populated from authentication middleware and made
/// available to GraphQL resolvers via `ctx.data::<CurrentUser>()`.
///
/// Fields marked with `from_context` in proto definitions will be
/// automatically populated from this context.
#[derive(Debug, Clone)]
pub struct CurrentUser {
    /// User's internal ID
    pub id: i64,
    /// User's email address
    pub email: String,
    /// User's display name
    pub name: String,
}
