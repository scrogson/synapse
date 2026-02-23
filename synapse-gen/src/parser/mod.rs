//! Parser module: decodes raw CodeGeneratorRequest bytes, extracts Synapse
//! extensions via prost-reflect, and builds the Schema IR.

mod extensions;
mod ir_builder;

use prost::Message;
use prost_types::compiler::CodeGeneratorRequest;

use crate::generator::GeneratorError;
use crate::ir::Schema;
use extensions::ExtractedOptions;

/// Owns the decoded `CodeGeneratorRequest` and pre-extracted options.
///
/// The `schema()` method borrows from the owned request to build a
/// `Schema<'_>` with zero copies of the descriptor data.
pub struct ParsedSchema {
    request: CodeGeneratorRequest,
    options: ExtractedOptions,
}

impl ParsedSchema {
    /// Parse raw protoc plugin bytes into a `ParsedSchema`.
    ///
    /// This first extracts all Synapse custom extensions from the raw bytes
    /// using prost-reflect (since prost discards unknown extensions on decode),
    /// then decodes the request with prost for the standard fields.
    pub fn parse(bytes: &[u8]) -> Result<Self, GeneratorError> {
        let options =
            extensions::extract_options(bytes).map_err(|e| GeneratorError::Parse(e))?;
        let request = CodeGeneratorRequest::decode(bytes)
            .map_err(|e| GeneratorError::Parse(e.to_string()))?;
        Ok(Self { request, options })
    }

    /// Build the Schema IR, borrowing from the owned request.
    pub fn schema(&self) -> Schema<'_> {
        ir_builder::build_schema(&self.request, &self.options)
    }

    /// Access the raw decoded request.
    pub fn request(&self) -> &CodeGeneratorRequest {
        &self.request
    }
}
