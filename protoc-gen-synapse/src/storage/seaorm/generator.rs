//! Code generation orchestration
//!
//! This module coordinates the overall code generation process,
//! iterating through proto files and generating SeaORM entities, enums, and storage traits.

use crate::graphql::GraphQLGenerator;
use crate::storage::seaorm::package::PackageGenerator;
use crate::storage::seaorm::entity::EntityGenerator;
use crate::error::GeneratorError;
use crate::typescript::TypeScriptGenerator;
use crate::validate::ValidateGenerator;
use crate::grpc::GrpcGenerator;
use crate::storage::seaorm::enum_gen::EnumGenerator;
use crate::storage::{StorageDefaultsGenerator, StorageTraitGenerator};
use crate::storage::seaorm::implementation::StorageImplGenerator;
use synapse_gen::{CodeGenerator, GeneratorContext, ParsedSchema};
use prost_types::compiler::CodeGeneratorResponse;

/// Generate SeaORM entities from raw protobuf bytes
///
/// Parses protobuf bytes with synapse-gen and runs all generators via IR types.
pub fn generate_from_bytes(bytes: &[u8]) -> Result<CodeGeneratorResponse, GeneratorError> {
    let parsed = ParsedSchema::parse(bytes)
        .map_err(|e| GeneratorError::DecodeError(e.to_string()))?;

    // Run all generators via synapse-gen IR
    let schema = parsed.schema();
    let validate_gen = ValidateGenerator;
    let enum_gen = EnumGenerator;
    let grpc_gen = GrpcGenerator;
    let trait_gen = StorageTraitGenerator;
    let defaults_gen = StorageDefaultsGenerator;
    let impl_gen = StorageImplGenerator;
    let entity_gen = EntityGenerator;
    let package_gen = PackageGenerator;
    let ts_gen = TypeScriptGenerator;
    let graphql_gen = GraphQLGenerator;

    let mut files = Vec::new();

    for package in &schema.packages {
        let ctx = GeneratorContext {
            schema: &schema,
            package,
        };

        // Validate generator
        for message in &package.messages {
            files.extend(
                validate_gen
                    .generate_message(&ctx, message)
                    .map_err(|e| GeneratorError::CodeGenError(e.to_string()))?,
            );
        }
        for entity in &package.entities {
            files.extend(
                validate_gen
                    .generate_entity(&ctx, entity)
                    .map_err(|e| GeneratorError::CodeGenError(e.to_string()))?,
            );
        }

        // Enum generator
        for enum_ir in &package.enums {
            files.extend(
                enum_gen
                    .generate_enum(&ctx, enum_ir)
                    .map_err(|e| GeneratorError::CodeGenError(e.to_string()))?,
            );
        }

        // gRPC generator
        for service in &package.services {
            files.extend(
                grpc_gen
                    .generate_service(&ctx, service)
                    .map_err(|e| GeneratorError::CodeGenError(e.to_string()))?,
            );
        }

        // Storage generators (trait, defaults, impl)
        for service in &package.services {
            files.extend(
                trait_gen
                    .generate_service(&ctx, service)
                    .map_err(|e| GeneratorError::CodeGenError(e.to_string()))?,
            );
            files.extend(
                defaults_gen
                    .generate_service(&ctx, service)
                    .map_err(|e| GeneratorError::CodeGenError(e.to_string()))?,
            );
            files.extend(
                impl_gen
                    .generate_service(&ctx, service)
                    .map_err(|e| GeneratorError::CodeGenError(e.to_string()))?,
            );
        }

        // Entity generator
        for entity_ir in &package.entities {
            files.extend(
                entity_gen
                    .generate_entity(&ctx, entity_ir)
                    .map_err(|e| GeneratorError::CodeGenError(e.to_string()))?,
            );
        }

        // GraphQL generator
        for entity in &package.entities {
            files.extend(
                graphql_gen
                    .generate_entity(&ctx, entity)
                    .map_err(|e| GeneratorError::CodeGenError(e.to_string()))?,
            );
        }
        for message in &package.messages {
            files.extend(
                graphql_gen
                    .generate_message(&ctx, message)
                    .map_err(|e| GeneratorError::CodeGenError(e.to_string()))?,
            );
        }
        for service in &package.services {
            files.extend(
                graphql_gen
                    .generate_service(&ctx, service)
                    .map_err(|e| GeneratorError::CodeGenError(e.to_string()))?,
            );
        }
        files.extend(
            graphql_gen
                .finalize_package(&ctx)
                .map_err(|e| GeneratorError::CodeGenError(e.to_string()))?,
        );

        // Package + TypeScript generators
        files.extend(
            package_gen
                .finalize_package(&ctx)
                .map_err(|e| GeneratorError::CodeGenError(e.to_string()))?,
        );
        files.extend(
            ts_gen
                .finalize_package(&ctx)
                .map_err(|e| GeneratorError::CodeGenError(e.to_string()))?,
        );
    }

    Ok(CodeGeneratorResponse {
        file: files
            .into_iter()
            .map(|f| prost_types::compiler::code_generator_response::File {
                name: Some(f.path),
                content: Some(f.content),
                ..Default::default()
            })
            .collect(),
        error: None,
        supported_features: Some(1), // FEATURE_PROTO3_OPTIONAL
    })
}
