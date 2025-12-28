//! Code generation orchestration
//!
//! This module coordinates the overall code generation process,
//! iterating through proto files and generating SeaORM entities, enums, and storage traits.

use super::{entity, enum_gen, implementation, options, package};
use crate::error::GeneratorError;
use crate::storage::seaorm::options::get_cached_entity_options;
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

        // Collect entities (messages with synapse.storage.entity option)
        // Only collect entities from files in the SAME package to avoid duplication
        let main_package = file_descriptor.package.as_deref().unwrap_or("");
        let mut entities: Vec<&prost_types::DescriptorProto> = Vec::new();
        let mut entity_file_map: Vec<(&prost_types::FileDescriptorProto, &prost_types::DescriptorProto)> = Vec::new();

        for proto_file in &request.proto_file {
            // Only process files from the same package
            let file_package = proto_file.package.as_deref().unwrap_or("");
            if file_package != main_package {
                continue;
            }

            let proto_file_name = proto_file.name.as_deref().unwrap_or("");
            for message in &proto_file.message_type {
                let msg_name = message.name.as_deref().unwrap_or("");
                if get_cached_entity_options(proto_file_name, msg_name).is_some() {
                    entities.push(message);
                    entity_file_map.push((proto_file, message));
                }
            }
        }

        // Generate code for entities found in imports
        for (proto_file, message) in &entity_file_map {
            // Generate entity if has entity options
            if let Some(generated) = entity::generate(proto_file, message)? {
                files.push(generated);
            }
            // Generate domain type if has validate options with generate_conversion
            if let Some(generated) = validate::generate(proto_file, message)? {
                files.push(generated);
            }
            // Generate GraphQL Object type if has graphql options
            if let Some(generated) = graphql::generate_message(proto_file, message)? {
                files.push(generated);
            }
            // Generate DataLoaders for relations
            for generated in graphql::generate_dataloaders(proto_file, message, &request.proto_file)? {
                files.push(generated);
            }
            // Generate entity loader for BelongsTo relations
            if let Some(generated) = graphql::generate_entity_loader(proto_file, message)? {
                files.push(generated);
            }
        }

        // Also process non-entity messages in the main file (request/response types)
        let file_name = file_descriptor.name.as_deref().unwrap_or("");
        for message in &file_descriptor.message_type {
            let msg_name = message.name.as_deref().unwrap_or("");
            // Skip if already processed as entity
            if get_cached_entity_options(file_name, msg_name).is_some() {
                continue;
            }
            // Generate GraphQL input types for request messages
            if let Some(generated) = graphql::generate_message(file_descriptor, message)? {
                files.push(generated);
            }
        }

        // Generate auto-generated filter types for entities
        if !entities.is_empty() {
            let entity_refs: Vec<_> = entities.iter().map(|e| *e).collect();
            for generated in graphql::generate_filters(file_descriptor, &entity_refs, &request.proto_file)? {
                files.push(generated);
            }
            for generated in graphql::generate_connections(file_descriptor, &entity_refs)? {
                files.push(generated);
            }
        }

        // Generate Node interface if there are node types in this file
        if let Some(generated) = graphql::generate_node_interface(file_descriptor)? {
            files.push(generated);
        }

        // Generate unified GraphQL schema (mod.rs with Query/Mutation/schema builder)
        if let Some(generated) = graphql::generate_schema(file_descriptor, &request.proto_file)? {
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
            if let Some(generated) = implementation::generate(file_descriptor, svc, &request.proto_file)? {
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
            // GraphQL input types (auto-generated from request messages)
            for generated in graphql::generate_inputs(file_descriptor, svc)? {
                files.push(generated);
            }
        }

        // Generate package mod.rs and subdirectory mod.rs files
        for generated in package::generate_all(file_descriptor, &request.proto_file)? {
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
