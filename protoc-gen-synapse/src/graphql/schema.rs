//! GraphQL schema generation
//!
//! Generates the unified Query, Mutation, and schema builder for a proto file.
//! This creates the graphql/mod.rs that wires all generated types together.

use crate::error::GeneratorError;
use crate::storage::seaorm::options::{
    get_cached_entity_options, get_cached_graphql_message_options, get_cached_graphql_service_options,
};
use heck::{ToSnakeCase, ToUpperCamelCase};
use proc_macro2::TokenStream;
use prost_types::compiler::code_generator_response::File;
use prost_types::FileDescriptorProto;
use quote::{format_ident, quote};

/// Collect information about all generated types for the schema
pub struct SchemaInfo {
    /// Entity types (message name -> snake_case module name)
    pub entities: Vec<(String, String)>,
    /// Input types (message name -> snake_case module name)
    pub input_types: Vec<(String, String)>,
    /// Filter types (message name -> snake_case module name)
    pub filter_types: Vec<(String, String)>,
    /// OrderBy types (message name -> snake_case module name)
    pub order_by_types: Vec<(String, String)>,
    /// Services (service name)
    pub services: Vec<String>,
}

/// Collect schema information from a file descriptor
pub fn collect_schema_info(file: &FileDescriptorProto) -> SchemaInfo {
    let file_name = file.name.as_deref().unwrap_or("");
    let mut info = SchemaInfo {
        entities: Vec::new(),
        input_types: Vec::new(),
        filter_types: Vec::new(),
        order_by_types: Vec::new(),
        services: Vec::new(),
    };

    for message in &file.message_type {
        let msg_name = message.name.as_deref().unwrap_or("");
        let snake_name = msg_name.to_snake_case();

        // Check for graphql options
        let graphql_opts = get_cached_graphql_message_options(file_name, msg_name);
        let entity_opts = get_cached_entity_options(file_name, msg_name);

        // Skip if graphql.skip is true
        if graphql_opts.as_ref().is_some_and(|o| o.skip) {
            continue;
        }

        // Categorize by type
        if graphql_opts.as_ref().is_some_and(|o| o.input_type) {
            info.input_types.push((msg_name.to_string(), snake_name));
        } else if msg_name.ends_with("Filter") {
            info.filter_types.push((msg_name.to_string(), snake_name));
        } else if msg_name.ends_with("OrderBy") {
            info.order_by_types.push((msg_name.to_string(), snake_name));
        } else if entity_opts.is_some() {
            // It's an entity with a table
            info.entities.push((msg_name.to_string(), snake_name));
        }
    }

    for service in &file.service {
        let svc_name = service.name.as_deref().unwrap_or("");
        let svc_opts = get_cached_graphql_service_options(file_name, svc_name);

        if svc_opts.as_ref().is_some_and(|o| o.skip) {
            continue;
        }

        info.services.push(svc_name.to_string());
    }

    info
}

