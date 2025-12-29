//! Synapse Deno Runtime
//!
//! This crate provides Deno runtime integration for executing custom GraphQL
//! resolvers written in TypeScript/JavaScript.
//!
//! # Example
//!
//! ```ignore
//! use synapse_deno::{DenoResolver, DenoConfig};
//!
//! let config = DenoConfig::default();
//! let resolver = DenoResolver::new(config).await?;
//!
//! // Call a resolver function
//! let result: String = resolver
//!     .call_field_resolver("resolvers/user.ts", "displayName", &user, &())
//!     .await?;
//! ```

#![deny(warnings)]
#![deny(missing_docs)]

mod error;
mod resolver;
mod runtime;

pub use error::DenoError;
pub use resolver::{DenoResolver, DenoConfig, DenoPermissions};
