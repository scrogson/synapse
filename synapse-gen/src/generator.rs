// Code generator traits and types for Synapse.

use std::collections::HashMap;

/// Error type for code generation failures.
#[derive(Debug, thiserror::Error)]
pub enum GeneratorError {
    /// An I/O error occurred.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// A code generation error occurred.
    #[error("code generation error: {0}")]
    CodeGen(String),
}

/// A file produced by a code generator.
#[derive(Debug, Clone)]
pub struct GeneratedFile {
    /// The relative path of the generated file.
    pub path: String,
    /// The content of the generated file.
    pub content: String,
}

/// Context provided to code generators during generation.
#[derive(Debug, Default)]
pub struct GeneratorContext {
    /// Arbitrary parameters passed to the generator.
    pub params: HashMap<String, String>,
}

/// Trait for implementing code generators.
pub trait CodeGenerator {
    /// Returns the name of this generator.
    fn name(&self) -> &str;

    /// Generate code, returning a list of files.
    fn generate(&self, ctx: &GeneratorContext) -> Result<Vec<GeneratedFile>, GeneratorError>;
}
