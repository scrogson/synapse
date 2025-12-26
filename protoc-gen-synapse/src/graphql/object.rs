//! GraphQL Object type generation
//!
//! Generates async-graphql Object types from protobuf message definitions.
//! Handles both output types (#[Object]) and input types (#[InputObject]).

use crate::error::GeneratorError;
use crate::options::synapse::storage::{RelationDef, RelationType};
use crate::storage::seaorm::options::{
    get_cached_entity_options, get_cached_graphql_field_options, get_cached_graphql_message_options,
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

    // Generate relation resolver methods from storage options
    let entity_opts = get_cached_entity_options(file_name, msg_name);
    let relation_resolvers = if let Some(ref entity) = entity_opts {
        generate_relation_resolvers(&type_name, &entity.relations)?
    } else {
        quote! {}
    };

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
        #[derive(Clone)]
        pub struct #type_ident {
            #struct_fields
        }

        #[Object]
        impl #type_ident {
            #node_impl
            #resolver_methods
            #relation_resolvers
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

    // Check if this type is one of the primitive filter types (to avoid self-import)
    let is_primitive_filter = matches!(
        type_name.as_str(),
        "Int64Filter" | "Int32Filter" | "StringFilter" | "BoolFilter" | "FloatFilter" | "DoubleFilter"
    );

    // Only import filter types if this isn't itself a primitive filter type
    let filter_imports = if is_primitive_filter {
        quote! {}
    } else {
        quote! {
            // Import common types from graphql module
            #[allow(unused_imports)]
            use super::{Int64Filter, StringFilter, BoolFilter, OrderDirection};
        }
    };

    // Generate From impl to convert GraphQL input to proto message
    let proto_ident = format_ident!("{}", msg_name);
    let from_impl = generate_input_from_impl(&type_ident, &proto_ident, &message.field);

    let code = quote! {
        //! GraphQL InputObject type for #msg_name
        //! @generated

        #![allow(missing_docs)]
        #![allow(unused_imports)]

        use async_graphql::InputObject;
        #filter_imports

        /// GraphQL input object type
        #[derive(InputObject, Default)]
        pub struct #type_ident {
            #struct_fields
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

/// Generate struct fields for an Object type
/// Note: All fields are included in the struct, even those marked with skip.
/// The skip option only affects resolver method generation, not struct fields.
/// This allows relation resolvers to access FK fields that aren't exposed in GraphQL.
fn generate_struct_fields(
    _file_name: &str,
    _msg_name: &str,
    fields: &[FieldDescriptorProto],
) -> Result<TokenStream, GeneratorError> {
    use prost_types::field_descriptor_proto::Label;

    let mut field_tokens = Vec::new();

    for field in fields {
        let field_name = field.name.as_deref().unwrap_or("");

        // Include ALL fields in the struct (skip only affects resolver generation)
        // Escape Rust keywords
        let snake_name = field_name.to_snake_case();
        let rust_name = escape_rust_keyword(&snake_name);
        let rust_type = proto_type_to_rust_type(field);

        // Check if optional or repeated
        let is_optional = field.proto3_optional.unwrap_or(false);
        let is_repeated = field.label() == Label::Repeated;

        let field_type = if is_repeated {
            quote! { Vec<#rust_type> }
        } else if is_optional {
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

    // Use base64url encoding for global IDs (URL-safe, works with any bytes)
    if is_uuid {
        // UUID-based ID - return as string reference
        quote! {
            /// Relay global ID
            async fn id(&self) -> ID {
                use base64::Engine;
                let raw = format!("{}:{}", #type_name_str, self.id);
                ID(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(raw.as_bytes()))
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
                use base64::Engine;
                let raw = format!("{}:{}", #type_name_str, self.id);
                ID(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(raw.as_bytes()))
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
                use base64::Engine;
                let raw = format!("{}:{}", #type_name_str, self.id);
                ID(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(raw.as_bytes()))
            }

            /// Internal database ID
            async fn internal_id(&self) -> ID {
                ID(self.id.to_string())
            }
        }
    }
}

/// Generate From impl for proto to GraphQL type conversion
/// Note: All fields are converted, including those marked with skip.
/// This allows relation resolvers to access FK fields.
fn generate_from_impl(
    _file: &FileDescriptorProto,
    message: &DescriptorProto,
    type_name: &str,
) -> Result<TokenStream, GeneratorError> {
    let msg_name = message.name.as_deref().unwrap_or("");
    let type_ident = format_ident!("{}", type_name);
    let proto_ident = format_ident!("{}", msg_name);

    let mut field_conversions = Vec::new();

    for field in &message.field {
        let field_name = field.name.as_deref().unwrap_or("");
        let rust_name = format_ident!("{}", field_name.to_snake_case());

        // Check if this is a Timestamp field (needs conversion to String)
        let is_timestamp = field
            .type_name
            .as_ref()
            .map(|t| t.contains("Timestamp"))
            .unwrap_or(false);

        let conversion = if is_timestamp {
            // Convert Timestamp to ISO 8601 string
            quote! {
                #rust_name: proto.#rust_name.map(|t| {
                    chrono::DateTime::from_timestamp(t.seconds, t.nanos as u32)
                        .map(|dt| dt.to_rfc3339())
                        .unwrap_or_default()
                }).unwrap_or_default(),
            }
        } else {
            quote! {
                #rust_name: proto.#rust_name,
            }
        };

        field_conversions.push(conversion);
    }

    Ok(quote! {
        impl From<super::super::#proto_ident> for #type_ident {
            fn from(proto: super::super::#proto_ident) -> Self {
                Self {
                    #(#field_conversions)*
                }
            }
        }
    })
}

/// Generate From impl for InputObject to proto message conversion
fn generate_input_from_impl(
    type_ident: &proc_macro2::Ident,
    proto_ident: &proc_macro2::Ident,
    fields: &[FieldDescriptorProto],
) -> TokenStream {
    let mut field_conversions = Vec::new();

    for field in fields {
        let field_name = field.name.as_deref().unwrap_or("");
        let snake_name = field_name.to_snake_case();

        // Escape Rust keywords
        let rust_name = escape_rust_keyword(&snake_name);

        // Check field type characteristics
        let proto_type = field.r#type();
        let is_optional = field.proto3_optional.unwrap_or(false);
        let is_repeated = field.label() == prost_types::field_descriptor_proto::Label::Repeated;
        let is_message = matches!(proto_type, Type::Message);
        let is_enum = matches!(proto_type, Type::Enum);

        let conversion = if is_enum && is_optional {
            // Optional enum: convert to i32 via super::super:: proto enum
            let enum_name = field
                .type_name
                .as_ref()
                .map(|t| t.rsplit('.').next().unwrap_or(t))
                .unwrap_or("");
            let enum_ident = format_ident!("{}", enum_name.to_upper_camel_case());
            quote! {
                #rust_name: input.#rust_name.map(|e| super::super::#enum_ident::from(e) as i32),
            }
        } else if is_enum {
            // Required enum: convert to i32 via super::super:: proto enum
            let enum_name = field
                .type_name
                .as_ref()
                .map(|t| t.rsplit('.').next().unwrap_or(t))
                .unwrap_or("");
            let enum_ident = format_ident!("{}", enum_name.to_upper_camel_case());
            quote! {
                #rust_name: super::super::#enum_ident::from(input.#rust_name) as i32,
            }
        } else if is_message && is_optional {
            // Optional nested type: .map(Into::into)
            quote! {
                #rust_name: input.#rust_name.map(Into::into),
            }
        } else if is_message && is_repeated {
            // Repeated nested type: .into_iter().map(Into::into).collect()
            quote! {
                #rust_name: input.#rust_name.into_iter().map(Into::into).collect(),
            }
        } else if is_message {
            // Required nested type: .into()
            quote! {
                #rust_name: input.#rust_name.into(),
            }
        } else {
            // Primitive type: direct copy
            quote! {
                #rust_name: input.#rust_name,
            }
        };

        field_conversions.push(conversion);
    }

    quote! {
        impl From<#type_ident> for super::super::#proto_ident {
            fn from(input: #type_ident) -> Self {
                Self {
                    #(#field_conversions)*
                }
            }
        }
    }
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
                // Handle Timestamp specially - convert to String in GraphQL
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

/// Generate relation resolver methods from storage entity relations
fn generate_relation_resolvers(
    parent_type: &str,
    relations: &[RelationDef],
) -> Result<TokenStream, GeneratorError> {
    let mut resolvers = Vec::new();

    for relation in relations {
        let resolver = generate_single_relation_resolver(parent_type, relation)?;
        resolvers.push(resolver);
    }

    Ok(quote! { #(#resolvers)* })
}

/// Generate a single relation resolver
fn generate_single_relation_resolver(
    _parent_type: &str,
    relation: &RelationDef,
) -> Result<TokenStream, GeneratorError> {
    let relation_name = &relation.name;
    let related_type = &relation.related;
    let foreign_key = &relation.foreign_key;

    let method_ident = format_ident!("{}", relation_name.to_snake_case());
    let related_ident = format_ident!("{}", related_type.to_upper_camel_case());
    let fk_ident = format_ident!("{}", foreign_key.to_snake_case());

    // Generate service and storage names
    let related_service = format!("{}Service", related_type.to_upper_camel_case());
    let storage_type = format!("SeaOrm{}Storage", related_service);
    let storage_ident = format_ident!("{}", storage_type);

    let relation_type = relation.r#type();

    match relation_type {
        RelationType::HasMany | RelationType::ManyToMany => {
            // HAS_MANY: User.posts - filter by foreign key
            // Storage method is list_{related_plural} e.g., list_posts
            let list_method = format_ident!("list_{}s", related_type.to_snake_case());
            let list_request = format_ident!("List{}sRequest", related_type.to_upper_camel_case());
            let connection_type = format_ident!("{}Connection", related_type.to_upper_camel_case());
            let filter_type = format_ident!("{}Filter", related_type.to_upper_camel_case());
            let int_filter = format_ident!("Int64Filter");

            // Generate storage trait name for import
            let storage_trait = format_ident!("{}Storage", related_service);

            Ok(quote! {
                /// Resolve related #relation_name
                async fn #method_ident(
                    &self,
                    ctx: &Context<'_>,
                    first: Option<i32>,
                    after: Option<String>,
                ) -> Result<super::#connection_type> {
                    // Import storage type and trait from parent module (blog/)
                    use super::super::#storage_ident;
                    use super::super::{#list_request, #filter_type, #int_filter};
                    // Import storage trait for method access
                    use super::super::#storage_trait;

                    let storage = ctx.data_unchecked::<std::sync::Arc<#storage_ident>>();
                    let mut filter = #filter_type::default();
                    filter.#fk_ident = Some(#int_filter {
                        eq: Some(self.id),
                        ..Default::default()
                    });

                    let request = #list_request {
                        after,
                        before: None,
                        first,
                        last: None,
                        filter: Some(filter),
                        order_by: None,
                    };

                    let response = storage.#list_method(request).await
                        .map_err(|e| async_graphql::Error::new(e.to_string()))?;

                    Ok(response.into())
                }
            })
        }
        RelationType::BelongsTo | RelationType::HasOne => {
            // BELONGS_TO: Post.author - get by id from foreign key
            // Storage method is get_{related} e.g., get_user
            // Response field is the related type name in snake_case e.g., response.user
            let get_method = format_ident!("get_{}", related_type.to_snake_case());
            let get_request = format_ident!("Get{}Request", related_type.to_upper_camel_case());
            let response_field = format_ident!("{}", related_type.to_snake_case());

            // Generate storage trait name for import
            let storage_trait = format_ident!("{}Storage", related_service);

            // For BELONGS_TO, we need the FK on this type, not the related type
            // The FK is on the current entity pointing to the related entity
            Ok(quote! {
                /// Resolve related #relation_name
                async fn #method_ident(
                    &self,
                    ctx: &Context<'_>,
                ) -> Result<Option<super::#related_ident>> {
                    // Import storage type and trait from parent module (blog/)
                    use super::super::#storage_ident;
                    use super::super::#get_request;
                    // Import storage trait for method access
                    use super::super::#storage_trait;

                    let storage = ctx.data_unchecked::<std::sync::Arc<#storage_ident>>();
                    let request = #get_request { id: self.#fk_ident };

                    match storage.#get_method(request).await {
                        Ok(response) => Ok(response.#response_field.map(super::#related_ident::from)),
                        Err(e) => {
                            // Return None for not found, propagate other errors
                            if e.to_string().contains("not found") {
                                Ok(None)
                            } else {
                                Err(async_graphql::Error::new(e.to_string()))
                            }
                        }
                    }
                }
            })
        }
        _ => Ok(quote! {}),
    }
}

/// Escape Rust keywords by prefixing with r#
fn escape_rust_keyword(name: &str) -> proc_macro2::Ident {
    // List of Rust keywords that need escaping
    const RUST_KEYWORDS: &[&str] = &[
        "as", "break", "const", "continue", "crate", "else", "enum", "extern",
        "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod",
        "move", "mut", "pub", "ref", "return", "self", "Self", "static", "struct",
        "super", "trait", "true", "type", "unsafe", "use", "where", "while",
        "async", "await", "dyn", "abstract", "become", "box", "do", "final",
        "macro", "override", "priv", "typeof", "unsized", "virtual", "yield", "try",
    ];

    if RUST_KEYWORDS.contains(&name) {
        format_ident!("r#{}", name)
    } else {
        format_ident!("{}", name)
    }
}
