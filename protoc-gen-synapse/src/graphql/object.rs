//! GraphQL Object type generation
//!
//! Generates async-graphql Object types from protobuf message definitions.
//! Handles both output types (#[Object]) and input types (#[InputObject]).

use crate::error::GeneratorError;
use crate::storage::seaorm::options::{
    get_cached_graphql_field_options, get_cached_graphql_message_options,
};
use heck::{ToSnakeCase, ToUpperCamelCase};
use proc_macro2::TokenStream;
use prost_types::compiler::code_generator_response::File;
use prost_types::field_descriptor_proto::Type;
use prost_types::{DescriptorProto, FieldDescriptorProto, FileDescriptorProto};
use quote::{format_ident, quote};

/// Generate a GraphQL Object type from a proto message
pub fn generate(
    file: &FileDescriptorProto,
    message: &DescriptorProto,
) -> Result<Option<File>, GeneratorError> {
    let file_name = file.name.as_deref().unwrap_or("");
    let msg_name = message.name.as_deref().unwrap_or("");

    // Check for graphql message options
    let msg_opts = get_cached_graphql_message_options(file_name, msg_name);

    // Skip if no graphql options (only generate annotated messages)
    if msg_opts.is_none() {
        return Ok(None);
    }

    let opts = msg_opts.unwrap();

    // Skip if explicitly marked
    if opts.skip {
        return Ok(None);
    }

    // Determine if this is an input type
    if opts.input_type {
        return generate_input_type(file, message, &opts);
    }

    // Generate output object type
    generate_object_type(file, message, &opts)
}

/// Generate an async-graphql #[Object] type
fn generate_object_type(
    file: &FileDescriptorProto,
    message: &DescriptorProto,
    opts: &crate::options::synapse::graphql::MessageOptions,
) -> Result<Option<File>, GeneratorError> {
    let file_name = file.name.as_deref().unwrap_or("");
    let msg_name = message.name.as_deref().unwrap_or("");

    // Determine type name
    let type_name = if opts.type_name.is_empty() {
        msg_name.to_upper_camel_case()
    } else {
        opts.type_name.clone()
    };

    let type_ident = format_ident!("{}", type_name);

    // Generate struct fields
    let struct_fields = generate_struct_fields(file_name, msg_name, &message.field)?;

    // Generate resolver methods
    let resolver_methods =
        generate_resolver_methods(file_name, msg_name, &message.field, opts.node)?;

    // Generate From impl for proto conversion
    let from_impl = generate_from_impl(file, message, &type_name)?;

    // Check if this implements Node interface
    let node_impl = if opts.node {
        // Find the id field to get its type
        let id_field = message.field.iter().find(|f| f.name.as_deref() == Some("id"));
        generate_node_methods(&type_name, id_field)
    } else {
        quote! {}
    };

    let code = quote! {
        //! GraphQL Object type for #msg_name
        //! @generated

        #![allow(missing_docs)]
        #![allow(unused_imports)]

        use async_graphql::{Object, Context, Result, ID};
        use async_graphql::dataloader::DataLoader;

        /// GraphQL object type
        pub struct #type_ident {
            #struct_fields
        }

        #[Object]
        impl #type_ident {
            #node_impl
            #resolver_methods
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
    let proto_path = file.name.as_deref().unwrap_or("unknown.proto");
    let output_path = proto_path.replace(
        ".proto",
        &format!("/graphql/{}.rs", type_name.to_snake_case()),
    );

    Ok(Some(File {
        name: Some(output_path),
        content: Some(formatted),
        ..Default::default()
    }))
}

