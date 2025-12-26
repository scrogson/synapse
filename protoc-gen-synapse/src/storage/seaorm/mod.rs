//! SeaORM backend for Rust code generation
//!
//! Generates SeaORM 2.0 entities with dense format from protobuf definitions.

mod column;
pub mod conversion;
mod entity;
mod enum_gen;
pub mod generator;
pub mod implementation;
mod oneof;
pub mod options;
pub mod package;
mod relation;
mod types;
