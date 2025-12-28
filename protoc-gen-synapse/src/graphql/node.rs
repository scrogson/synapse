//! Relay Node interface generation
//!
//! Generates the Node interface for types marked with `node: true`.
//! Provides global ID encoding/decoding using base62 for URL-safe IDs.

use crate::error::GeneratorError;
use crate::storage::seaorm::options::get_cached_graphql_type_options;
use heck::ToUpperCamelCase;
use proc_macro2::TokenStream;
use prost_types::compiler::code_generator_response::File;
use prost_types::{DescriptorProto, FileDescriptorProto};
use quote::{format_ident, quote};

/// Collect all node types from a file descriptor
pub fn collect_node_types(
    file: &FileDescriptorProto,
) -> Vec<(&DescriptorProto, String)> {
    let file_name = file.name.as_deref().unwrap_or("");
    let mut node_types = Vec::new();

    for message in &file.message_type {
        let msg_name = message.name.as_deref().unwrap_or("");
        let msg_opts = get_cached_graphql_type_options(file_name, msg_name);

        if msg_opts.as_ref().is_some_and(|o| o.node && !o.skip) {
            let type_name = msg_opts
                .as_ref()
                .filter(|o| !o.name.is_empty())
                .map(|o| o.name.clone())
                .unwrap_or_else(|| msg_name.to_upper_camel_case());
            node_types.push((message, type_name));
        }
    }

    node_types
}

/// Generate the Node interface enum and node query
pub fn generate_node_interface(
    file: &FileDescriptorProto,
    node_types: &[(&DescriptorProto, String)],
) -> Result<Option<File>, GeneratorError> {
    if node_types.is_empty() {
        return Ok(None);
    }

    // Generate enum variants
    let variants = generate_node_variants(node_types);

    // Generate node query resolver
    let node_resolver = generate_node_resolver(node_types);

    // Generate nodes (batch) query resolver
    let nodes_resolver = generate_nodes_resolver(node_types);

    let code = quote! {
        //! Relay Node interface
        //! @generated

        #![allow(missing_docs)]
        #![allow(unused_imports)]

        use async_graphql::{Interface, Object, Context, Result, ID};
        use tonic::transport::Channel;

        /// Relay Node interface - allows fetching any object by global ID
        #[derive(Interface)]
        #[graphql(field(name = "id", ty = "ID"))]
        pub enum Node {
            #variants
        }

        /// Root query for fetching nodes by global ID
        pub struct NodeQuery;

        #[Object]
        impl NodeQuery {
            #node_resolver
            #nodes_resolver
        }

        /// Encode a local ID to a global Relay ID using base62
        pub fn encode_global_id(type_name: &str, local_id: i64) -> ID {
            let raw = format!("{}:{}", type_name, local_id);
            ID(base62::encode(raw.as_bytes()))
        }

        /// Decode a global Relay ID to type name and local ID
        pub fn decode_global_id(id: &ID) -> Option<(String, i64)> {
            let bytes = base62::decode(id.as_str()).ok()?;
            let s = String::from_utf8(bytes).ok()?;
            let (type_name, local_id) = s.split_once(':')?;
            let id = local_id.parse().ok()?;
            Some((type_name.to_string(), id))
        }

        /// Cursor encoding for pagination
        pub fn encode_cursor(id: i64) -> String {
            base62::encode(id.to_string().as_bytes())
        }

        /// Cursor decoding for pagination
        pub fn decode_cursor(cursor: &str) -> Option<i64> {
            base62::decode(cursor)
                .ok()
                .and_then(|b| String::from_utf8(b).ok())
                .and_then(|s| s.parse().ok())
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
    let output_path = format!("{}/graphql/node.rs", package.replace('.', "/"));

    Ok(Some(File {
        name: Some(output_path),
        content: Some(formatted),
        ..Default::default()
    }))
}

/// Generate Node enum variants
fn generate_node_variants(node_types: &[(&DescriptorProto, String)]) -> TokenStream {
    let variants: Vec<_> = node_types
        .iter()
        .map(|(_, type_name)| {
            let ident = format_ident!("{}", type_name);
            quote! { #ident(#ident), }
        })
        .collect();

    quote! { #(#variants)* }
}

/// Generate the node query resolver
fn generate_node_resolver(node_types: &[(&DescriptorProto, String)]) -> TokenStream {
    // Generate match arms for each type
    let match_arms: Vec<_> = node_types
        .iter()
        .map(|(_, type_name)| {
            let type_str = type_name.as_str();
            let type_ident = format_ident!("{}", type_name);
            let loader_ident = format_ident!("{}Loader", type_name);

            quote! {
                #type_str => {
                    let loader = ctx.data_unchecked::<async_graphql::dataloader::DataLoader<#loader_ident>>();
                    let entity = loader.load_one(local_id).await?;
                    Ok(entity.map(Node::#type_ident))
                }
            }
        })
        .collect();

    quote! {
        /// Fetch any node by its global ID
        async fn node(&self, ctx: &Context<'_>, id: ID) -> Result<Option<Node>> {
            let (type_name, local_id) = decode_global_id(&id)
                .ok_or_else(|| async_graphql::Error::new("Invalid node ID"))?;

            match type_name.as_str() {
                #(#match_arms)*
                _ => Ok(None),
            }
        }
    }
}

/// Generate the nodes (batch) query resolver
fn generate_nodes_resolver(node_types: &[(&DescriptorProto, String)]) -> TokenStream {
    // Generate match arms for each type (batch version)
    let match_arms: Vec<_> = node_types
        .iter()
        .map(|(_, type_name)| {
            let type_str = type_name.as_str();
            let type_ident = format_ident!("{}", type_name);
            let loader_ident = format_ident!("{}Loader", type_name);

            quote! {
                #type_str => {
                    let loader = ctx.data_unchecked::<async_graphql::dataloader::DataLoader<#loader_ident>>();
                    let entity = loader.load_one(local_id).await?;
                    results.push(entity.map(Node::#type_ident));
                }
            }
        })
        .collect();

    quote! {
        /// Fetch multiple nodes by their global IDs
        async fn nodes(&self, ctx: &Context<'_>, ids: Vec<ID>) -> Result<Vec<Option<Node>>> {
            let mut results = Vec::with_capacity(ids.len());

            for id in ids {
                let parsed = decode_global_id(&id);

                if let Some((type_name, local_id)) = parsed {
                    match type_name.as_str() {
                        #(#match_arms)*
                        _ => results.push(None),
                    }
                } else {
                    results.push(None);
                }
            }

            Ok(results)
        }
    }
}

/// Generate Node-related methods for a specific type
#[allow(dead_code)]
pub fn generate_type_node_impl(type_name: &str) -> TokenStream {
    let type_name_str = type_name;

    quote! {
        /// Relay global ID
        pub fn global_id(&self) -> async_graphql::ID {
            super::node::encode_global_id(#type_name_str, self.id)
        }

        /// Decode global Relay ID to local ID
        pub fn from_global_id(id: &async_graphql::ID) -> Option<i64> {
            let (type_name, local_id) = super::node::decode_global_id(id)?;
            if type_name != #type_name_str {
                return None;
            }
            Some(local_id)
        }
    }
}
