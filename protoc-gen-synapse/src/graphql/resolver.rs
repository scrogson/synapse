//! GraphQL resolver generation
//!
//! Generates async-graphql Query/Mutation resolvers from protobuf service definitions.
//! Resolvers use gRPC clients (tonic-generated) for data fetching.
//!
//! For mutations with context-injected fields, the resolver extracts values from
//! the GraphQL context and passes them to `input.to_request()`.

use heck::{ToSnakeCase, ToUpperCamelCase};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use synapse_gen::ir::options::GraphQLMethodKind;
use synapse_gen::ir::{Method, Service};
use synapse_gen::{GeneratedFile, GeneratorContext, GeneratorError};

/// Information about context-injected fields for a request message
struct ContextFieldInfo {
    /// Field name (snake_case)
    name: String,
    /// Context path (e.g., "current_user.id")
    path: String,
}

/// Check a request message for context-injected fields
fn get_context_fields(
    ctx: &GeneratorContext,
    request_type_name: &str,
) -> Vec<ContextFieldInfo> {
    // Find the request message in the package's messages
    let msg = ctx
        .package
        .messages
        .iter()
        .find(|m| m.name == request_type_name);

    let Some(message) = msg else {
        return Vec::new();
    };

    let mut context_fields = Vec::new();

    for field in &message.fields {
        if let Some(ref graphql_opts) = field.graphql {
            if let Some(ref ctx_source) = graphql_opts.from_context {
                context_fields.push(ContextFieldInfo {
                    name: field.name.to_snake_case(),
                    path: ctx_source.path.clone(),
                });
            }
        }
    }

    context_fields
}

/// Generate GraphQL resolvers from a proto service
pub fn generate(
    ctx: &GeneratorContext,
    service: &Service,
) -> Result<Vec<GeneratedFile>, GeneratorError> {
    let svc_name = &service.name;

    // Check for graphql service options
    let svc_opts = service.graphql.as_ref();

    // Skip if explicitly marked
    if svc_opts.is_some_and(|o| o.skip) {
        return Ok(vec![]);
    }

    // Group methods by operation type
    let mut queries: Vec<&Method> = Vec::new();
    let mut mutations: Vec<&Method> = Vec::new();

    for method in &service.methods {
        if let Some(ref graphql_opts) = method.graphql {
            match graphql_opts.kind {
                GraphQLMethodKind::Mutation => {
                    if !graphql_opts.skip {
                        mutations.push(method);
                    }
                }
                GraphQLMethodKind::Query => {
                    if !graphql_opts.skip {
                        queries.push(method);
                    }
                }
                GraphQLMethodKind::Subscription => {
                    // Not yet supported
                }
            }
        }
        // If a method has no graphql option, it's not exposed in GraphQL
    }

    let mut files = Vec::new();
    let package_name = &ctx.package.name;

    // Generate Query struct if there are query methods
    if !queries.is_empty() {
        if let Some(query_file) = generate_query_struct(package_name, svc_name, &queries)? {
            files.push(query_file);
        }
    }

    // Generate Mutation struct if there are mutation methods
    if !mutations.is_empty() {
        if let Some(mutation_file) = generate_mutation_struct(ctx, svc_name, &mutations)? {
            files.push(mutation_file);
        }
    }

    Ok(files)
}

