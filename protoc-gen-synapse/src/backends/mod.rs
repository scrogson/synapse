//! Backend implementations for code generation
//!
//! Each backend generates code for a specific ORM/language combination.

mod seaorm;

use crate::ir::{Entity, Enum};
use prost_types::compiler::code_generator_response::File;

/// A code generation backend
pub trait Backend: Send + Sync {
    /// Backend name (e.g., "seaorm", "ecto", "gorm")
    fn name(&self) -> &str;

    /// File extension for generated files
    fn file_extension(&self) -> &str;

    /// Generate entity/model code
    fn generate_entity(&self, entity: &Entity) -> Result<File, BackendError>;

    /// Generate enum code
    fn generate_enum(&self, enum_def: &Enum) -> Result<File, BackendError>;

    /// Generate storage trait code (optional)
    fn generate_storage_trait(
        &self,
        _service_name: &str,
        _methods: &[crate::ir::Method],
    ) -> Result<Option<File>, BackendError> {
        Ok(None)
    }

    /// Generate prelude/imports file (optional)
    fn generate_prelude(&self, _package: &str) -> Result<Option<File>, BackendError> {
        Ok(None)
    }

    /// Generate module file (mod.rs, etc.) (optional)
    fn generate_module(&self, _package: &str, _entities: &[&str]) -> Result<Option<File>, BackendError> {
        Ok(None)
    }
}

/// Backend errors
#[derive(Debug, thiserror::Error)]
pub enum BackendError {
    #[error("unknown backend: {0}")]
    UnknownBackend(String),

    #[error("code generation error: {0}")]
    CodeGenError(String),

    #[error("type mapping error: {0}")]
    TypeMappingError(String),
}

/// Get a backend by name
pub fn get_backend(name: &str) -> Result<Box<dyn Backend>, BackendError> {
    match name.to_lowercase().as_str() {
        "seaorm" | "sea_orm" | "sea-orm" => Ok(Box::new(seaorm::SeaOrmBackend)),
        // "ecto" => Ok(Box::new(ecto::EctoBackend)),
        // "gorm" => Ok(Box::new(gorm::GormBackend)),
        other => Err(BackendError::UnknownBackend(other.to_string())),
    }
}