/// Generate the graphql/mod.rs file that wires everything together
pub fn generate(file: &FileDescriptorProto) -> Result<Option<File>, GeneratorError> {
    let info = collect_schema_info(file);

    // Skip if no GraphQL types to generate
    if info.entities.is_empty() && info.services.is_empty() {
        return Ok(None);
    }

    // Generate module declarations
    let mut mod_declarations = Vec::new();
    let mut pub_uses = Vec::new();

    // Entity modules
    for (name, snake) in &info.entities {
        let mod_name = format_ident!("{}", snake);
        let type_name = format_ident!("{}", name);
        mod_declarations.push(quote! { mod #mod_name; });
        pub_uses.push(quote! { pub use #mod_name::#type_name; });
    }

    // Input type modules
    for (name, snake) in &info.input_types {
        let mod_name = format_ident!("{}", snake);
        let type_name = format_ident!("{}", name);
        mod_declarations.push(quote! { mod #mod_name; });
        pub_uses.push(quote! { pub use #mod_name::#type_name; });
    }

    // Filter type modules
    for (name, snake) in &info.filter_types {
        let mod_name = format_ident!("{}", snake);
        let type_name = format_ident!("{}", name);
        mod_declarations.push(quote! { mod #mod_name; });
        pub_uses.push(quote! { pub use #mod_name::#type_name; });
    }

    // OrderBy type modules
    for (name, snake) in &info.order_by_types {
        let mod_name = format_ident!("{}", snake);
        let type_name = format_ident!("{}", name);
        mod_declarations.push(quote! { mod #mod_name; });
        pub_uses.push(quote! { pub use #mod_name::#type_name; });
    }

    // Service resolver modules (Query and Mutation)
    let mut query_imports = Vec::new();
    let mut mutation_imports = Vec::new();
    let mut storage_imports = Vec::new();
    let mut storage_params = Vec::new();
    let mut storage_data = Vec::new();

    for svc_name in &info.services {
        let svc_snake = svc_name.to_snake_case();
        let svc_camel = svc_name.to_upper_camel_case();

        // Query module
        let query_mod = format_ident!("{}_query", svc_snake);
        let query_type = format_ident!("{}Query", svc_camel);
        mod_declarations.push(quote! { mod #query_mod; });
        query_imports.push(quote! { pub use #query_mod::#query_type; });

        // Mutation module
        let mutation_mod = format_ident!("{}_mutation", svc_snake);
        let mutation_type = format_ident!("{}Mutation", svc_camel);
        mod_declarations.push(quote! { mod #mutation_mod; });
        mutation_imports.push(quote! { pub use #mutation_mod::#mutation_type; });

        // Storage type
        let storage_type = format_ident!("SeaOrm{}Storage", svc_camel);
        let storage_param = format_ident!("{}_storage", svc_snake);
        storage_imports.push(quote! { use super::#storage_type; });
        storage_params.push(quote! { #storage_param: Arc<#storage_type> });
        storage_data.push(quote! { .data(#storage_param) });
    }

    // Generate Connection types for entities
    let connection_types = generate_connection_types(&info.entities);

    // Generate the combined Query and Mutation
    let combined_query = generate_combined_query(&info.services);
    let combined_mutation = generate_combined_mutation(&info.services);

    // Generate schema builder
    let schema_builder = generate_schema_builder(&info.services);

    let code = quote! {
        //! GraphQL module
        //!
        //! Re-exports generated types and provides Query/Mutation schema.
        //! All types are generated by protoc-gen-synapse.
        //! @generated

        #![allow(missing_docs)]
        #![allow(unused_imports)]
        #![allow(dead_code)]

        // Sub-modules
        #(#mod_declarations)*

        // Re-export types
        #(#pub_uses)*
        #(#query_imports)*
        #(#mutation_imports)*

        // Imports for schema
        use async_graphql::{Object, Context, Result, ID, EmptySubscription, Schema, MergedObject};
        use std::sync::Arc;
        #(#storage_imports)*

        // Connection types
        #connection_types

        // Combined Query
        #combined_query

        // Combined Mutation
        #combined_mutation

        // Schema builder
        #schema_builder
    };

    // Format the generated code
    let content = code.to_string();
    let formatted = match syn::parse_file(&content) {
        Ok(parsed) => prettyplease::unparse(&parsed),
        Err(_) => content,
    };

    // Determine output path
    let proto_path = file.name.as_deref().unwrap_or("unknown.proto");
    let output_path = proto_path.replace(".proto", "/graphql/mod.rs");

    Ok(Some(File {
        name: Some(output_path),
        content: Some(formatted),
        ..Default::default()
    }))
}

/// Generate Connection types for all entities
fn generate_connection_types(entities: &[(String, String)]) -> TokenStream {
    let mut types = Vec::new();

    // PageInfo is common
    types.push(quote! {
        /// Relay PageInfo
        #[derive(async_graphql::SimpleObject, Clone, Default)]
        pub struct PageInfo {
            pub has_next_page: bool,
            pub has_previous_page: bool,
            pub start_cursor: Option<String>,
            pub end_cursor: Option<String>,
        }

        impl From<super::PageInfo> for PageInfo {
            fn from(p: super::PageInfo) -> Self {
                Self {
                    has_next_page: p.has_next_page,
                    has_previous_page: p.has_previous_page,
                    start_cursor: p.start_cursor,
                    end_cursor: p.end_cursor,
                }
            }
        }

        /// Order direction for sorting
        #[derive(async_graphql::Enum, Clone, Copy, PartialEq, Eq, Default)]
        pub enum OrderDirection {
            #[default]
            /// Ascending order
            Asc,
            /// Descending order
            Desc,
        }

        impl From<OrderDirection> for super::OrderDirection {
            fn from(d: OrderDirection) -> Self {
                match d {
                    OrderDirection::Asc => Self::Asc,
                    OrderDirection::Desc => Self::Desc,
                }
            }
        }

        impl From<super::OrderDirection> for OrderDirection {
            fn from(d: super::OrderDirection) -> Self {
                match d {
                    super::OrderDirection::Asc => Self::Asc,
                    super::OrderDirection::Desc => Self::Desc,
                    _ => Self::Asc,
                }
            }
        }
    });

    // Generate Edge and Connection for each entity
    for (name, _) in entities {
        let type_ident = format_ident!("{}", name);
        let edge_ident = format_ident!("{}Edge", name);
        let connection_ident = format_ident!("{}Connection", name);
        let edge_doc = format!("Relay Edge for {}", name);
        let connection_doc = format!("Relay Connection for {}", name);

        types.push(quote! {
            #[doc = #edge_doc]
            #[derive(async_graphql::SimpleObject, Clone)]
            pub struct #edge_ident {
                pub cursor: String,
                pub node: #type_ident,
            }

            #[doc = #connection_doc]
            #[derive(async_graphql::SimpleObject, Clone)]
            pub struct #connection_ident {
                pub edges: Vec<#edge_ident>,
                pub page_info: PageInfo,
            }

            impl From<super::#connection_ident> for #connection_ident {
                fn from(c: super::#connection_ident) -> Self {
                    Self {
                        edges: c.edges.into_iter().map(|edge| {
                            // Convert proto edge to graphql edge
                            let node = edge.node.map(#type_ident::from)
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
        });
    }

    quote! { #(#types)* }
}

/// Generate combined Query using MergedObject
fn generate_combined_query(services: &[String]) -> TokenStream {
    if services.is_empty() {
        return quote! {
            pub struct Query;

            #[Object]
            impl Query {
                async fn health(&self) -> bool { true }
            }
        };
    }

    let query_types: Vec<_> = services
        .iter()
        .map(|s| {
            let query_type = format_ident!("{}Query", s.to_upper_camel_case());
            quote! { #query_type }
        })
        .collect();

    quote! {
        /// Combined Query merging all service queries
        #[derive(MergedObject, Default)]
        pub struct Query(#(#query_types),*);
    }
}

/// Generate combined Mutation using MergedObject
fn generate_combined_mutation(services: &[String]) -> TokenStream {
    if services.is_empty() {
        return quote! {
            pub struct Mutation;

            #[Object]
            impl Mutation {
                async fn noop(&self) -> bool { true }
            }
        };
    }

    let mutation_types: Vec<_> = services
        .iter()
        .map(|s| {
            let mutation_type = format_ident!("{}Mutation", s.to_upper_camel_case());
            quote! { #mutation_type }
        })
        .collect();

    quote! {
        /// Combined Mutation merging all service mutations
        #[derive(MergedObject, Default)]
        pub struct Mutation(#(#mutation_types),*);
    }
}

/// Generate schema builder function
fn generate_schema_builder(services: &[String]) -> TokenStream {
    let storage_params: Vec<_> = services
        .iter()
        .map(|s| {
            let svc_snake = s.to_snake_case();
            let svc_camel = s.to_upper_camel_case();
            let storage_type = format_ident!("SeaOrm{}Storage", svc_camel);
            let param_name = format_ident!("{}_storage", svc_snake);
            quote! { #param_name: Arc<#storage_type> }
        })
        .collect();

    let storage_data: Vec<_> = services
        .iter()
        .map(|s| {
            let svc_snake = s.to_snake_case();
            let param_name = format_ident!("{}_storage", svc_snake);
            quote! { .data(#param_name) }
        })
        .collect();

    // Determine package name for schema type alias
    let schema_name = format_ident!("AppSchema");

    quote! {
        /// Schema type alias
        pub type #schema_name = Schema<Query, Mutation, EmptySubscription>;

        /// Build the GraphQL schema with storage instances
        pub fn build_schema(#(#storage_params),*) -> #schema_name {
            Schema::build(Query::default(), Mutation::default(), EmptySubscription)
                #(#storage_data)*
                .finish()
        }
    }
}