/// Generate a Query struct with resolver methods using gRPC client
fn generate_query_struct(
    package_name: &str,
    svc_name: &str,
    methods: &[&Method],
) -> Result<Option<GeneratedFile>, GeneratorError> {
    let query_name = format!("{}Query", svc_name.to_upper_camel_case());
    let query_ident = format_ident!("{}", query_name);

    // gRPC client type - in submodule {service}_client::{Service}Client
    let client_module = format!("{}_client", svc_name.to_snake_case());
    let client_module_ident = format_ident!("{}", client_module);
    let client_type = format!("{}Client", svc_name.to_upper_camel_case());
    let client_ident = format_ident!("{}", client_type);

    // Generate resolver methods
    let resolver_methods = generate_query_resolver_methods(svc_name, methods)?;

    let code = quote! {
        //! GraphQL Query resolvers for #svc_name
        //! @generated

        #![allow(missing_docs)]
        #![allow(unused_imports)]

        use async_graphql::{Object, Context, Result};
        use tonic::transport::Channel;
        // Import gRPC client from parent module (blog/{svc}_client)
        use super::super::#client_module_ident::#client_ident;

        /// Query resolvers from #svc_name (uses gRPC client)
        #[derive(Default)]
        pub struct #query_ident;

        #[Object]
        impl #query_ident {
            #resolver_methods
        }

        // Client type alias
        type Client = #client_ident<Channel>;
    };

    // Format the generated code
    let content = code.to_string();
    let formatted = match syn::parse_file(&content) {
        Ok(parsed) => prettyplease::unparse(&parsed),
        Err(_) => content,
    };

    // Determine output file path
    let output_path = format!(
        "{}/graphql/{}_query.rs",
        package_name.replace('.', "/"),
        svc_name.to_snake_case()
    );

    Ok(Some(GeneratedFile {
        path: output_path,
        content: formatted,
    }))
}

/// Generate a Mutation struct with resolver methods using gRPC client
fn generate_mutation_struct(
    ctx: &GeneratorContext,
    svc_name: &str,
    methods: &[&Method],
) -> Result<Option<GeneratedFile>, GeneratorError> {
    let mutation_name = format!("{}Mutation", svc_name.to_upper_camel_case());
    let mutation_ident = format_ident!("{}", mutation_name);

    // gRPC client type - in submodule {service}_client::{Service}Client
    let client_module = format!("{}_client", svc_name.to_snake_case());
    let client_module_ident = format_ident!("{}", client_module);
    let client_type = format!("{}Client", svc_name.to_upper_camel_case());
    let client_ident = format_ident!("{}", client_type);

    // Generate resolver methods
    let resolver_methods = generate_mutation_resolver_methods(ctx, svc_name, methods)?;

    let code = quote! {
        //! GraphQL Mutation resolvers for #svc_name
        //! @generated

        #![allow(missing_docs)]
        #![allow(unused_imports)]

        use async_graphql::{Object, Context, Result};
        use tonic::transport::Channel;
        // Import gRPC client from parent module (blog/{svc}_client)
        use super::super::#client_module_ident::#client_ident;

        /// Mutation resolvers from #svc_name (uses gRPC client)
        #[derive(Default)]
        pub struct #mutation_ident;

        #[Object]
        impl #mutation_ident {
            #resolver_methods
        }

        // Client type alias
        type Client = #client_ident<Channel>;
    };

    // Format the generated code
    let content = code.to_string();
    let formatted = match syn::parse_file(&content) {
        Ok(parsed) => prettyplease::unparse(&parsed),
        Err(_) => content,
    };

    // Determine output file path
    let package_name = &ctx.package.name;
    let output_path = format!(
        "{}/graphql/{}_mutation.rs",
        package_name.replace('.', "/"),
        svc_name.to_snake_case()
    );

    Ok(Some(GeneratedFile {
        path: output_path,
        content: formatted,
    }))
}

