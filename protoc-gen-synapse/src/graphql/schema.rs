//! GraphQL schema generation
//!
//! Generates the unified Query, Mutation, and schema builder for a proto file.
//! This creates the graphql/mod.rs that wires all generated types together.

use heck::{ToSnakeCase, ToUpperCamelCase};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use synapse_gen::ir::options::GraphQLMethodKind;
use synapse_gen::ir::RelationType;
use synapse_gen::{GeneratedFile, GeneratorContext, GeneratorError};

/// Collect information about all generated types for the schema
pub struct SchemaInfo {
    /// Entity types (message name -> snake_case module name)
    pub entities: Vec<(String, String)>,
    /// Input types (message name -> snake_case module name)
    pub input_types: Vec<(String, String)>,
    /// Auto-generated input types from request messages (input name -> snake_case module name)
    pub auto_input_types: Vec<(String, String)>,
    /// Services (service name)
    pub services: Vec<String>,
    /// Whether we have auto-generated filters
    pub has_auto_filters: bool,
    /// HasMany relations (parent_type, related_type) for DataLoader registration
    pub has_many_relations: Vec<(String, String)>,
}

/// Collect schema information from the IR context
pub fn collect_schema_info(ctx: &GeneratorContext) -> SchemaInfo {
    let mut info = SchemaInfo {
        entities: Vec::new(),
        input_types: Vec::new(),
        auto_input_types: Vec::new(),
        services: Vec::new(),
        has_auto_filters: false,
        has_many_relations: Vec::new(),
    };

    // Entities from IR
    for entity in &ctx.package.entities {
        // Skip if graphql.skip is true
        if entity.graphql.as_ref().is_some_and(|o| o.skip) {
            continue;
        }

        let msg_name = &entity.name;
        let snake_name = msg_name.to_snake_case();

        info.entities.push((msg_name.clone(), snake_name.clone()));
        info.has_auto_filters = true;

        // Collect HasMany relations for DataLoader registration
        for relation in &entity.relations {
            let has_fk = !relation.foreign_key.is_empty();
            if has_fk
                && matches!(
                    relation.relation_type,
                    RelationType::HasMany | RelationType::ManyToMany
                )
            {
                info.has_many_relations.push((
                    msg_name.clone(),
                    relation.related.clone(),
                ));
            }
        }
    }

    // Input types from messages
    for message in &ctx.package.messages {
        let graphql_opts = message.graphql.as_ref();

        // Skip if graphql.skip is true
        if graphql_opts.is_some_and(|o| o.skip) {
            continue;
        }

        if graphql_opts.is_some_and(|o| o.input) {
            let msg_name = &message.name;
            let snake_name = msg_name.to_snake_case();
            info.input_types.push((msg_name.clone(), snake_name));
        }
    }

    // Services from IR
    for service in &ctx.package.services {
        let svc_name = &service.name;

        if service.graphql.as_ref().is_some_and(|o| o.skip) {
            continue;
        }

        info.services.push(svc_name.clone());

        // Collect auto-generated input types from mutation methods
        for method in &service.methods {
            let Some(ref graphql_opts) = method.graphql else {
                continue;
            };

            if graphql_opts.kind != GraphQLMethodKind::Mutation || graphql_opts.skip {
                continue;
            }

            // Check if this is a create or update operation
            let method_name = &method.name;
            let is_create = method_name.to_lowercase().starts_with("create");
            let is_update = method_name.to_lowercase().starts_with("update");

            if is_create || is_update {
                // Derive input type name from request type
                let request_name = method.input_type.rsplit('.').next().unwrap_or(&method.input_type);
                let input_name = request_name.replace("Request", "Input");
                let input_snake = input_name.to_snake_case();
                info.auto_input_types.push((input_name, input_snake));
            }
        }
    }

    info
}

