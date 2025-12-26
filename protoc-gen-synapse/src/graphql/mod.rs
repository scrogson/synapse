//! GraphQL code generation with async-graphql
//!
//! This module generates GraphQL schema code from protobuf definitions.
//! It creates:
//! - Object types from proto messages
//! - Query/Mutation resolvers from services
//! - DataLoader integration for N+1 prevention
//! - Relay Node interface and connections
//! - Combined schema with Query/Mutation/Subscription
//! - Auto-generated filter types (IntFilter, StringFilter, etc.)
//! - Auto-generated connection types (PageInfo, Edge, Connection)

mod connection;
mod dataloader;
mod filter;
mod input;
mod node;
mod object;
mod resolver;
mod schema;

use crate::error::GeneratorError;
use prost_types::compiler::code_generator_response::File;
use prost_types::{DescriptorProto, FileDescriptorProto, ServiceDescriptorProto};

/// Generate GraphQL Object types from a message
#[allow(dead_code)]
pub fn generate_message(
    file: &FileDescriptorProto,
    message: &DescriptorProto,
) -> Result<Option<File>, GeneratorError> {
    object::generate(file, message)
}

/// Generate GraphQL resolvers from a service
#[allow(dead_code)]
pub fn generate_service(
    file: &FileDescriptorProto,
    service: &ServiceDescriptorProto,
) -> Result<Vec<File>, GeneratorError> {
    resolver::generate(file, service)
}

/// Generate the Relay Node interface for a file
#[allow(dead_code)]
pub fn generate_node_interface(file: &FileDescriptorProto) -> Result<Option<File>, GeneratorError> {
    let node_types = node::collect_node_types(file);
    node::generate_node_interface(file, &node_types)
}

/// Generate DataLoaders for a message (based on its relations)
#[allow(dead_code)]
pub fn generate_dataloaders(
    file: &FileDescriptorProto,
    message: &DescriptorProto,
) -> Result<Vec<File>, GeneratorError> {
    dataloader::generate(file, message)
}

/// Generate the unified GraphQL schema mod.rs for a file
///
/// This creates the graphql/mod.rs that:
/// - Imports all generated sub-modules (entities, inputs, filters)
/// - Defines combined Query/Mutation using MergedObject
/// - Defines Connection types (PageInfo, Edge, Connection)
/// - Provides schema builder function
#[allow(dead_code)]
pub fn generate_schema(
    file: &FileDescriptorProto,
    all_files: &[FileDescriptorProto],
) -> Result<Option<File>, GeneratorError> {
    schema::generate(file, all_files)
}

/// Generate auto-generated filter types for entities in a package
///
/// Creates:
/// - Primitive filters (IntFilter, StringFilter, BoolFilter)
/// - Entity-specific filters (UserFilter, PostFilter)
/// - Entity-specific order by types (UserOrderBy, PostOrderBy)
/// - OrderDirection enum
pub fn generate_filters(
    file: &FileDescriptorProto,
    entities: &[&DescriptorProto],
    all_files: &[FileDescriptorProto],
) -> Result<Vec<File>, GeneratorError> {
    filter::generate_filters_for_package(file, entities, all_files)
}

/// Generate auto-generated Relay connection types for entities in a package
///
/// Creates:
/// - PageInfo type
/// - Entity Edge types (UserEdge, PostEdge)
/// - Entity Connection types (UserConnection, PostConnection)
pub fn generate_connections(
    file: &FileDescriptorProto,
    entities: &[&DescriptorProto],
) -> Result<Vec<File>, GeneratorError> {
    connection::generate_connections_for_package(file, entities)
}

/// Generate auto-generated input types from mutation request messages
///
/// Creates GraphQL InputObject types from request messages:
/// - CreateUserRequest → CreateUserInput (all fields)
/// - UpdateUserRequest → UpdateUserInput (all fields except id)
pub fn generate_inputs(
    file: &FileDescriptorProto,
    service: &ServiceDescriptorProto,
) -> Result<Vec<File>, GeneratorError> {
    input::generate_inputs_for_service(file, service)
}
