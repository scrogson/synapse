//! DataLoader generation
//!
//! Generates async-graphql DataLoaders for efficient batched data fetching.
//! DataLoaders prevent N+1 queries by batching multiple lookups into single requests.

use crate::error::GeneratorError;
use crate::storage::seaorm::options::{
    get_cached_entity_options, get_cached_graphql_message_options,
};
use heck::{ToSnakeCase, ToUpperCamelCase};
use proc_macro2::TokenStream;
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

/// Generate a DataLoader for a specific relation
fn generate_relation_loader(
    file: &FileDescriptorProto,
    parent_type: &str,
    relation: &crate::options::synapse::storage::RelationDef,
) -> Result<Option<File>, GeneratorError> {
    let _relation_name = &relation.name;
    let related_type = &relation.related;
    let foreign_key = &relation.foreign_key;

    // Generate loader name (e.g., PostsByUserLoader)
    let loader_name = format!(
        "{}By{}Loader",
        related_type.to_upper_camel_case(),
        parent_type.to_upper_camel_case()
    );
    let loader_ident = format_ident!("{}", loader_name);

    // Related type ident
    let related_ident = format_ident!("{}", related_type.to_upper_camel_case());

    // Foreign key ident
    let fk_ident = format_ident!("{}", foreign_key.to_snake_case());

    // Generate RPC method name for batch loading
    // Convention: list_{related_plural}_by_{parent}_ids
    let rpc_method_name = format!(
        "list_{}_by_{}_ids",
        related_type.to_snake_case(),
        parent_type.to_snake_case()
    );
    let rpc_method_ident = format_ident!("{}", rpc_method_name);

    // Service client type (assume related type has a corresponding service)
    let service_name = format!("{}Service", related_type.to_upper_camel_case());
    let client_ident = format_ident!("{}Client", service_name);

    let relation_type = relation.r#type();
    let is_many = matches!(
        relation_type,
        crate::options::synapse::storage::RelationType::HasMany
            | crate::options::synapse::storage::RelationType::ManyToMany
    );

    let value_type = if is_many {
        quote! { Vec<#related_ident> }
    } else {
        quote! { #related_ident }
    };

    // Generate the loop body based on whether it's a many or single relation
    let loop_body = if is_many {
        quote! {
            map.entry(key)
                .or_insert_with(Vec::new)
                .push(#related_ident::from(item));
        }
    } else {
        quote! {
            map.insert(key, #related_ident::from(item));
        }
    };

    let code = quote! {
        //! DataLoader for #relation_name relation
        //! @generated

        #![allow(missing_docs)]
        #![allow(unused_imports)]

        use async_graphql::dataloader::Loader;
        use std::collections::HashMap;
        use tonic::transport::Channel;

        /// DataLoader for fetching #related_type by #parent_type ID
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
            type Value = #value_type;
            type Error = async_graphql::Error;

            async fn load(
                &self,
                keys: &[i64],
            ) -> Result<HashMap<i64, Self::Value>, Self::Error> {
                // Build batch request
                let request = BatchRequest {
                    ids: keys.to_vec(),
                };

                let response = self.client
                    .clone()
                    .#rpc_method_ident(request)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.message()))?;

                // Group results by foreign key
                let mut map: HashMap<i64, Self::Value> = HashMap::new();

                for item in response.into_inner().items {
                    let key = item.#fk_ident;
                    #loop_body
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

/// Generate a simple ID-based loader for fetching entities by their primary key
#[allow(dead_code)]
pub fn generate_id_loader(type_name: &str, service_name: &str) -> TokenStream {
    let loader_name = format_ident!("{}Loader", type_name);
    let type_ident = format_ident!("{}", type_name);
    let client_ident = format_ident!("{}Client", service_name);

    quote! {
        /// DataLoader for fetching #type_name by ID
        pub struct #loader_name {
            client: #client_ident<tonic::transport::Channel>,
        }

        impl #loader_name {
            /// Create a new loader with the given gRPC client
            pub fn new(client: #client_ident<tonic::transport::Channel>) -> Self {
                Self { client }
            }
        }

        impl async_graphql::dataloader::Loader<i64> for #loader_name {
            type Value = #type_ident;
            type Error = async_graphql::Error;

            async fn load(
                &self,
                keys: &[i64],
            ) -> Result<std::collections::HashMap<i64, Self::Value>, Self::Error> {
                // Build batch request
                let request = BatchGetRequest {
                    ids: keys.to_vec(),
                };

                let response = self.client
                    .clone()
                    .batch_get(request)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.message()))?;

                // Build result map
                let mut map = std::collections::HashMap::new();
                for item in response.into_inner().items {
                    map.insert(item.id, #type_ident::from(item));
                }

                Ok(map)
            }
        }
    }
}
