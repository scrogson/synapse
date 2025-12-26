//! GraphQL resolver generation
//!
//! Generates async-graphql Query/Mutation resolvers from protobuf service definitions.
//! Resolvers use the storage layer directly (not gRPC clients).

use crate::error::GeneratorError;
use crate::storage::seaorm::options::{
    get_cached_graphql_method_options, get_cached_graphql_service_options,
};
use heck::{ToSnakeCase, ToUpperCamelCase};
use proc_macro2::TokenStream;
use prost_types::compiler::code_generator_response::File;
use prost_types::{FileDescriptorProto, MethodDescriptorProto, ServiceDescriptorProto};
use quote::{format_ident, quote};

/// Generate GraphQL resolvers from a proto service
pub fn generate(
    file: &FileDescriptorProto,
    service: &ServiceDescriptorProto,
) -> Result<Vec<File>, GeneratorError> {
    let file_name = file.name.as_deref().unwrap_or("");
    let svc_name = service.name.as_deref().unwrap_or("");

    // Check for graphql service options
    let svc_opts = get_cached_graphql_service_options(file_name, svc_name);

    // Skip if explicitly marked
    if svc_opts.as_ref().is_some_and(|o| o.skip) {
        return Ok(vec![]);
    }

    // Group methods by operation type
    let mut queries = Vec::new();
    let mut mutations = Vec::new();

    for method in &service.method {
        let method_name = method.name.as_deref().unwrap_or("");
        let method_opts = get_cached_graphql_method_options(file_name, svc_name, method_name);

        // Skip if marked
        if method_opts.as_ref().is_some_and(|o| o.skip) {
            continue;
        }

        // Determine operation type (default to Query)
        let operation = method_opts
            .as_ref()
            .filter(|o| !o.operation.is_empty())
            .map(|o| o.operation.as_str())
            .unwrap_or("Query");

        match operation {
            "Mutation" => mutations.push((method, method_opts)),
            _ => queries.push((method, method_opts)),
        }
    }

    let mut files = Vec::new();

    // Generate Query struct if there are query methods
    if !queries.is_empty() {
        if let Some(query_file) = generate_query_struct(file, service, &queries)? {
            files.push(query_file);
        }
    }

    // Generate Mutation struct if there are mutation methods
    if !mutations.is_empty() {
        if let Some(mutation_file) = generate_mutation_struct(file, service, &mutations)? {
            files.push(mutation_file);
        }
    }

    Ok(files)
}

/// Generate a Query struct with resolver methods using storage
fn generate_query_struct(
    file: &FileDescriptorProto,
    service: &ServiceDescriptorProto,
    methods: &[(
        &MethodDescriptorProto,
        Option<crate::options::synapse::graphql::MethodOptions>,
    )],
) -> Result<Option<File>, GeneratorError> {
    let svc_name = service.name.as_deref().unwrap_or("");
    let query_name = format!("{}Query", svc_name.to_upper_camel_case());
    let query_ident = format_ident!("{}", query_name);

    // Storage type
    let storage_type = format!("SeaOrm{}Storage", svc_name.to_upper_camel_case());
    let storage_ident = format_ident!("{}", storage_type);

    // Generate resolver methods
    let resolver_methods = generate_query_resolver_methods(svc_name, methods)?;

    // Generate storage trait name
    let storage_trait = format_ident!("{}Storage", svc_name.to_upper_camel_case());

    let code = quote! {
        //! GraphQL Query resolvers for #svc_name
        //! @generated

        #![allow(missing_docs)]
        #![allow(unused_imports)]

        use async_graphql::{Object, Context, Result};
        use std::sync::Arc;
        // Import storage type and trait from parent module (blog/)
        use super::super::#storage_ident;
        use super::super::#storage_trait;

        /// Query resolvers from #svc_name (uses storage layer)
        #[derive(Default)]
        pub struct #query_ident;

        #[Object]
        impl #query_ident {
            #resolver_methods
        }

        // Storage type alias
        type Storage = #storage_ident;
    };

    // Format the generated code
    let content = code.to_string();
    let formatted = match syn::parse_file(&content) {
        Ok(parsed) => prettyplease::unparse(&parsed),
        Err(_) => content,
    };

    // Determine output file path
    let proto_path = file.name.as_deref().unwrap_or("unknown.proto");
    let output_path = proto_path.replace(
        ".proto",
        &format!("/graphql/{}_query.rs", svc_name.to_snake_case()),
    );

    Ok(Some(File {
        name: Some(output_path),
        content: Some(formatted),
        ..Default::default()
    }))
}

