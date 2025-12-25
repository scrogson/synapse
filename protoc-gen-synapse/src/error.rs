//! Error types for code generation
//!
//! This module contains error types used across all code generators.

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
