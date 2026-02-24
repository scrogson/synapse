//! Code generation orchestration
//!
//! This module coordinates the overall code generation process,
//! iterating through proto files and generating SeaORM entities, enums, and storage traits.

use super::{options, package};
use crate::storage::seaorm::entity::EntityGenerator;
use crate::error::GeneratorError;
use crate::storage::seaorm::options::get_cached_entity_options;
use crate::{graphql, typescript};
use crate::validate::ValidateGenerator;
use crate::grpc::GrpcGenerator;
use crate::storage::seaorm::enum_gen::EnumGenerator;
use crate::storage::{StorageDefaultsGenerator, StorageTraitGenerator};
use crate::storage::seaorm::implementation::StorageImplGenerator;
use synapse_gen::{CodeGenerator, GeneratorContext, ParsedSchema};
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

        // Process each service in the file
        for svc in &file_descriptor.service {
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

        // Generate TypeScript type definitions
        if let Some(generated) = typescript::generate_types(file_descriptor, &request.proto_file)? {
            files.push(generated);
        }

        // Generate TypeScript resolver contracts
        if let Some(generated) = typescript::generate_resolvers(file_descriptor, &request.proto_file)? {
            files.push(generated);
        }

        // Generate TypeScript DataLoader interfaces
        if let Some(generated) = typescript::generate_dataloaders(file_descriptor, &request.proto_file)? {
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
/// It runs both the new synapse-gen-based generators (validate, enum, grpc, storage) and the legacy
/// generators (entity, graphql, etc.) and merges their outputs.
pub fn generate_from_bytes(bytes: &[u8]) -> Result<CodeGeneratorResponse, GeneratorError> {
    // Parse with synapse-gen for new generators (validate, enum, grpc)
    let parsed = ParsedSchema::parse(bytes)
        .map_err(|e| GeneratorError::DecodeError(e.to_string()))?;

    // Pre-process bytes to extract extension data for legacy generators
    options::preprocess_request_bytes(bytes).map_err(GeneratorError::DecodeError)?;

    // Decode with prost for legacy generators
    let request = CodeGeneratorRequest::decode(bytes)
        .map_err(|e| GeneratorError::DecodeError(e.to_string()))?;

    // Run new generators via synapse-gen IR
    let schema = parsed.schema();
    let validate_gen = ValidateGenerator;
    let mut new_gen_files = Vec::new();

    for package in &schema.packages {
        let ctx = GeneratorContext {
            schema: &schema,
            package,
        };
        for message in &package.messages {
            new_gen_files.extend(
                validate_gen
                    .generate_message(&ctx, message)
                    .map_err(|e| GeneratorError::CodeGenError(e.to_string()))?,
            );
        }
        for entity in &package.entities {
            new_gen_files.extend(
                validate_gen
                    .generate_entity(&ctx, entity)
                    .map_err(|e| GeneratorError::CodeGenError(e.to_string()))?,
            );
        }
    }

    // Run enum generator via synapse-gen IR
    let enum_gen = EnumGenerator;
    for package in &schema.packages {
        let ctx = GeneratorContext {
            schema: &schema,
            package,
        };
        for enum_ir in &package.enums {
            new_gen_files.extend(
                enum_gen
                    .generate_enum(&ctx, enum_ir)
                    .map_err(|e| GeneratorError::CodeGenError(e.to_string()))?,
            );
        }
    }

    // Run gRPC generator via synapse-gen IR
    let grpc_gen = GrpcGenerator;
    for package in &schema.packages {
        let ctx = GeneratorContext {
            schema: &schema,
            package,
        };
        for service in &package.services {
            new_gen_files.extend(
                grpc_gen
                    .generate_service(&ctx, service)
                    .map_err(|e| GeneratorError::CodeGenError(e.to_string()))?,
            );
        }
    }

    // Run storage generators via synapse-gen IR
    let trait_gen = StorageTraitGenerator;
    let defaults_gen = StorageDefaultsGenerator;
    let impl_gen = StorageImplGenerator;
    for package in &schema.packages {
        let ctx = GeneratorContext {
            schema: &schema,
            package,
        };
        for service in &package.services {
            new_gen_files.extend(
                trait_gen
                    .generate_service(&ctx, service)
                    .map_err(|e| GeneratorError::CodeGenError(e.to_string()))?,
            );
            new_gen_files.extend(
                defaults_gen
                    .generate_service(&ctx, service)
                    .map_err(|e| GeneratorError::CodeGenError(e.to_string()))?,
            );
            new_gen_files.extend(
                impl_gen
                    .generate_service(&ctx, service)
                    .map_err(|e| GeneratorError::CodeGenError(e.to_string()))?,
            );
        }
    }

    // Run entity generator via synapse-gen IR
    let entity_gen = EntityGenerator;
    for package in &schema.packages {
        let ctx = GeneratorContext {
            schema: &schema,
            package,
        };
        for entity_ir in &package.entities {
            new_gen_files.extend(
                entity_gen
                    .generate_entity(&ctx, entity_ir)
                    .map_err(|e| GeneratorError::CodeGenError(e.to_string()))?,
            );
        }
    }

    // Run legacy generators
    let mut response = generate(request)?;

    // Merge new generator files into the response.
    // Note: No collision detection between new and legacy generators here.
    // This will be resolved when all generators migrate to SynapseGenerator.
    for f in new_gen_files {
        response.file.push(prost_types::compiler::code_generator_response::File {
            name: Some(f.path),
            content: Some(f.content),
            ..Default::default()
        });
    }

    Ok(response)
}
