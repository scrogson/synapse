//! SeaORM backend for Rust code generation
//!
//! Generates SeaORM 2.0 entities with dense format from protobuf definitions.

mod column;
mod entity;
mod enum_gen;
pub mod generator;
mod oneof;
pub mod options;
mod relation;
mod types;
