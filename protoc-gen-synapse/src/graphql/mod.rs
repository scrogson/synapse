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

use synapse_gen::ir::{Entity, Message, Service};
use synapse_gen::{CodeGenerator, GeneratedFile, GeneratorContext, GeneratorError};

pub struct GraphQLGenerator;

impl CodeGenerator for GraphQLGenerator {
    fn name(&self) -> &str {
        "graphql"
    }

    fn generate_entity(
        &self,
        ctx: &GeneratorContext,
        entity: &Entity,
    ) -> Result<Vec<GeneratedFile>, GeneratorError> {
        let mut files = Vec::new();

        // Object type
        files.extend(object::generate_for_entity(&ctx.package.name, entity)?);

        // DataLoaders for relations
        let all_entities: Vec<&Entity> = ctx.package.entities.iter().collect();
        files.extend(dataloader::generate(&ctx.package.name, entity, &all_entities)?);

        // Entity loader for BelongsTo relations
        if let Some(f) = dataloader::generate_entity_loader(&ctx.package.name, entity)? {
            files.push(f);
        }

        Ok(files)
    }

    fn generate_message(
        &self,
        ctx: &GeneratorContext,
        message: &Message,
    ) -> Result<Vec<GeneratedFile>, GeneratorError> {
        // Object/Input types for non-entity messages
        let mut files = Vec::new();
        if let Some(f) = object::generate_for_message(&ctx.package.name, message)? {
            files.push(f);
        }
        Ok(files)
    }

    fn generate_service(
        &self,
        ctx: &GeneratorContext,
        service: &Service,
    ) -> Result<Vec<GeneratedFile>, GeneratorError> {
        let mut files = Vec::new();

        // Query/Mutation resolvers
        files.extend(resolver::generate(ctx, service)?);

        // Auto-generated inputs
        files.extend(input::generate_inputs_for_service(ctx, service)?);

        Ok(files)
    }

    fn finalize_package(
        &self,
        ctx: &GeneratorContext,
    ) -> Result<Vec<GeneratedFile>, GeneratorError> {
        let mut files = Vec::new();
        let entities: Vec<&Entity> = ctx.package.entities.iter().collect();

        // Filters and connections
        if !entities.is_empty() {
            files.extend(filter::generate_filters_for_package(ctx, &entities)?);
            files.extend(connection::generate_connections_for_package(
                &ctx.package.name,
                &entities,
            )?);
        }

        // Node interface
        let node_types = node::collect_node_types(&entities);
        if let Some(f) = node::generate_node_interface(&ctx.package.name, &node_types)? {
            files.push(f);
        }

        // Unified schema
        if let Some(f) = schema::generate(ctx)? {
            files.push(f);
        }

        Ok(files)
    }
}