/// Generate the graphql/mod.rs file that wires everything together
pub fn generate(
    ctx: &GeneratorContext,
) -> Result<Option<GeneratedFile>, GeneratorError> {
    let info = collect_schema_info(ctx);

    // Skip if no GraphQL types to generate
    if info.entities.is_empty() && info.services.is_empty() {
        return Ok(None);
    }

    // Generate module declarations
    let mut mod_declarations = Vec::new();
    let mut pub_uses = Vec::new();

    // Import shared filter types from synapse::relay::graphql (not generated per-package)
    // These types are generated once in synapse/relay/graphql/ and shared across all packages
    if info.has_auto_filters {
        pub_uses.push(quote! {
            pub use super::super::synapse::relay::graphql::{
                IntFilter, StringFilter, BoolFilter, TimestampFilter,
                OrderDirection, PageInfo
            };
        });
    }

    // Entity modules and their auto-generated types (GraphQL wrappers)
    for (name, snake) in &info.entities {
        let mod_name = format_ident!("{}", snake);
        let type_name = format_ident!("{}", name);
        mod_declarations.push(quote! { mod #mod_name; });
        pub_uses.push(quote! { pub use #mod_name::#type_name; });

        // Filter, orderBy, edge, connection for this entity (always generated)
        let filter_name = format!("{}Filter", name);
        let order_by_name = format!("{}OrderBy", name);
        let edge_name = format!("{}Edge", name);
        let connection_name = format!("{}Connection", name);
        let loader_name = format!("{}Loader", name);

        let filter_mod = format_ident!("{}_filter", snake);
        let filter_type = format_ident!("{}", filter_name);
        mod_declarations.push(quote! { mod #filter_mod; });
        pub_uses.push(quote! { pub use #filter_mod::#filter_type; });

        let order_by_mod = format_ident!("{}_order_by", snake);
        let order_by_type = format_ident!("{}", order_by_name);
        mod_declarations.push(quote! { mod #order_by_mod; });
        pub_uses.push(quote! { pub use #order_by_mod::#order_by_type; });

        // Edge and Connection
        let edge_mod = format_ident!("{}_edge", snake);
        let edge_type = format_ident!("{}", edge_name);
        mod_declarations.push(quote! { mod #edge_mod; });
        pub_uses.push(quote! { pub use #edge_mod::#edge_type; });

        let connection_mod = format_ident!("{}_connection", snake);
        let connection_type = format_ident!("{}", connection_name);
        mod_declarations.push(quote! { mod #connection_mod; });
        pub_uses.push(quote! { pub use #connection_mod::#connection_type; });

        // Entity loader (for BelongsTo relations)
        let loader_mod = format_ident!("{}_loader", snake);
        let loader_type = format_ident!("{}", loader_name);
        mod_declarations.push(quote! { mod #loader_mod; });
        pub_uses.push(quote! { pub use #loader_mod::#loader_type; });
    }

    // HasMany relation loaders (e.g., PostsByUserLoader)
    for (parent_type, related_type) in &info.has_many_relations {
        let loader_name = format!(
            "{}sBy{}Loader",
            related_type.to_upper_camel_case(),
            parent_type.to_upper_camel_case()
        );
        let loader_mod = format_ident!(
            "{}s_by_{}_loader",
            related_type.to_snake_case(),
            parent_type.to_snake_case()
        );
        let loader_type = format_ident!("{}", loader_name);
        mod_declarations.push(quote! { mod #loader_mod; });
        pub_uses.push(quote! { pub use #loader_mod::#loader_type; });
    }

    // User-defined input type modules
    for (name, snake) in &info.input_types {
        let mod_name = format_ident!("{}", snake);
        let type_name = format_ident!("{}", name);
        mod_declarations.push(quote! { mod #mod_name; });
        pub_uses.push(quote! { pub use #mod_name::#type_name; });
    }

    // Auto-generated input types from request messages
    for (name, snake) in &info.auto_input_types {
        let mod_name = format_ident!("{}", snake);
        let type_name = format_ident!("{}", name);
        mod_declarations.push(quote! { mod #mod_name; });
        pub_uses.push(quote! { pub use #mod_name::#type_name; });
    }

    // Service resolver modules (Query and Mutation)
    let mut query_imports = Vec::new();
    let mut mutation_imports = Vec::new();
    let mut client_imports = Vec::new();

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

        // Client type (from tonic-generated submodule)
        let client_module = format_ident!("{}_client", svc_snake);
        let client_type = format_ident!("{}Client", svc_camel);
        client_imports.push(quote! { use super::#client_module::#client_type; });
    }

    // Generate the combined Query and Mutation
    let combined_query = generate_combined_query(&info.services);
    let combined_mutation = generate_combined_mutation(&info.services);

    // Generate schema builder
    let schema_builder = generate_schema_builder(&info.services, &info.entities, &info.has_many_relations);

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
        use async_graphql::dataloader::DataLoader;
        use tonic::transport::Channel;
        #(#client_imports)*

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

    // Determine output path using package name
    let package_name = &ctx.package.name;
    let output_path = format!("{}/graphql/mod.rs", package_name.replace('.', "/"));

    Ok(Some(GeneratedFile {
        path: output_path,
        content: formatted,
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
fn generate_schema_builder(
    services: &[String],
    entities: &[(String, String)],
    has_many_relations: &[(String, String)],
) -> TokenStream {
    // Generate client parameters (one per service)
    let client_params: Vec<_> = services
        .iter()
        .map(|s| {
            let svc_snake = s.to_snake_case();
            let svc_camel = s.to_upper_camel_case();
            let client_type = format_ident!("{}Client", svc_camel);
            let param_name = format_ident!("{}_client", svc_snake);
            quote! { #param_name: #client_type<Channel> }
        })
        .collect();

    // Generate client data registration
    let client_data: Vec<_> = services
        .iter()
        .map(|s| {
            let svc_snake = s.to_snake_case();
            let param_name = format_ident!("{}_client", svc_snake);
            quote! { .data(#param_name.clone()) }
        })
        .collect();

    // Generate DataLoader creation and data registration for each entity (BelongsTo)
    let loader_data: Vec<_> = entities
        .iter()
        .map(|(name, _snake)| {
            let loader_type = format_ident!("{}Loader", name);
            // Derive service name from entity name (e.g., User -> UserService -> user_service_client)
            let service_param = format_ident!("{}_service_client", name.to_snake_case());
            quote! {
                .data(DataLoader::new(
                    #loader_type::new(#service_param.clone()),
                    tokio::spawn
                ))
            }
        })
        .collect();

    // Generate DataLoader creation for HasMany relations (e.g., PostsByUserLoader)
    let relation_loader_data: Vec<_> = has_many_relations
        .iter()
        .map(|(parent_type, related_type)| {
            let loader_type = format_ident!(
                "{}sBy{}Loader",
                related_type.to_upper_camel_case(),
                parent_type.to_upper_camel_case()
            );
            // Use the related entity's service client
            let service_param = format_ident!("{}_service_client", related_type.to_snake_case());
            quote! {
                .data(DataLoader::new(
                    #loader_type::new(#service_param.clone()),
                    tokio::spawn
                ))
            }
        })
        .collect();

    // Determine package name for schema type alias
    let schema_name = format_ident!("AppSchema");

    quote! {
        /// Schema type alias
        pub type #schema_name = Schema<Query, Mutation, EmptySubscription>;

        /// Build the GraphQL schema with gRPC clients
        ///
        /// Creates DataLoaders for efficient batched loading in relation resolvers.
        pub fn build_schema(#(#client_params),*) -> #schema_name {
            Schema::build(Query::default(), Mutation::default(), EmptySubscription)
                #(#client_data)*
                #(#loader_data)*
                #(#relation_loader_data)*
                .finish()
        }
    }
}
