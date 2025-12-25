//! Relay Connection generation
//!
//! Generates Relay-style cursor pagination for list methods marked with `connection: true`.
//! Uses async-graphql's built-in Connection types.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};

/// Generate a connection type alias for a given output type
#[allow(dead_code)]
pub fn generate_connection_type(output_type: &str) -> TokenStream {
    let output_ident = format_ident!("{}", output_type);
    let connection_name = format_ident!("{}Connection", output_type);

    quote! {
        /// Relay connection type for #output_type
        pub type #connection_name = async_graphql::connection::Connection<
            String,
            #output_ident,
            async_graphql::connection::EmptyFields,
            async_graphql::connection::EmptyFields,
        >;
    }
}

/// Generate a connection edge type alias for a given output type
#[allow(dead_code)]
pub fn generate_edge_type(output_type: &str) -> TokenStream {
    let output_ident = format_ident!("{}", output_type);
    let edge_name = format_ident!("{}Edge", output_type);

    quote! {
        /// Relay edge type for #output_type
        pub type #edge_name = async_graphql::connection::Edge<
            String,
            #output_ident,
            async_graphql::connection::EmptyFields,
        >;
    }
}

/// Generate the PageInfo type (standard Relay type, usually provided by async-graphql)
#[allow(dead_code)]
pub fn generate_page_info() -> TokenStream {
    quote! {
        // PageInfo is provided by async_graphql::connection::PageInfo
        // Re-exported for convenience
        pub use async_graphql::connection::PageInfo;
    }
}

/// Generate a full connection resolver body for a list method
#[allow(dead_code)]
pub fn generate_connection_resolver_body(
    rpc_method: &str,
    output_type: &str,
    item_field: &str,
) -> TokenStream {
    let rpc_method_ident = format_ident!("{}", rpc_method);
    let output_ident = format_ident!("{}", output_type);
    let item_field_ident = format_ident!("{}", item_field);

    quote! {
        use async_graphql::connection::{Connection, Edge, query};

        query(
            after,
            before,
            first,
            last,
            |after, before, first, last| async move {
                // Decode cursor to get keyset position
                let cursor = after.as_ref().or(before.as_ref())
                    .and_then(|c| super::node::decode_cursor(c));

                // Calculate limit and direction
                let limit = first.or(last).unwrap_or(20) as i32;
                let is_backward = last.is_some();

                // Build the request
                // Note: The actual request structure depends on the proto definition
                let request = ListRequest {
                    cursor: cursor.map(|id| id.to_string()),
                    limit,
                    direction: if is_backward { "backward".to_string() } else { "forward".to_string() },
                };

                let response = client
                    .clone()
                    .#rpc_method_ident(request)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.message()))?;

                let resp = response.into_inner();

                // Build connection from response
                let mut connection = Connection::new(
                    resp.has_previous_page,
                    resp.has_next_page,
                );

                // Add edges
                for item in resp.#item_field_ident {
                    let cursor = super::node::encode_cursor(item.id);
                    connection.edges.push(Edge::new(cursor, #output_ident::from(item)));
                }

                Ok::<_, async_graphql::Error>(connection)
            }
        ).await
    }
}

/// Generate connection helper module
#[allow(dead_code)]
pub fn generate_connection_helpers() -> TokenStream {
    quote! {
        /// Helper module for Relay connection pagination
        pub mod connection_helpers {
            use super::*;

            /// Default page size for connections
            pub const DEFAULT_PAGE_SIZE: i32 = 20;

            /// Maximum page size for connections
            pub const MAX_PAGE_SIZE: i32 = 100;

            /// Validate and normalize pagination arguments
            pub fn normalize_pagination(
                first: Option<i32>,
                last: Option<i32>,
            ) -> i32 {
                let limit = first.or(last).unwrap_or(DEFAULT_PAGE_SIZE);
                limit.min(MAX_PAGE_SIZE).max(1)
            }

            /// Determine if pagination is backward
            pub fn is_backward_pagination(
                last: Option<i32>,
            ) -> bool {
                last.is_some()
            }
        }
    }
}
