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

/// Check if a message type exists in the file
fn message_exists(file: &FileDescriptorProto, name: &str) -> bool {
    file.message_type
        .iter()
        .any(|m| m.name.as_deref() == Some(name))
}

/// Collect information about all generated types for the schema
pub struct SchemaInfo {
    /// Entity types (message name -> snake_case module name)
    pub entities: Vec<(String, String)>,
    /// Input types (message name -> snake_case module name)
    pub input_types: Vec<(String, String)>,
    /// Services (service name)
    pub services: Vec<String>,
    /// Whether we have auto-generated filters
    pub has_auto_filters: bool,
}

/// Collect schema information from a file descriptor
pub fn collect_schema_info(file: &FileDescriptorProto) -> SchemaInfo {
    let file_name = file.name.as_deref().unwrap_or("");
    let mut info = SchemaInfo {
        entities: Vec::new(),
        input_types: Vec::new(),
        services: Vec::new(),
        has_auto_filters: false,
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
            // Include input types (including proto-defined Filter/OrderBy types)
            info.input_types.push((msg_name.to_string(), snake_name));
        } else if entity_opts.is_some() {
            // It's an entity with a table - filters may be auto-generated
            info.entities.push((msg_name.to_string(), snake_name));
            info.has_auto_filters = true;
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

    // Primitive filter modules - auto-generate if not in proto, otherwise object.rs generates wrapper
    if info.has_auto_filters {
        // IntFilter
        if !message_exists(file, "IntFilter") {
            mod_declarations.push(quote! { mod int_filter; });
            pub_uses.push(quote! { pub use int_filter::IntFilter; });
        }
        // StringFilter
        if !message_exists(file, "StringFilter") {
            mod_declarations.push(quote! { mod string_filter; });
            pub_uses.push(quote! { pub use string_filter::StringFilter; });
        }
        // BoolFilter
        if !message_exists(file, "BoolFilter") {
            mod_declarations.push(quote! { mod bool_filter; });
            pub_uses.push(quote! { pub use bool_filter::BoolFilter; });
        }
        // OrderDirection (always auto-generated)
        mod_declarations.push(quote! { mod order_direction; });
        pub_uses.push(quote! { pub use order_direction::OrderDirection; });
        // PageInfo (always auto-generated)
        mod_declarations.push(quote! { mod page_info; });
        pub_uses.push(quote! { pub use page_info::PageInfo; });
    }

    // Entity modules and their auto-generated types
    for (name, snake) in &info.entities {
        let mod_name = format_ident!("{}", snake);
        let type_name = format_ident!("{}", name);
        mod_declarations.push(quote! { mod #mod_name; });
        pub_uses.push(quote! { pub use #mod_name::#type_name; });

        // Filter, orderBy, edge, connection for this entity
        // Auto-generate if not in proto, otherwise object.rs generates wrapper
        let filter_name = format!("{}Filter", name);
        let order_by_name = format!("{}OrderBy", name);
        let edge_name = format!("{}Edge", name);
        let connection_name = format!("{}Connection", name);

        if !message_exists(file, &filter_name) {
            let filter_mod = format_ident!("{}_filter", snake);
            let filter_type = format_ident!("{}", filter_name);
            mod_declarations.push(quote! { mod #filter_mod; });
            pub_uses.push(quote! { pub use #filter_mod::#filter_type; });
        }

        if !message_exists(file, &order_by_name) {
            let order_by_mod = format_ident!("{}_order_by", snake);
            let order_by_type = format_ident!("{}", order_by_name);
            mod_declarations.push(quote! { mod #order_by_mod; });
            pub_uses.push(quote! { pub use #order_by_mod::#order_by_type; });
        }

        // Edge and Connection (always auto-generated)
        let edge_mod = format_ident!("{}_edge", snake);
        let edge_type = format_ident!("{}", edge_name);
        mod_declarations.push(quote! { mod #edge_mod; });
        pub_uses.push(quote! { pub use #edge_mod::#edge_type; });

        let connection_mod = format_ident!("{}_connection", snake);
        let connection_type = format_ident!("{}", connection_name);
        mod_declarations.push(quote! { mod #connection_mod; });
        pub_uses.push(quote! { pub use #connection_mod::#connection_type; });
    }

    // User-defined input type modules
    for (name, snake) in &info.input_types {
        let mod_name = format_ident!("{}", snake);
        let type_name = format_ident!("{}", name);
        mod_declarations.push(quote! { mod #mod_name; });
        pub_uses.push(quote! { pub use #mod_name::#type_name; });
    }

    // Service resolver modules (Query and Mutation)
    let mut query_imports = Vec::new();
    let mut mutation_imports = Vec::new();
    let mut storage_imports = Vec::new();

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
        storage_imports.push(quote! { use super::#storage_type; });
    }

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
