//! DataLoader generation
//!
//! Generates async-graphql DataLoaders for efficient batched data fetching.
//! DataLoaders prevent N+1 queries by batching multiple lookups into single requests.
//!
//! Two types of loaders are generated:
//! 1. ID Loaders (for BelongsTo): Load entities by their primary key
//! 2. Relation Loaders (for HasMany): Load related entities by foreign key

use heck::{ToSnakeCase, ToUpperCamelCase};
use quote::{format_ident, quote};
use synapse_gen::ir::{Entity, Relation, RelationType};
use synapse_gen::{GeneratedFile, GeneratorError};

/// Generate DataLoaders for an entity based on its relations
pub fn generate(
    package_name: &str,
    entity: &Entity,
    all_entities: &[&Entity],
) -> Result<Vec<GeneratedFile>, GeneratorError> {
    // Check for graphql type options
    let graphql_opts = entity.graphql.as_ref();

    // Skip if no graphql options or explicitly skipped
    if graphql_opts.is_some_and(|o| o.skip) {
        return Ok(vec![]);
    }

    // If no relations, skip
    if entity.relations.is_empty() {
        return Ok(vec![]);
    }

    // Determine type name
    let type_name = graphql_opts
        .filter(|o| !o.name.is_empty())
        .map(|o| o.name.clone())
        .unwrap_or_else(|| entity.name.to_upper_camel_case());

    let mut loaders = Vec::new();

    // Generate a loader for each relation
    for relation in &entity.relations {
        if let Some(loader) = generate_relation_loader(package_name, &type_name, relation, all_entities)? {
            loaders.push(loader);
        }
    }

    Ok(loaders)
}

/// Generate a DataLoader for a specific relation (HasMany)
///
/// Uses the List RPC with an IN filter on the foreign key for true batch loading.
fn generate_relation_loader(
    package_name: &str,
    parent_type: &str,
    relation: &Relation,
    all_entities: &[&Entity],
) -> Result<Option<GeneratedFile>, GeneratorError> {
    let related_type = &relation.related;
    let foreign_key = &relation.foreign_key;

    // Only generate for HasMany/ManyToMany relations
    let is_many = matches!(
        relation.relation_type,
        RelationType::HasMany | RelationType::ManyToMany
    );

    if !is_many {
        return Ok(None);
    }

    // Skip if related_type or foreign_key is empty
    // ManyToMany relations use `through` table instead of foreign_key
    if related_type.is_empty() || foreign_key.is_empty() {
        return Ok(None);
    }

    // Check if the FK field on the related entity is optional
    let fk_is_optional = find_field_optionality(all_entities, related_type, foreign_key);

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

    // Generate populate code based on FK optionality
    let populate_code = if fk_is_optional {
        // Optional FK: unwrap before using as map key
        quote! {
            if let Some(key) = entity.#fk_ident {
                if let Some(vec) = map.get_mut(&key) {
                    vec.push(entity);
                }
            }
        }
    } else {
        // Required FK: use directly
        quote! {
            let key = entity.#fk_ident;
            if let Some(vec) = map.get_mut(&key) {
                vec.push(entity);
            }
        }
    };

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
                        #populate_code
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
    let output_path = format!(
        "{}/graphql/{}.rs",
        package_name.replace('.', "/"),
        loader_name.to_snake_case()
    );

    Ok(Some(GeneratedFile {
        path: output_path,
        content: formatted,
    }))
}

/// Generate an ID-based loader file for fetching entities by their primary key
///
/// Uses the List RPC with an IN filter for true batch loading (single query).
/// It's used for BelongsTo relations (e.g., Post.author uses UserLoader).
pub fn generate_entity_loader(
    package_name: &str,
    entity: &Entity,
) -> Result<Option<GeneratedFile>, GeneratorError> {
    // Check for graphql type options
    let graphql_opts = entity.graphql.as_ref();

    // Skip if no graphql options or explicitly skipped
    if graphql_opts.is_some_and(|o| o.skip) {
        return Ok(None);
    }

    // Determine type name
    let type_name = graphql_opts
        .filter(|o| !o.name.is_empty())
        .map(|o| o.name.clone())
        .unwrap_or_else(|| entity.name.to_upper_camel_case());

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
    let output_path = format!(
        "{}/graphql/{}_loader.rs",
        package_name.replace('.', "/"),
        type_name.to_snake_case()
    );

    Ok(Some(GeneratedFile {
        path: output_path,
        content: formatted,
    }))
}

/// Find if a field on a message is optional (proto3_optional)
fn find_field_optionality(
    all_entities: &[&Entity],
    message_name: &str,
    field_name: &str,
) -> bool {
    let field_snake = field_name.to_snake_case();
    let message_camel = message_name.to_upper_camel_case();

    for entity in all_entities {
        if entity.name.to_upper_camel_case() == message_camel {
            // Found the entity, now find the field in raw descriptor
            for field in &entity.raw.field {
                let name = field.name.as_deref().unwrap_or("");
                if name.to_snake_case() == field_snake {
                    return field.proto3_optional.unwrap_or(false);
                }
            }
        }
    }
    false
}
