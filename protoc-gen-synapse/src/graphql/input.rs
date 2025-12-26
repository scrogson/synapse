//! Auto-generated GraphQL input types from request messages
//!
//! Generates InputObject types from mutation request messages:
//! - CreateUserRequest → CreateUserInput (all fields)
//! - UpdateUserRequest → UpdateUserInput (all fields except id)

use crate::error::GeneratorError;
use crate::storage::seaorm::options::get_cached_graphql_method_options;
use heck::{ToSnakeCase, ToUpperCamelCase};
use prost_types::compiler::code_generator_response::File;
use prost_types::field_descriptor_proto::Type;
use prost_types::{FieldDescriptorProto, FileDescriptorProto, ServiceDescriptorProto};
use quote::{format_ident, quote};

/// Generate input types for mutation methods in a service
pub fn generate_inputs_for_service(
    file: &FileDescriptorProto,
    service: &ServiceDescriptorProto,
) -> Result<Vec<File>, GeneratorError> {
    let file_name = file.name.as_deref().unwrap_or("");
    let svc_name = service.name.as_deref().unwrap_or("");
    let mut files = Vec::new();

    for method in &service.method {
        let method_name = method.name.as_deref().unwrap_or("");
        let method_opts = get_cached_graphql_method_options(file_name, svc_name, method_name);

        // Skip if marked
        if method_opts.as_ref().is_some_and(|o| o.skip) {
            continue;
        }

        // Only process mutations (create/update operations)
        let operation = method_opts
            .as_ref()
            .filter(|o| !o.operation.is_empty())
            .map(|o| o.operation.as_str())
            .unwrap_or("Query");

        if operation != "Mutation" {
            continue;
        }

        // Check if this is a create or update operation
        let is_create = method_name.to_lowercase().starts_with("create");
        let is_update = method_name.to_lowercase().starts_with("update");

        if !is_create && !is_update {
            continue;
        }

        // Get the request message type
        let request_type_name = method
            .input_type
            .as_ref()
            .map(|t| t.rsplit('.').next().unwrap_or(t))
            .unwrap_or("");

        // Find the request message in the file
        let request_msg = file
            .message_type
            .iter()
            .find(|m| m.name.as_deref() == Some(request_type_name));

        if let Some(msg) = request_msg {
            // Generate input type name: CreateUserRequest → CreateUserInput
            let input_name = request_type_name.replace("Request", "Input");

            if let Some(input_file) =
                generate_input_type(file, msg, &input_name, is_update)?
            {
                files.push(input_file);
            }
        }
    }

    Ok(files)
}

/// Generate a GraphQL InputObject from a request message
fn generate_input_type(
    file: &FileDescriptorProto,
    message: &prost_types::DescriptorProto,
    input_name: &str,
    is_update: bool,
) -> Result<Option<File>, GeneratorError> {
    let input_ident = format_ident!("{}", input_name);
    let request_name = message.name.as_deref().unwrap_or("");
    let request_ident = format_ident!("{}", request_name);

    let mut field_tokens = Vec::new();
    let mut from_conversion_tokens = Vec::new();
    let mut self_conversion_tokens = Vec::new();

    for field in &message.field {
        let field_name = field.name.as_deref().unwrap_or("");
        let snake_name = field_name.to_snake_case();
        let field_ident = format_ident!("{}", snake_name);

        // For update operations, skip the id field (it's a separate argument)
        if is_update && field_name == "id" {
            continue;
        }

        let is_optional = field.proto3_optional.unwrap_or(false);
        let rust_type = proto_type_to_rust_type(field);

        let field_type = if is_optional {
            quote! { Option<#rust_type> }
        } else {
            quote! { #rust_type }
        };

        field_tokens.push(quote! {
            pub #field_ident: #field_type,
        });

        from_conversion_tokens.push(quote! {
            #field_ident: input.#field_ident,
        });

        self_conversion_tokens.push(quote! {
            #field_ident: self.#field_ident,
        });
    }

    // Build the From impl based on whether this is create or update
    let from_impl = if is_update {
        // Update: generate a `to_request` method that takes id
        quote! {
            impl #input_ident {
                /// Convert to proto request with the given id
                pub fn to_request(self, id: i64) -> super::super::#request_ident {
                    super::super::#request_ident {
                        id,
                        #(#self_conversion_tokens)*
                    }
                }
            }
        }
    } else {
        // Create: simple From impl
        quote! {
            impl From<#input_ident> for super::super::#request_ident {
                fn from(input: #input_ident) -> Self {
                    Self {
                        #(#from_conversion_tokens)*
                    }
                }
            }
        }
    };

    let code = quote! {
        //! Auto-generated GraphQL input type from request message
        //! @generated

        #![allow(missing_docs)]
        #![allow(unused_imports)]

        use async_graphql::InputObject;

        /// GraphQL input type (auto-generated from request message)
        #[derive(InputObject, Default, Clone)]
        pub struct #input_ident {
            #(#field_tokens)*
        }

        #from_impl
    };

    // Format the generated code
    let content = code.to_string();
    let formatted = match syn::parse_file(&content) {
        Ok(parsed) => prettyplease::unparse(&parsed),
        Err(_) => content,
    };

    // Determine output file path
    let package = file.package.as_deref().unwrap_or("");
    let output_path = format!(
        "{}/graphql/{}.rs",
        package.replace('.', "/"),
        input_name.to_snake_case()
    );

    Ok(Some(File {
        name: Some(output_path),
        content: Some(formatted),
        ..Default::default()
    }))
}

/// Convert proto field type to Rust type
fn proto_type_to_rust_type(field: &FieldDescriptorProto) -> proc_macro2::TokenStream {
    let proto_type = field.r#type();

    match proto_type {
        Type::Double => quote! { f64 },
        Type::Float => quote! { f32 },
        Type::Int64 | Type::Sfixed64 | Type::Sint64 => quote! { i64 },
        Type::Uint64 | Type::Fixed64 => quote! { u64 },
        Type::Int32 | Type::Sfixed32 | Type::Sint32 => quote! { i32 },
        Type::Uint32 | Type::Fixed32 => quote! { u32 },
        Type::Bool => quote! { bool },
        Type::String => quote! { String },
        Type::Bytes => quote! { Vec<u8> },
        Type::Message | Type::Enum | Type::Group => {
            if let Some(type_name) = field.type_name.as_ref() {
                if type_name.contains("Timestamp") {
                    return quote! { String };
                }
                let name = type_name
                    .rsplit('.')
                    .next()
                    .unwrap_or(type_name)
                    .to_upper_camel_case();
                let ident = format_ident!("{}", name);
                quote! { #ident }
            } else {
                quote! { () }
            }
        }
    }
}