/// Generate a Mutation struct with resolver methods using storage
fn generate_mutation_struct(
    file: &FileDescriptorProto,
    service: &ServiceDescriptorProto,
    methods: &[(
        &MethodDescriptorProto,
        Option<crate::options::synapse::graphql::MethodOptions>,
    )],
) -> Result<Option<File>, GeneratorError> {
    let svc_name = service.name.as_deref().unwrap_or("");
    let mutation_name = format!("{}Mutation", svc_name.to_upper_camel_case());
    let mutation_ident = format_ident!("{}", mutation_name);

    // Storage type
    let storage_type = format!("SeaOrm{}Storage", svc_name.to_upper_camel_case());
    let storage_ident = format_ident!("{}", storage_type);

    // Generate resolver methods
    let resolver_methods = generate_mutation_resolver_methods(svc_name, methods)?;

    // Generate storage trait name
    let storage_trait = format_ident!("{}Storage", svc_name.to_upper_camel_case());

    let code = quote! {
        //! GraphQL Mutation resolvers for #svc_name
        //! @generated

        #![allow(missing_docs)]
        #![allow(unused_imports)]

        use async_graphql::{Object, Context, Result};
        use std::sync::Arc;
        // Import storage type and trait from parent module (blog/)
        use super::super::#storage_ident;
        use super::super::#storage_trait;

        /// Mutation resolvers from #svc_name (uses storage layer)
        #[derive(Default)]
        pub struct #mutation_ident;

        #[Object]
        impl #mutation_ident {
            #resolver_methods
        }

        // Storage type alias
        type Storage = #storage_ident;
    };

    // Format the generated code
    let content = code.to_string();
    let formatted = match syn::parse_file(&content) {
        Ok(parsed) => prettyplease::unparse(&parsed),
        Err(_) => content,
    };

    // Determine output file path
    let proto_path = file.name.as_deref().unwrap_or("unknown.proto");
    let output_path = proto_path.replace(
        ".proto",
        &format!("/graphql/{}_mutation.rs", svc_name.to_snake_case()),
    );

    Ok(Some(File {
        name: Some(output_path),
        content: Some(formatted),
        ..Default::default()
    }))
}

