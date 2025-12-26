//! DataLoader generation
//!
//! Generates async-graphql DataLoaders for efficient batched data fetching.
//! DataLoaders prevent N+1 queries by batching multiple lookups into single requests.
//!
//! Two types of loaders are generated:
//! 1. ID Loaders (for BelongsTo): Load entities by their primary key
//! 2. Relation Loaders (for HasMany): Load related entities by foreign key

use crate::error::GeneratorError;
use crate::storage::seaorm::options::{
    get_cached_entity_options, get_cached_graphql_message_options,
};
use heck::{ToSnakeCase, ToUpperCamelCase};
use prost_types::compiler::code_generator_response::File;
use prost_types::{DescriptorProto, FileDescriptorProto};
use quote::{format_ident, quote};

/// Generate DataLoaders for a message type based on its relations
pub fn generate(
    file: &FileDescriptorProto,
    message: &DescriptorProto,
) -> Result<Vec<File>, GeneratorError> {
    let file_name = file.name.as_deref().unwrap_or("");
    let msg_name = message.name.as_deref().unwrap_or("");

    // Check for graphql message options
    let msg_opts = get_cached_graphql_message_options(file_name, msg_name);

    // Skip if no graphql options or explicitly skipped
    if msg_opts.as_ref().is_some_and(|o| o.skip) {
        return Ok(vec![]);
    }

    // Get entity options to find relations
    let entity_opts = get_cached_entity_options(file_name, msg_name);

    // If no entity options or no relations, skip
    if entity_opts.is_none() {
        return Ok(vec![]);
    }

    let entity = entity_opts.unwrap();
    if entity.relations.is_empty() {
        return Ok(vec![]);
    }

    // Determine type name
    let type_name = msg_opts
        .as_ref()
        .filter(|o| !o.type_name.is_empty())
        .map(|o| o.type_name.clone())
        .unwrap_or_else(|| msg_name.to_upper_camel_case());

    let mut loaders = Vec::new();

    // Generate a loader for each relation
    for relation in &entity.relations {
        if let Some(loader) = generate_relation_loader(file, &type_name, relation)? {
            loaders.push(loader);
        }
    }

    Ok(loaders)
}

/// Generate a DataLoader for a specific relation (HasMany)
///
/// Uses the List RPC with an IN filter on the foreign key for true batch loading.
fn generate_relation_loader(
    file: &FileDescriptorProto,
    parent_type: &str,
    relation: &crate::options::synapse::storage::RelationDef,
) -> Result<Option<File>, GeneratorError> {
    let related_type = &relation.related;
    let foreign_key = &relation.foreign_key;

    let relation_type = relation.r#type();

    // Only generate for HasMany/ManyToMany relations
    let is_many = matches!(
        relation_type,
        crate::options::synapse::storage::RelationType::HasMany
            | crate::options::synapse::storage::RelationType::ManyToMany
    );

    if !is_many {
        return Ok(None);
    }

    // Generate loader name (e.g., PostsByUserLoader)
    let loader_name = format!(
        "{}sBy{}Loader",
        related_type.to_upper_camel_case(),
        parent_type.to_upper_camel_case()
    );
    let loader_ident = format_ident!("{}", loader_name);

    // Related type ident
    let related_ident = format_ident!("{}", related_type.to_upper_camel_case());

    // Foreign key ident for grouping results
    let fk_ident = format_ident!("{}", foreign_key.to_snake_case());

    // Service client type
    let service_name = format!("{}Service", related_type.to_upper_camel_case());
    let client_module = format!("{}_client", service_name.to_snake_case());
    let client_module_ident = format_ident!("{}", client_module);
    let client_ident = format_ident!("{}Client", service_name);

    // List request and filter type names
    let list_request = format_ident!("List{}sRequest", related_type.to_upper_camel_case());
    let list_method = format_ident!("list_{}s", related_type.to_snake_case());
    let filter_type = format_ident!("{}Filter", related_type.to_upper_camel_case());

    let code = quote! {
        //! DataLoader for HasMany relation
        //! @generated

        #![allow(missing_docs)]
        #![allow(unused_imports)]

        use async_graphql::dataloader::Loader;
        use std::collections::HashMap;
        use tonic::transport::Channel;
        // Import gRPC client and types from parent module
        use super::super::#client_module_ident::#client_ident;
        use super::super::#list_request;
        use super::super::#filter_type;
        use super::super::super::synapse::relay::IntFilter;

        /// DataLoader for fetching #related_type by #parent_type ID (HasMany)
        ///
        /// Uses List RPC with IN filter on foreign key for true batch loading.
        pub struct #loader_ident {
            client: #client_ident<Channel>,
        }

        impl #loader_ident {
            /// Create a new loader with the given gRPC client
            pub fn new(client: #client_ident<Channel>) -> Self {
                Self { client }
            }
        }

        impl Loader<i64> for #loader_ident {
            type Value = Vec<super::#related_ident>;
            type Error = async_graphql::Error;

            async fn load(
                &self,
                keys: &[i64],
            ) -> Result<HashMap<i64, Self::Value>, Self::Error> {
                if keys.is_empty() {
                    return Ok(HashMap::new());
                }

                // Build filter with IN clause on foreign key
                let filter = #filter_type {
                    #fk_ident: Some(IntFilter {
                        r#in: keys.to_vec(),
                        ..Default::default()
                    }),
                    ..Default::default()
                };

                // Single List RPC call with IN filter
                // Use a high limit to get all related items
                let request = #list_request {
                    filter: Some(filter),
                    first: Some(1000), // High limit for batch loading
                    ..Default::default()
                };

                let response = self.client
                    .clone()
                    .#list_method(request)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.message()))?;

                // Group results by foreign key
                let mut map: HashMap<i64, Vec<super::#related_ident>> = HashMap::new();

                // Initialize empty vecs for all requested keys
                for &key in keys {
                    map.insert(key, Vec::new());
                }

                // Populate from response
                for edge in response.into_inner().edges {
                    if let Some(node) = edge.node {
                        let entity = super::#related_ident::from(node);
                        let key = entity.#fk_ident;
                        if let Some(vec) = map.get_mut(&key) {
                            vec.push(entity);
                        }
                    }
                }

                Ok(map)
            }
        }
    };

    // Format the generated code
    let content = code.to_string();
    let formatted = match syn::parse_file(&content) {
        Ok(parsed) => prettyplease::unparse(&parsed),
        Err(_) => content,
    };

    // Determine output file path
    let package = file.package.as_deref().unwrap_or("");
    let output_path = format!(
        "{}/graphql/{}.rs",
        package.replace('.', "/"),
        loader_name.to_snake_case()
    );

    Ok(Some(File {
        name: Some(output_path),
        content: Some(formatted),
        ..Default::default()
    }))
}

