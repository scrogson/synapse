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
mod service;
mod types;

/// Error type for code generation
#[derive(Debug, thiserror::Error)]
pub enum GeneratorError {
    /// Code generation failed
    #[error("code generation error: {0}")]
    CodeGenError(String),

    /// Failed to parse protobuf descriptor
    #[error("parse error: {0}")]
    Parse(String),

    /// Failed to decode protobuf message
    #[error("decode error: {0}")]
    DecodeError(String),
}

impl From<String> for GeneratorError {
    fn from(s: String) -> Self {
        GeneratorError::CodeGenError(s)
    }
}