/// Generate Query resolver methods (get, list operations)
fn generate_query_resolver_methods(
    svc_name: &str,
    methods: &[(
        &MethodDescriptorProto,
        Option<crate::options::synapse::graphql::MethodOptions>,
    )],
) -> Result<TokenStream, GeneratorError> {
    let mut method_tokens = Vec::new();
    let entity_name = svc_name.trim_end_matches("Service");
    let entity_snake = entity_name.to_snake_case();

    for (method, opts) in methods {
        let method_name = method.name.as_deref().unwrap_or("");

        // Determine Rust method name (snake_case - async-graphql converts to camelCase automatically)
        let field_name = opts
            .as_ref()
            .filter(|o| !o.name.is_empty())
            .map(|o| o.name.to_snake_case())
            .unwrap_or_else(|| method_name.to_snake_case());
        let field_ident = format_ident!("{}", field_name);

        // Generate description if present
        let description_attr = opts
            .as_ref()
            .filter(|o| !o.description.is_empty())
            .map(|o| {
                let desc = &o.description;
                quote! { #[doc = #desc] }
            })
            .unwrap_or_else(|| quote! {});

        // Storage method name
        let storage_method = format_ident!("{}", method_name.to_snake_case());

        // Determine if this is a list or get operation
        let is_list = method_name.to_lowercase().starts_with("list");

        // Get output type name from options or derive from method name
        let output_type = opts
            .as_ref()
            .filter(|o| !o.output_type.is_empty())
            .map(|o| format_ident!("{}", o.output_type))
            .unwrap_or_else(|| {
                if is_list {
                    format_ident!("{}Connection", entity_name.to_upper_camel_case())
                } else {
                    format_ident!("{}", entity_name.to_upper_camel_case())
                }
            });

        // Get output field from response
        let output_field = opts
            .as_ref()
            .filter(|o| !o.output_field.is_empty())
            .map(|o| format_ident!("{}", o.output_field))
            .unwrap_or_else(|| format_ident!("{}", entity_snake));

        // Get request type
        let request_type = method
            .input_type
            .as_ref()
            .map(|t| {
                let name = t.rsplit('.').next().unwrap_or(t).to_upper_camel_case();
                format_ident!("{}", name)
            })
            .unwrap_or_else(|| format_ident!("()"));

        let resolver = if is_list {
            // List operation - return connection with filter/orderBy support
            // Derive filter and orderBy types from entity name
            let filter_type = format_ident!("{}Filter", entity_name.to_upper_camel_case());
            let order_by_type = format_ident!("{}OrderBy", entity_name.to_upper_camel_case());

            quote! {
                #description_attr
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
                    let storage = ctx.data_unchecked::<Arc<Storage>>();
                    let request = super::super::#request_type {
                        after,
                        before,
                        first,
                        last,
                        filter: filter.map(|f| f.into()),
                        order_by: order_by.map(|o| o.into()),
                    };
                    let response = storage.#storage_method(request).await
                        .map_err(|e| async_graphql::Error::new(e.to_string()))?;
                    Ok(response.into())
                }
            }
        } else {
            // Get operation - return single entity
            quote! {
                #description_attr
                async fn #field_ident(
                    &self,
                    ctx: &Context<'_>,
                    id: i64,
                ) -> Result<Option<super::#output_type>> {
                    let storage = ctx.data_unchecked::<Arc<Storage>>();
                    let request = super::super::#request_type { id };
                    match storage.#storage_method(request).await {
                        Ok(response) => Ok(response.#output_field.map(super::#output_type::from)),
                        Err(e) => {
                            if e.to_string().contains("not found") {
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
    svc_name: &str,
    methods: &[(
        &MethodDescriptorProto,
        Option<crate::options::synapse::graphql::MethodOptions>,
    )],
) -> Result<TokenStream, GeneratorError> {
    let mut method_tokens = Vec::new();
    let entity_name = svc_name.trim_end_matches("Service");
    let entity_snake = entity_name.to_snake_case();

    for (method, opts) in methods {
        let method_name = method.name.as_deref().unwrap_or("");

        // Determine Rust method name (snake_case - async-graphql converts to camelCase automatically)
        let field_name = opts
            .as_ref()
            .filter(|o| !o.name.is_empty())
            .map(|o| o.name.to_snake_case())
            .unwrap_or_else(|| method_name.to_snake_case());
        let field_ident = format_ident!("{}", field_name);

        // Generate description if present
        let description_attr = opts
            .as_ref()
            .filter(|o| !o.description.is_empty())
            .map(|o| {
                let desc = &o.description;
                quote! { #[doc = #desc] }
            })
            .unwrap_or_else(|| quote! {});

        // Storage method name
        let storage_method = format_ident!("{}", method_name.to_snake_case());

        // Get request type
        let request_type = method
            .input_type
            .as_ref()
            .map(|t| {
                let name = t.rsplit('.').next().unwrap_or(t).to_upper_camel_case();
                format_ident!("{}", name)
            })
            .unwrap_or_else(|| format_ident!("()"));

        // Get input type from options
        let input_type = opts
            .as_ref()
            .filter(|o| !o.input_type.is_empty())
            .map(|o| format_ident!("{}", o.input_type));

        // Get output type from options
        let output_type = opts
            .as_ref()
            .filter(|o| !o.output_type.is_empty())
            .map(|o| format_ident!("{}", o.output_type))
            .unwrap_or_else(|| format_ident!("{}", entity_name.to_upper_camel_case()));

        // Get output field from response
        let output_field = opts
            .as_ref()
            .filter(|o| !o.output_field.is_empty())
            .map(|o| format_ident!("{}", o.output_field))
            .unwrap_or_else(|| format_ident!("{}", entity_snake));

        // Determine operation type
        let is_create = method_name.to_lowercase().starts_with("create");
        let is_update = method_name.to_lowercase().starts_with("update");
        let is_delete = method_name.to_lowercase().starts_with("delete");

        let resolver = if is_delete {
            // Delete operation - return bool
            quote! {
                #description_attr
                async fn #field_ident(
                    &self,
                    ctx: &Context<'_>,
                    id: i64,
                ) -> Result<bool> {
                    let storage = ctx.data_unchecked::<Arc<Storage>>();
                    let request = super::super::#request_type { id };
                    let response = storage.#storage_method(request).await
                        .map_err(|e| async_graphql::Error::new(e.to_string()))?;
                    Ok(response.success)
                }
            }
        } else if is_create {
            // Create operation with input
            if let Some(input) = input_type {
                quote! {
                    #description_attr
                    async fn #field_ident(
                        &self,
                        ctx: &Context<'_>,
                        input: super::#input,
                    ) -> Result<super::#output_type> {
                        let storage = ctx.data_unchecked::<Arc<Storage>>();
                        let request = super::super::#request_type {
                            input: Some(input.into()),
                        };
                        let response = storage.#storage_method(request).await
                            .map_err(|e| async_graphql::Error::new(e.to_string()))?;
                        Ok(response.#output_field.map(super::#output_type::from)
                            .ok_or_else(|| async_graphql::Error::new("Failed to create"))?)
                    }
                }
            } else {
                quote! {}
            }
        } else if is_update {
            // Update operation with id and input
            if let Some(input) = input_type {
                quote! {
                    #description_attr
                    async fn #field_ident(
                        &self,
                        ctx: &Context<'_>,
                        id: i64,
                        input: super::#input,
                    ) -> Result<super::#output_type> {
                        let storage = ctx.data_unchecked::<Arc<Storage>>();
                        let request = super::super::#request_type {
                            id,
                            input: Some(input.into()),
                        };
                        let response = storage.#storage_method(request).await
                            .map_err(|e| async_graphql::Error::new(e.to_string()))?;
                        Ok(response.#output_field.map(super::#output_type::from)
                            .ok_or_else(|| async_graphql::Error::new("Failed to update"))?)
                    }
                }
            } else {
                quote! {}
            }
        } else {
            // Generic mutation
            quote! {
                #description_attr
                async fn #field_ident(
                    &self,
                    ctx: &Context<'_>,
                    request: super::super::#request_type,
                ) -> Result<super::#output_type> {
                    let storage = ctx.data_unchecked::<Arc<Storage>>();
                    let response = storage.#storage_method(request).await
                        .map_err(|e| async_graphql::Error::new(e.to_string()))?;
                    Ok(response.#output_field.map(super::#output_type::from)
                        .ok_or_else(|| async_graphql::Error::new("Operation failed"))?)
                }
            }
        };

        method_tokens.push(resolver);
    }

    Ok(quote! { #(#method_tokens)* })
}
