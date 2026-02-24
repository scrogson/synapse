//! Auto-generated Relay connection types for GraphQL
//!
//! Generates:
//! - PageInfo type (once per package)
//! - Entity Edge types (UserEdge, PostEdge, etc.)
//! - Entity Connection types (UserConnection, PostConnection, etc.)

use heck::{ToSnakeCase, ToUpperCamelCase};
use quote::{format_ident, quote};
use synapse_gen::ir::Entity;
use synapse_gen::{GeneratedFile, GeneratorError};

/// Generate all Relay connection types for a package
///
/// Only generates types that are NOT already defined in proto.
/// Proto-defined types are handled by object.rs which generates proper GraphQL wrappers.
pub fn generate_connections_for_package(
    package_name: &str,
    entities: &[&Entity],
) -> Result<Vec<GeneratedFile>, GeneratorError> {
    let mut files = Vec::new();

    // Always generate PageInfo (proto PageInfo needs GraphQL wrapper)
    files.push(generate_page_info()?);

    // Always generate Edge and Connection types for each entity
    // (proto Connection types have synapse.storage.connection_type, not graphql.message)
    for entity in entities {
        let entity_name = entity.raw.name.as_deref().unwrap_or("");
        files.push(generate_entity_edge(package_name, entity_name)?);
        files.push(generate_entity_connection(package_name, entity_name)?);
    }

    Ok(files)
}

/// Generate PageInfo type (in shared synapse/relay/graphql location)
fn generate_page_info() -> Result<GeneratedFile, GeneratorError> {
    let code = quote! {
        //! Auto-generated Relay PageInfo type
        //! @generated

        #![allow(missing_docs)]

        use async_graphql::SimpleObject;

        /// Relay PageInfo for cursor-based pagination
        #[derive(SimpleObject, Clone, Default)]
        pub struct PageInfo {
            /// Whether there are more items after the last edge
            pub has_next_page: bool,
            /// Whether there are more items before the first edge
            pub has_previous_page: bool,
            /// Cursor of the first edge
            pub start_cursor: Option<String>,
            /// Cursor of the last edge
            pub end_cursor: Option<String>,
        }

        // Convert from proto PageInfo
        impl From<super::super::PageInfo> for PageInfo {
            fn from(p: super::super::PageInfo) -> Self {
                Self {
                    has_next_page: p.has_next_page,
                    has_previous_page: p.has_previous_page,
                    start_cursor: p.start_cursor,
                    end_cursor: p.end_cursor,
                }
            }
        }
    };

    let content = code.to_string();
    let formatted = match syn::parse_file(&content) {
        Ok(parsed) => prettyplease::unparse(&parsed),
        Err(_) => content,
    };

    // Output to shared location: synapse/relay/graphql/
    let output_path = "synapse/relay/graphql/page_info.rs".to_string();

    Ok(GeneratedFile {
        path: output_path,
        content: formatted,
    })
}

/// Generate entity Edge type (e.g., UserEdge)
fn generate_entity_edge(
    package_name: &str,
    entity_name: &str,
) -> Result<GeneratedFile, GeneratorError> {
    let edge_name = format!("{}Edge", entity_name.to_upper_camel_case());
    let edge_ident = format_ident!("{}", edge_name);
    let entity_ident = format_ident!("{}", entity_name.to_upper_camel_case());

    let code = quote! {
        //! Auto-generated Relay Edge type for #entity_name
        //! @generated

        #![allow(missing_docs)]

        use async_graphql::SimpleObject;
        use super::#entity_ident;

        /// Relay Edge for #entity_name
        #[derive(SimpleObject, Clone)]
        pub struct #edge_ident {
            /// Cursor for this edge
            pub cursor: String,
            /// The node (entity) at this edge
            pub node: #entity_ident,
        }
    };

    let content = code.to_string();
    let formatted = match syn::parse_file(&content) {
        Ok(parsed) => prettyplease::unparse(&parsed),
        Err(_) => content,
    };

    let output_path = format!(
        "{}/graphql/{}_edge.rs",
        package_name.replace('.', "/"),
        entity_name.to_snake_case()
    );

    Ok(GeneratedFile {
        path: output_path,
        content: formatted,
    })
}

/// Generate entity Connection type (e.g., UserConnection)
fn generate_entity_connection(
    package_name: &str,
    entity_name: &str,
) -> Result<GeneratedFile, GeneratorError> {
    let connection_name = format!("{}Connection", entity_name.to_upper_camel_case());
    let connection_ident = format_ident!("{}", connection_name);
    let edge_name = format!("{}Edge", entity_name.to_upper_camel_case());
    let edge_ident = format_ident!("{}", edge_name);
    let entity_ident = format_ident!("{}", entity_name.to_upper_camel_case());

    let code = quote! {
        //! Auto-generated Relay Connection type for entity
        //! @generated

        #![allow(missing_docs)]

        use async_graphql::SimpleObject;
        use super::#edge_ident;
        use super::#entity_ident;
        // Import PageInfo from shared synapse::relay::graphql
        use super::super::super::synapse::relay::graphql::PageInfo;

        /// Relay Connection for entity
        #[derive(SimpleObject, Clone)]
        pub struct #connection_ident {
            /// List of edges
            pub edges: Vec<#edge_ident>,
            /// Pagination info
            pub page_info: PageInfo,
        }

        impl From<super::super::#connection_ident> for #connection_ident {
            fn from(c: super::super::#connection_ident) -> Self {
                Self {
                    edges: c.edges.into_iter().map(|edge| {
                        let node = edge.node.map(#entity_ident::from)
                            .expect("Edge node should not be None");
                        #edge_ident {
                            cursor: edge.cursor,
                            node,
                        }
                    }).collect(),
                    page_info: c.page_info.map(PageInfo::from).unwrap_or_default(),
                }
            }
        }
    };

    let content = code.to_string();
    let formatted = match syn::parse_file(&content) {
        Ok(parsed) => prettyplease::unparse(&parsed),
        Err(_) => content,
    };

    let output_path = format!(
        "{}/graphql/{}_connection.rs",
        package_name.replace('.', "/"),
        entity_name.to_snake_case()
    );

    Ok(GeneratedFile {
        path: output_path,
        content: formatted,
    })
}
