//! GraphQL resolver generation
//!
//! Generates async-graphql Query/Mutation resolvers from protobuf service definitions.
//! Each RPC method specifies its own operation type via synapse.graphql.method options.

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

/// Generate a Query struct with resolver methods
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

    // Generate client type path
    let client_ident = format_ident!("{}Client", svc_name.to_upper_camel_case());

    // Generate resolver methods
    let resolver_methods = generate_resolver_methods(methods)?;

    let code = quote! {
        //! GraphQL Query resolvers for #svc_name
        //! @generated

        #![allow(missing_docs)]
        #![allow(unused_imports)]

        use async_graphql::{Object, Context, Result};
        use tonic::transport::Channel;

        /// Query resolvers from #svc_name
        pub struct #query_ident;

        #[Object]
        impl #query_ident {
            #resolver_methods
        }

        // Client type alias for convenience
        type ServiceClient = super::proto::#client_ident<Channel>;
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

/// Generate a Mutation struct with resolver methods
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

    // Generate client type path
    let client_ident = format_ident!("{}Client", svc_name.to_upper_camel_case());

    // Generate resolver methods
    let resolver_methods = generate_resolver_methods(methods)?;

    let code = quote! {
        //! GraphQL Mutation resolvers for #svc_name
        //! @generated

        #![allow(missing_docs)]
        #![allow(unused_imports)]

        use async_graphql::{Object, Context, Result};
        use tonic::transport::Channel;

        /// Mutation resolvers from #svc_name
        pub struct #mutation_ident;

        #[Object]
        impl #mutation_ident {
            #resolver_methods
        }

        // Client type alias for convenience
        type ServiceClient = super::proto::#client_ident<Channel>;
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

/// Generate resolver methods for a list of RPC methods
fn generate_resolver_methods(
    methods: &[(
        &MethodDescriptorProto,
        Option<crate::options::synapse::graphql::MethodOptions>,
    )],
) -> Result<TokenStream, GeneratorError> {
    let mut method_tokens = Vec::new();

    for (method, opts) in methods {
        let method_name = method.name.as_deref().unwrap_or("");

        // Determine GraphQL field name
        let field_name = opts
            .as_ref()
            .filter(|o| !o.name.is_empty())
            .map(|o| o.name.clone())
            .unwrap_or_else(|| method_name.to_snake_case());
        let field_ident = format_ident!("{}", field_name);

        // Generate description if present
        let description_attr = opts
            .as_ref()
            .filter(|o| !o.description.is_empty())
            .map(|o| {
                let desc = &o.description;
                quote! { #[graphql(desc = #desc)] }
            })
            .unwrap_or_else(|| quote! {});

        // Get RPC input/output types
        let rpc_input_type = method
            .input_type
            .as_ref()
            .map(|t| {
                let name = t.rsplit('.').next().unwrap_or(t).to_upper_camel_case();
                format_ident!("{}", name)
            })
            .unwrap_or_else(|| format_ident!("()"));

        let rpc_output_type = method
            .output_type
            .as_ref()
            .map(|t| {
                let name = t.rsplit('.').next().unwrap_or(t).to_upper_camel_case();
                format_ident!("{}", name)
            })
            .unwrap_or_else(|| format_ident!("()"));

        // Check for GraphQL-specific input/output type overrides
        let has_custom_input = opts
            .as_ref()
            .is_some_and(|o| !o.input_type.is_empty());
        let has_custom_output = opts
            .as_ref()
            .is_some_and(|o| !o.output_type.is_empty());
        let output_field = opts
            .as_ref()
            .filter(|o| !o.output_field.is_empty())
            .map(|o| format_ident!("{}", o.output_field));

        // GraphQL types (may differ from RPC types)
        let graphql_input_type = opts
            .as_ref()
            .filter(|o| !o.input_type.is_empty())
            .map(|o| format_ident!("{}", o.input_type))
            .unwrap_or_else(|| rpc_input_type.clone());

        let graphql_output_type = opts
            .as_ref()
            .filter(|o| !o.output_type.is_empty())
            .map(|o| format_ident!("{}", o.output_type))
            .unwrap_or_else(|| rpc_output_type.clone());

        // Generate RPC method name for the client
        let rpc_method_ident = format_ident!("{}", method_name.to_snake_case());

        // Generate resolver with appropriate wrapping/unwrapping
        let resolver = if has_custom_input && has_custom_output {
            // Custom input AND output - wrap input, unwrap output
            let extract = if let Some(field) = output_field {
                quote! { response.into_inner().#field }
            } else {
                quote! { response.into_inner().into() }
            };

            quote! {
                #description_attr
                async fn #field_ident(
                    &self,
                    ctx: &Context<'_>,
                    input: #graphql_input_type,
                ) -> Result<#graphql_output_type> {
                    let client = ctx.data_unchecked::<ServiceClient>();

                    let request = #rpc_input_type { input };
                    let response = client
                        .clone()
                        .#rpc_method_ident(request)
                        .await
                        .map_err(|e| async_graphql::Error::new(e.message()))?;

                    Ok(#extract)
                }
            }
        } else if has_custom_output {
            // Only custom output - extract field from response
            let extract = if let Some(field) = output_field {
                quote! { response.into_inner().#field }
            } else {
                quote! { response.into_inner().into() }
            };

            quote! {
                #description_attr
                async fn #field_ident(
                    &self,
                    ctx: &Context<'_>,
                    input: #graphql_input_type,
                ) -> Result<#graphql_output_type> {
                    let client = ctx.data_unchecked::<ServiceClient>();

                    let response = client
                        .clone()
                        .#rpc_method_ident(input)
                        .await
                        .map_err(|e| async_graphql::Error::new(e.message()))?;

                    Ok(#extract)
                }
            }
        } else {
            // No custom types - pass through directly
            quote! {
                #description_attr
                async fn #field_ident(
                    &self,
                    ctx: &Context<'_>,
                    input: #graphql_input_type,
                ) -> Result<#graphql_output_type> {
                    let client = ctx.data_unchecked::<ServiceClient>();

                    let response = client
                        .clone()
                        .#rpc_method_ident(input)
                        .await
                        .map_err(|e| async_graphql::Error::new(e.message()))?;

                    Ok(response.into_inner())
                }
            }
        };

        method_tokens.push(resolver);
    }

    Ok(quote! { #(#method_tokens)* })
}
