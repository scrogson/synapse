//! Code generation orchestration
//!
//! This module coordinates the overall code generation process,
//! iterating through proto files and generating SeaORM entities, enums, and storage traits.

use super::{entity, enum_gen, implementation, options, package};
use crate::error::GeneratorError;
use crate::{graphql, grpc, validate};
use prost::Message;
use prost_types::compiler::{CodeGeneratorRequest, CodeGeneratorResponse};

/// Generate SeaORM entities and enums from a CodeGeneratorRequest
pub fn generate(request: CodeGeneratorRequest) -> Result<CodeGeneratorResponse, GeneratorError> {
    let mut files = Vec::new();

    // Process each file that was requested for generation
    for file_name in &request.file_to_generate {
        // Find the corresponding FileDescriptorProto
        let file_descriptor = request
            .proto_file
            .iter()
            .find(|f| f.name.as_ref() == Some(file_name))
            .ok_or_else(|| {
                GeneratorError::CodeGenError(format!("File descriptor not found: {}", file_name))
            })?;

        // Process each message in the file
        for message in &file_descriptor.message_type {
            // Generate entity if has entity options
            if let Some(generated) = entity::generate(file_descriptor, message)? {
                files.push(generated);
            }
            // Generate domain type if has validate options with generate_conversion
            if let Some(generated) = validate::generate(file_descriptor, message)? {
                files.push(generated);
            }
            // Generate GraphQL Object type if has graphql options
            if let Some(generated) = graphql::generate_message(file_descriptor, message)? {
                files.push(generated);
            }
            // Generate DataLoaders for relations
            for generated in graphql::generate_dataloaders(file_descriptor, message)? {
                files.push(generated);
            }
        }

        // Generate Node interface if there are node types in this file
        if let Some(generated) = graphql::generate_node_interface(file_descriptor)? {
            files.push(generated);
        }

        // Generate unified GraphQL schema (mod.rs with Query/Mutation/schema builder)
        if let Some(generated) = graphql::generate_schema(file_descriptor)? {
            files.push(generated);
        }

        // Process each enum in the file
        for enum_desc in &file_descriptor.enum_type {
            if let Some(generated) = enum_gen::generate(file_descriptor, enum_desc)? {
                files.push(generated);
            }
        }

        // Process each service in the file
        for svc in &file_descriptor.service {
            // Storage trait generation
            if let Some(generated) = crate::storage::generate(file_descriptor, svc)? {
                files.push(generated);
            }
            // Storage implementation generation (SeaORM-based)
            if let Some(generated) = implementation::generate(file_descriptor, svc)? {
                files.push(generated);
            }
            // gRPC service generation
            if let Some(generated) = grpc::generate(file_descriptor, svc)? {
                files.push(generated);
            }
            // GraphQL resolver generation (Query/Mutation structs)
            for generated in graphql::generate_service(file_descriptor, svc)? {
                files.push(generated);
            }
        }

        // Generate package mod.rs
        if let Some(generated) = package::generate(file_descriptor)? {
            files.push(generated);
        }

        // Generate conversions.rs
        if let Some(generated) = package::generate_conversions(file_descriptor)? {
            files.push(generated);
        }
    }

    Ok(CodeGeneratorResponse {
        file: files,
        error: None,
        supported_features: Some(1), // FEATURE_PROTO3_OPTIONAL
    })
}

/// Generate SeaORM entities from raw protobuf bytes
///
/// This entry point preserves extension data by using prost-reflect for decoding.
pub fn generate_from_bytes(bytes: &[u8]) -> Result<CodeGeneratorResponse, GeneratorError> {
    // Pre-process bytes to extract extension data using prost-reflect
    options::preprocess_request_bytes(bytes).map_err(GeneratorError::DecodeError)?;

    // Now decode with prost (extension data is cached)
    let request = CodeGeneratorRequest::decode(bytes)
        .map_err(|e| GeneratorError::DecodeError(e.to_string()))?;

    // Generate using the regular path (which will use cached options)
    generate(request)
}
