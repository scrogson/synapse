use crate::ir::*;

/// A generated file with a path and content string.
#[derive(Debug, Clone)]
pub struct GeneratedFile {
    pub path: String,
    pub content: String,
}

/// Context passed to every generator callback.
pub struct GeneratorContext<'a> {
    pub schema: &'a Schema<'a>,
    pub package: &'a Package<'a>,
}

/// Errors that generators can produce.
#[derive(Debug, thiserror::Error)]
pub enum GeneratorError {
    #[error("missing required option '{option}' on {entity}")]
    MissingOption { entity: String, option: String },

    #[error("invalid option '{option}' on {entity}: {message}")]
    InvalidOption {
        entity: String,
        option: String,
        message: String,
    },

    #[error("file path collision: '{path}' produced by generators '{first}' and '{second}'")]
    FileCollision {
        path: String,
        first: String,
        second: String,
    },

    #[error("parse error: {0}")]
    Parse(String),

    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

/// The trait third-party developers implement to generate code
/// from Synapse-annotated protobuf definitions.
///
/// All methods default to no-op (empty Vec), so generators only
/// implement the callbacks they care about.
pub trait CodeGenerator: Send + Sync {
    /// Human-readable name, used in error messages and logging.
    fn name(&self) -> &str;

    /// Called once per entity (message with synapse.storage.entity).
    fn generate_entity(
        &self,
        _ctx: &GeneratorContext,
        _entity: &Entity,
    ) -> Result<Vec<GeneratedFile>, GeneratorError> {
        Ok(vec![])
    }

    /// Called once per service.
    fn generate_service(
        &self,
        _ctx: &GeneratorContext,
        _service: &Service,
    ) -> Result<Vec<GeneratedFile>, GeneratorError> {
        Ok(vec![])
    }

    /// Called once per enum.
    fn generate_enum(
        &self,
        _ctx: &GeneratorContext,
        _enum: &Enum,
    ) -> Result<Vec<GeneratedFile>, GeneratorError> {
        Ok(vec![])
    }

    /// Called once per non-entity message (request/response types).
    fn generate_message(
        &self,
        _ctx: &GeneratorContext,
        _message: &Message,
    ) -> Result<Vec<GeneratedFile>, GeneratorError> {
        Ok(vec![])
    }

    /// Called once per package after all entities/services/enums/messages.
    /// Use for module files, package-level rollups, index files.
    fn finalize_package(
        &self,
        _ctx: &GeneratorContext,
    ) -> Result<Vec<GeneratedFile>, GeneratorError> {
        Ok(vec![])
    }

    /// Called once after all packages. Use for top-level files.
    fn finalize(&self, _schema: &Schema) -> Result<Vec<GeneratedFile>, GeneratorError> {
        Ok(vec![])
    }
}