/// Generate Query resolver methods (get, list operations)
fn generate_query_resolver_methods(
    svc_name: &str,
    methods: &[&Method],
) -> Result<TokenStream, GeneratorError> {
    let mut method_tokens = Vec::new();
    let entity_name = svc_name.trim_end_matches("Service");
    let entity_snake = entity_name.to_snake_case();

    for method in methods {
        let method_name = &method.name;
        let opts = method.graphql.as_ref().unwrap(); // Caller guarantees this exists

        // Determine Rust method name (snake_case - async-graphql converts to camelCase automatically)
        let field_name = if !opts.name.is_empty() {
            opts.name.to_snake_case()
        } else {
            method_name.to_snake_case()
        };
        let field_ident = format_ident!("{}", field_name);

        // gRPC method name (snake_case)
        let grpc_method = format_ident!("{}", method_name.to_snake_case());

        // Determine if this is a list or get operation
        let is_list = method_name.to_lowercase().starts_with("list");

        // Get output type name from options or derive from method name
        let output_type = if !opts.output_type.is_empty() {
            format_ident!("{}", opts.output_type)
        } else if is_list {
            format_ident!("{}Connection", entity_name.to_upper_camel_case())
        } else {
            format_ident!("{}", entity_name.to_upper_camel_case())
        };

        // Get output field from response
        let output_field = if !opts.output_field.is_empty() {
            format_ident!("{}", opts.output_field)
        } else {
            format_ident!("{}", entity_snake)
        };

        // Get request type from IR method input_type
        let request_type = {
            let name = method.input_type.rsplit('.').next().unwrap_or(&method.input_type).to_upper_camel_case();
            format_ident!("{}", name)
        };

        let resolver = if is_list {
            // List operation - return connection with filter/orderBy support
            // Derive filter and orderBy types from entity name
            let filter_type = format_ident!("{}Filter", entity_name.to_upper_camel_case());
            let order_by_type = format_ident!("{}OrderBy", entity_name.to_upper_camel_case());

            quote! {
                async fn #field_ident(
                    &self,
                    ctx: &Context<'_>,
                    after: Option<String>,
                    before: Option<String>,
                    first: Option<i32>,
                    last: Option<i32>,
                    filter: Option<super::#filter_type>,
                    order_by: Option<super::#order_by_type>,
                ) -> Result<super::#output_type> {
                    let client = ctx.data_unchecked::<Client>();
                    let request = super::super::#request_type {
                        after,
                        before,
                        first,
                        last,
                        filter: filter.map(|f| f.into()),
                        order_by: order_by.map(|o| o.into()),
                    };
                    let response = client.clone().#grpc_method(request).await
                        .map_err(|e| async_graphql::Error::new(e.to_string()))?;
                    Ok(response.into_inner().into())
                }
            }
        } else {
            // Get operation - return single entity
            quote! {
                async fn #field_ident(
                    &self,
                    ctx: &Context<'_>,
                    id: i64,
                ) -> Result<Option<super::#output_type>> {
                    let client = ctx.data_unchecked::<Client>();
                    let request = super::super::#request_type { id };
                    match client.clone().#grpc_method(request).await {
                        Ok(response) => Ok(response.into_inner().#output_field.map(super::#output_type::from)),
                        Err(e) => {
                            if e.code() == tonic::Code::NotFound {
                                Ok(None)
                            } else {
                                Err(async_graphql::Error::new(e.to_string()))
                            }
                        }
                    }
                }
            }
        };

        method_tokens.push(resolver);
    }

    Ok(quote! { #(#method_tokens)* })
}

/// Generate Mutation resolver methods (create, update, delete operations)
fn generate_mutation_resolver_methods(
    ctx: &GeneratorContext,
    svc_name: &str,
    methods: &[&Method],
) -> Result<TokenStream, GeneratorError> {
    let mut method_tokens = Vec::new();
    let entity_name = svc_name.trim_end_matches("Service");
    let entity_snake = entity_name.to_snake_case();

    for method in methods {
        let method_name = &method.name;
        let opts = method.graphql.as_ref().unwrap(); // Caller guarantees this exists

        // Determine Rust method name (snake_case - async-graphql converts to camelCase automatically)
        let field_name = if !opts.name.is_empty() {
            opts.name.to_snake_case()
        } else {
            method_name.to_snake_case()
        };
        let field_ident = format_ident!("{}", field_name);

        // gRPC method name (snake_case)
        let grpc_method = format_ident!("{}", method_name.to_snake_case());

        // Get request type name (without package prefix)
        let request_type_name = method.input_type.rsplit('.').next().unwrap_or(&method.input_type).to_string();

        let request_type = format_ident!("{}", request_type_name.to_upper_camel_case());

        // Get output type from options
        let output_type = if !opts.output_type.is_empty() {
            format_ident!("{}", opts.output_type)
        } else {
            format_ident!("{}", entity_name.to_upper_camel_case())
        };

        // Get output field from response
        let output_field = if !opts.output_field.is_empty() {
            format_ident!("{}", opts.output_field)
        } else {
            format_ident!("{}", entity_snake)
        };

        // Determine operation type
        let is_create = method_name.to_lowercase().starts_with("create");
        let is_update = method_name.to_lowercase().starts_with("update");
        let is_delete = method_name.to_lowercase().starts_with("delete");

        // Derive input type name from request type: CreateUserRequest → CreateUserInput
        let derived_input_type = format_ident!(
            "{}",
            request_type.to_string().replace("Request", "Input")
        );

        // Check for context-injected fields in create operations
        let context_fields = if is_create {
            get_context_fields(ctx, &request_type_name)
        } else {
            Vec::new()
        };

        let resolver = if is_delete {
            // Delete operation - return bool
            quote! {
                async fn #field_ident(
                    &self,
                    ctx: &Context<'_>,
                    id: i64,
                ) -> Result<bool> {
                    let client = ctx.data_unchecked::<Client>();
                    let request = super::super::#request_type { id };
                    let response = client.clone().#grpc_method(request).await
                        .map_err(|e| async_graphql::Error::new(e.to_string()))?;
                    Ok(response.into_inner().success)
                }
            }
        } else if is_create && !context_fields.is_empty() {
            // Create operation with context-injected fields
            // Extract values from context and call to_request()
            let ctx_extractions: Vec<_> = context_fields
                .iter()
                .map(|cf| {
                    let field_ident = format_ident!("{}", cf.name);
                    let path = &cf.path;
                    // For now, generate a placeholder that extracts from CurrentUser
                    // The actual implementation depends on the context type
                    quote! {
                        let #field_ident = ctx
                            .data::<crate::CurrentUser>()
                            .map_err(|_| async_graphql::Error::new(format!("Authentication required for field '{}' (from context path: {})", stringify!(#field_ident), #path)))?
                            .id;
                    }
                })
                .collect();

            let ctx_args: Vec<_> = context_fields
                .iter()
                .map(|cf| format_ident!("{}", cf.name))
                .collect();

            quote! {
                async fn #field_ident(
                    &self,
                    ctx: &Context<'_>,
                    input: super::#derived_input_type,
                ) -> Result<super::#output_type> {
                    let client = ctx.data_unchecked::<Client>();
                    // Extract context-injected fields
                    #(#ctx_extractions)*
                    let request = input.to_request(#(#ctx_args),*);
                    let response = client.clone().#grpc_method(request).await
                        .map_err(|e| async_graphql::Error::new(e.to_string()))?;
                    Ok(response.into_inner().#output_field.map(super::#output_type::from)
                        .ok_or_else(|| async_graphql::Error::new("Failed to create"))?)
                }
            }
        } else if is_create {
            // Create operation without context fields - use From impl
            quote! {
                async fn #field_ident(
                    &self,
                    ctx: &Context<'_>,
                    input: super::#derived_input_type,
                ) -> Result<super::#output_type> {
                    let client = ctx.data_unchecked::<Client>();
                    let request: super::super::#request_type = input.into();
                    let response = client.clone().#grpc_method(request).await
                        .map_err(|e| async_graphql::Error::new(e.to_string()))?;
                    Ok(response.into_inner().#output_field.map(super::#output_type::from)
                        .ok_or_else(|| async_graphql::Error::new("Failed to create"))?)
                }
            }
        } else if is_update {
            // Update operation - id is separate, input uses to_request method
            quote! {
                async fn #field_ident(
                    &self,
                    ctx: &Context<'_>,
                    id: i64,
                    input: super::#derived_input_type,
                ) -> Result<super::#output_type> {
                    let client = ctx.data_unchecked::<Client>();
                    let request = input.to_request(id);
                    let response = client.clone().#grpc_method(request).await
                        .map_err(|e| async_graphql::Error::new(e.to_string()))?;
                    Ok(response.into_inner().#output_field.map(super::#output_type::from)
                        .ok_or_else(|| async_graphql::Error::new("Failed to update"))?)
                }
            }
        } else {
            // Generic mutation
            quote! {
                async fn #field_ident(
                    &self,
                    ctx: &Context<'_>,
                    request: super::super::#request_type,
                ) -> Result<super::#output_type> {
                    let client = ctx.data_unchecked::<Client>();
                    let response = client.clone().#grpc_method(request).await
                        .map_err(|e| async_graphql::Error::new(e.to_string()))?;
                    Ok(response.into_inner().#output_field.map(super::#output_type::from)
                        .ok_or_else(|| async_graphql::Error::new("Operation failed"))?)
                }
            }
        };

        method_tokens.push(resolver);
    }

    Ok(quote! { #(#method_tokens)* })
}