/// Generate an async-graphql #[InputObject] type
fn generate_input_type(
    file: &FileDescriptorProto,
    message: &DescriptorProto,
    opts: &crate::options::synapse::graphql::MessageOptions,
) -> Result<Option<File>, GeneratorError> {
    let file_name = file.name.as_deref().unwrap_or("");
    let msg_name = message.name.as_deref().unwrap_or("");

    // Determine type name
    let type_name = if opts.type_name.is_empty() {
        msg_name.to_upper_camel_case()
    } else {
        opts.type_name.clone()
    };

    let type_ident = format_ident!("{}", type_name);

    // Generate struct fields for input type
    let struct_fields = generate_input_fields(file_name, msg_name, &message.field)?;

    let code = quote! {
        //! GraphQL InputObject type for #msg_name
        //! @generated

        #![allow(missing_docs)]
        #![allow(unused_imports)]

        use async_graphql::InputObject;

        /// GraphQL input object type
        #[derive(InputObject)]
        pub struct #type_ident {
            #struct_fields
        }
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
        &format!("/graphql/{}.rs", type_name.to_snake_case()),
    );

    Ok(Some(File {
        name: Some(output_path),
        content: Some(formatted),
        ..Default::default()
    }))
}

/// Generate struct fields for an Object type
fn generate_struct_fields(
    file_name: &str,
    msg_name: &str,
    fields: &[FieldDescriptorProto],
) -> Result<TokenStream, GeneratorError> {
    let mut field_tokens = Vec::new();

    for field in fields {
        let field_name = field.name.as_deref().unwrap_or("");
        let field_number = field.number.unwrap_or(0);

        // Check for graphql field options
        let field_opts = get_cached_graphql_field_options(file_name, msg_name, field_number);

        // Skip if marked
        if field_opts.as_ref().is_some_and(|o| o.skip) {
            continue;
        }

        let rust_name = format_ident!("{}", field_name.to_snake_case());
        let rust_type = proto_type_to_rust_type(field);

        // Check if optional
        let is_optional = field.proto3_optional.unwrap_or(false);

        let field_type = if is_optional {
            quote! { Option<#rust_type> }
        } else {
            quote! { #rust_type }
        };

        field_tokens.push(quote! {
            pub #rust_name: #field_type,
        });
    }

    Ok(quote! { #(#field_tokens)* })
}

/// Generate struct fields for an InputObject type
fn generate_input_fields(
    file_name: &str,
    msg_name: &str,
    fields: &[FieldDescriptorProto],
) -> Result<TokenStream, GeneratorError> {
    // Same as struct fields for now
    generate_struct_fields(file_name, msg_name, fields)
}

/// Generate resolver methods for an Object type
fn generate_resolver_methods(
    file_name: &str,
    msg_name: &str,
    fields: &[FieldDescriptorProto],
    is_node: bool,
) -> Result<TokenStream, GeneratorError> {
    let mut method_tokens = Vec::new();

    for field in fields {
        let field_name = field.name.as_deref().unwrap_or("");
        let field_number = field.number.unwrap_or(0);

        // Check for graphql field options
        let field_opts = get_cached_graphql_field_options(file_name, msg_name, field_number);

        // Skip if marked
        if field_opts.as_ref().is_some_and(|o| o.skip) {
            continue;
        }

        // Skip 'id' field if this type implements Node interface (Node provides id() method)
        if is_node && field_name == "id" {
            continue;
        }

        // Determine method name (use override if present)
        let method_name = field_opts
            .as_ref()
            .filter(|o| !o.name.is_empty())
            .map(|o| o.name.clone())
            .unwrap_or_else(|| field_name.to_snake_case());

        let method_ident = format_ident!("{}", method_name);

        // Check if optional
        let is_optional = field.proto3_optional.unwrap_or(false);

        // Generate deprecation attribute if present
        let deprecated_attr = if let Some(ref opts) = field_opts {
            if let Some(ref dep) = opts.deprecated {
                let reason = &dep.reason;
                quote! { #[graphql(deprecation = #reason)] }
            } else {
                quote! {}
            }
        } else {
            quote! {}
        };

        // Determine return type based on field type
        let return_type = proto_type_to_resolver_return_type(field, is_optional);

        // Generate method body
        let method_body = generate_field_resolver_body(field, is_optional);

        method_tokens.push(quote! {
            #deprecated_attr
            async fn #method_ident(&self) -> #return_type {
                #method_body
            }
        });
    }

    Ok(quote! { #(#method_tokens)* })
}

/// Generate Node interface methods (global ID)
fn generate_node_methods(type_name: &str, id_field: Option<&FieldDescriptorProto>) -> TokenStream {
    let type_name_str = type_name;

    // Determine ID type category for proper handling
    let (proto_type, field_type_name) = id_field
        .map(|f| (f.r#type(), f.type_name.as_deref().unwrap_or("")))
        .unwrap_or((Type::Int64, ""));

    let is_string_type = matches!(proto_type, Type::String);

    // Check if this is a UUID type (by type name or field type name convention)
    let is_uuid = field_type_name.to_lowercase().contains("uuid");

    if is_uuid {
        // UUID-based ID - return as string reference
        quote! {
            /// Relay global ID
            async fn id(&self) -> ID {
                let raw = format!("{}:{}", #type_name_str, self.id);
                ID(base62::encode(raw.as_bytes()))
            }

            /// Internal database ID (UUID)
            async fn internal_id(&self) -> ID {
                ID(self.id.clone())
            }
        }
    } else if is_string_type {
        // String-based ID - clone directly into ID
        quote! {
            /// Relay global ID
            async fn id(&self) -> ID {
                let raw = format!("{}:{}", #type_name_str, self.id);
                ID(base62::encode(raw.as_bytes()))
            }

            /// Internal database ID
            async fn internal_id(&self) -> ID {
                ID(self.id.clone())
            }
        }
    } else {
        // Numeric ID - convert to string
        quote! {
            /// Relay global ID
            async fn id(&self) -> ID {
                let raw = format!("{}:{}", #type_name_str, self.id);
                ID(base62::encode(raw.as_bytes()))
            }

            /// Internal database ID
            async fn internal_id(&self) -> ID {
                ID(self.id.to_string())
            }
        }
    }
}

/// Generate From impl for proto to GraphQL type conversion
fn generate_from_impl(
    file: &FileDescriptorProto,
    message: &DescriptorProto,
    type_name: &str,
) -> Result<TokenStream, GeneratorError> {
    let file_name = file.name.as_deref().unwrap_or("");
    let msg_name = message.name.as_deref().unwrap_or("");
    let type_ident = format_ident!("{}", type_name);
    let proto_ident = format_ident!("{}", msg_name);

    let mut field_conversions = Vec::new();

    for field in &message.field {
        let field_name = field.name.as_deref().unwrap_or("");
        let field_number = field.number.unwrap_or(0);

        // Check for graphql field options
        let field_opts = get_cached_graphql_field_options(file_name, msg_name, field_number);

        // Skip if marked
        if field_opts.as_ref().is_some_and(|o| o.skip) {
            continue;
        }

        let rust_name = format_ident!("{}", field_name.to_snake_case());

        field_conversions.push(quote! {
            #rust_name: proto.#rust_name,
        });
    }

    Ok(quote! {
        impl From<proto::#proto_ident> for #type_ident {
            fn from(proto: proto::#proto_ident) -> Self {
                Self {
                    #(#field_conversions)*
                }
            }
        }
    })
}

/// Convert proto field type to Rust type
fn proto_type_to_rust_type(field: &FieldDescriptorProto) -> TokenStream {
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
            // For message types, use the type name
            if let Some(type_name) = field.type_name.as_ref() {
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

/// Convert proto field type to resolver return type
fn proto_type_to_resolver_return_type(
    field: &FieldDescriptorProto,
    is_optional: bool,
) -> TokenStream {
    let base_type = proto_type_to_rust_type(field);

    // Check for string types that should return references
    let proto_type = field.r#type();
    let is_string = matches!(proto_type, Type::String);

    if is_optional {
        if is_string {
            quote! { Option<&str> }
        } else {
            quote! { Option<#base_type> }
        }
    } else if is_string {
        quote! { &str }
    } else {
        quote! { #base_type }
    }
}

/// Generate field resolver body
fn generate_field_resolver_body(field: &FieldDescriptorProto, is_optional: bool) -> TokenStream {
    let field_name = field.name.as_deref().unwrap_or("");
    let field_ident = format_ident!("{}", field_name.to_snake_case());

    let proto_type = field.r#type();
    let is_string = matches!(proto_type, Type::String);

    if is_optional && is_string {
        quote! { self.#field_ident.as_deref() }
    } else if is_string {
        quote! { &self.#field_ident }
    } else if is_optional {
        quote! { self.#field_ident }
    } else {
        quote! { self.#field_ident.clone() }
    }
}
