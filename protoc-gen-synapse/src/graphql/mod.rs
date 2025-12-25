//! GraphQL code generation with async-graphql
//!
//! This module generates GraphQL schema code from protobuf definitions.
//! It creates:
//! - Object types from proto messages
//! - Query/Mutation resolvers from services
//! - DataLoader integration for N+1 prevention
//! - Relay Node interface and connections

mod connection;
mod dataloader;
mod node;
mod object;
mod resolver;

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