/// Generate an ID-based loader file for fetching entities by their primary key
///
/// Uses the List RPC with an IN filter for true batch loading (single query).
/// It's used for BelongsTo relations (e.g., Post.author uses UserLoader).
pub fn generate_entity_loader(
    file: &FileDescriptorProto,
    message: &DescriptorProto,
) -> Result<Option<File>, GeneratorError> {
    let file_name = file.name.as_deref().unwrap_or("");
    let msg_name = message.name.as_deref().unwrap_or("");

    // Check for graphql message options
    let msg_opts = get_cached_graphql_message_options(file_name, msg_name);

    // Skip if no graphql options or explicitly skipped
    if msg_opts.as_ref().is_some_and(|o| o.skip) {
        return Ok(None);
    }

    // Get entity options - only generate loaders for entities
    let entity_opts = get_cached_entity_options(file_name, msg_name);
    if entity_opts.is_none() {
        return Ok(None);
    }

    // Determine type name
    let type_name = msg_opts
        .as_ref()
        .filter(|o| !o.type_name.is_empty())
        .map(|o| o.type_name.clone())
        .unwrap_or_else(|| msg_name.to_upper_camel_case());

    let loader_name = format!("{}Loader", type_name);
    let loader_ident = format_ident!("{}", loader_name);
    let type_ident = format_ident!("{}", type_name);

    // Service and client names
    let service_name = format!("{}Service", type_name);
    let client_module = format!("{}_client", service_name.to_snake_case());
    let client_module_ident = format_ident!("{}", client_module);
    let client_ident = format_ident!("{}Client", service_name);

    // List request and filter type names
    let list_request = format_ident!("List{}sRequest", type_name);
    let list_method = format_ident!("list_{}", format!("{}s", type_name.to_snake_case()));
    let filter_type = format_ident!("{}Filter", type_name);

    let code = quote! {
        //! DataLoader for #type_name entities
        //! @generated

        #![allow(missing_docs)]
        #![allow(unused_imports)]

        use async_graphql::dataloader::Loader;
        use std::collections::HashMap;
        use tonic::transport::Channel;
        // Import gRPC client and types from parent module
        use super::super::#client_module_ident::#client_ident;
        use super::super::#list_request;
        use super::super::#filter_type;
        use super::super::super::synapse::relay::IntFilter;

        /// DataLoader for fetching #type_name entities by ID
        ///
        /// Uses List RPC with IN filter for true batch loading (single query).
        pub struct #loader_ident {
            client: #client_ident<Channel>,
        }

        impl #loader_ident {
            /// Create a new loader with the given gRPC client
            pub fn new(client: #client_ident<Channel>) -> Self {
                Self { client }
            }
        }

        impl Loader<i64> for #loader_ident {
            type Value = super::#type_ident;
            type Error = async_graphql::Error;

            async fn load(
                &self,
                keys: &[i64],
            ) -> Result<HashMap<i64, Self::Value>, Self::Error> {
                if keys.is_empty() {
                    return Ok(HashMap::new());
                }

                // Build filter with IN clause for batch loading
                let filter = #filter_type {
                    id: Some(IntFilter {
                        r#in: keys.to_vec(),
                        ..Default::default()
                    }),
                    ..Default::default()
                };

                // Single List RPC call with IN filter
                let request = #list_request {
                    filter: Some(filter),
                    first: Some(keys.len() as i32),
                    ..Default::default()
                };

                let response = self.client
                    .clone()
                    .#list_method(request)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.message()))?;

                // Map results by ID from connection edges
                let mut map: HashMap<i64, Self::Value> = HashMap::new();
                for edge in response.into_inner().edges {
                    if let Some(node) = edge.node {
                        let entity = super::#type_ident::from(node);
                        // Use internal id field for mapping
                        map.insert(entity.id, entity);
                    }
                }

                Ok(map)
            }
        }
    };

    // Format the generated code
    let content = code.to_string();
    let formatted = match syn::parse_file(&content) {
        Ok(parsed) => prettyplease::unparse(&parsed),
        Err(_) => content,
    };

    // Determine output file path
    let package = file.package.as_deref().unwrap_or("");
    let output_path = format!(
        "{}/graphql/{}_loader.rs",
        package.replace('.', "/"),
        type_name.to_snake_case()
    );

    Ok(Some(File {
        name: Some(output_path),
        content: Some(formatted),
        ..Default::default()
    }))
}
