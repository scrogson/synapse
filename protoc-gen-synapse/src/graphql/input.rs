//! Auto-generated GraphQL input types from request messages
//!
//! Generates InputObject types from mutation request messages:
//! - CreateUserRequest → CreateUserInput (all fields except context-injected)
//! - UpdateUserRequest → UpdateUserInput (all fields except id and context-injected)
//!
//! Fields marked with `from_context` are excluded from the GraphQL input
//! and populated server-side from the authentication context.

use heck::{ToSnakeCase, ToUpperCamelCase};
use prost_types::field_descriptor_proto::Type;
use prost_types::FieldDescriptorProto;
use quote::{format_ident, quote};
use synapse_gen::ir::options::GraphQLMethodKind;
use synapse_gen::ir::Service;
use synapse_gen::{GeneratedFile, GeneratorContext, GeneratorError};

/// Generate input types for mutation methods in a service
pub fn generate_inputs_for_service(
    ctx: &GeneratorContext,
    service: &Service,
) -> Result<Vec<GeneratedFile>, GeneratorError> {
    let mut files = Vec::new();

    for method in &service.methods {
        // Only process mutations (methods with graphql option and Mutation kind)
        let Some(ref graphql_opts) = method.graphql else {
            continue;
        };

        if graphql_opts.kind != GraphQLMethodKind::Mutation || graphql_opts.skip {
            continue;
        }

        let method_name = &method.name;

        // Check if this is a create or update operation
        let is_create = method_name.to_lowercase().starts_with("create");
        let is_update = method_name.to_lowercase().starts_with("update");

        if !is_create && !is_update {
            continue;
        }

        // Get the request message type name
        let request_type_name = method.input_type.rsplit('.').next().unwrap_or(&method.input_type);

        // Find the request message in the package's messages
        let request_msg = ctx
            .package
            .messages
            .iter()
            .find(|m| m.name == request_type_name);

        if let Some(msg) = request_msg {
            // Generate input type name: CreateUserRequest → CreateUserInput
            let input_name = request_type_name.replace("Request", "Input");

            if let Some(input_file) =
                generate_input_type(ctx, msg, &input_name, is_update)?
            {
                files.push(input_file);
            }
        }
    }

    Ok(files)
}

/// Information about a context-injected field
struct ContextField {
    /// Field name (snake_case)
    name: String,
    /// Rust type for the field
    rust_type: proc_macro2::TokenStream,
    /// Context path (e.g., "current_user.id")
    path: String,
    /// Whether the field is required (for future error handling)
    #[allow(dead_code)]
    required: bool,
}

/// Generate a GraphQL InputObject from a request message
fn generate_input_type(
    ctx: &GeneratorContext,
    message: &synapse_gen::ir::Message,
    input_name: &str,
    is_update: bool,
) -> Result<Option<GeneratedFile>, GeneratorError> {
    let package_name = &ctx.package.name;
    let msg_name = &message.name;

    let input_ident = format_ident!("{}", input_name);
    let request_ident = format_ident!("{}", msg_name);

    let mut field_tokens = Vec::new();
    let mut from_conversion_tokens = Vec::new();
    let mut self_conversion_tokens = Vec::new();
    let mut context_fields: Vec<ContextField> = Vec::new();

    for ir_field in &message.fields {
        let field_name = &ir_field.name;
        let snake_name = field_name.to_snake_case();
        let field_ident = format_ident!("{}", snake_name);
        let raw_field = ir_field.raw;

        // For update operations, skip the id field (it's a separate argument)
        if is_update && field_name == "id" {
            continue;
        }

        // Check for from_context option - these fields are excluded from input
        // and populated server-side
        if let Some(ref graphql_opts) = ir_field.graphql {
            if let Some(ref ctx_source) = graphql_opts.from_context {
                // Track this as a context-injected field
                context_fields.push(ContextField {
                    name: snake_name.clone(),
                    rust_type: proto_type_to_rust_type(raw_field),
                    path: ctx_source.path.clone(),
                    required: ctx_source.required,
                });
                continue; // Skip from input type
            }
        }

        let is_optional = raw_field.proto3_optional.unwrap_or(false);
        let rust_type = proto_type_to_rust_type(raw_field);

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

    // Build the conversion impl based on whether we have context fields
    let from_impl = if !context_fields.is_empty() {
        // Has context fields - generate to_request method with context params
        let ctx_params: Vec<_> = context_fields
            .iter()
            .map(|cf| {
                let name = format_ident!("{}", cf.name);
                let ty = &cf.rust_type;
                quote! { #name: #ty }
            })
            .collect();

        let ctx_fields_assign: Vec<_> = context_fields
            .iter()
            .map(|cf| {
                let name = format_ident!("{}", cf.name);
                quote! { #name }
            })
            .collect();

        // Generate doc comment showing context paths
        let ctx_doc: Vec<_> = context_fields
            .iter()
            .map(|cf| format!("- `{}`: from context path `{}`", cf.name, cf.path))
            .collect();
        let ctx_doc_str = ctx_doc.join("\n    /// ");

        if is_update {
            quote! {
                impl #input_ident {
                    /// Convert to proto request with the given id and context values
                    ///
                    /// Context-injected fields:
                    #[doc = #ctx_doc_str]
                    pub fn to_request(self, id: i64, #(#ctx_params),*) -> super::super::#request_ident {
                        super::super::#request_ident {
                            id,
                            #(#ctx_fields_assign,)*
                            #(#self_conversion_tokens)*
                        }
                    }
                }
            }
        } else {
            quote! {
                impl #input_ident {
                    /// Convert to proto request with context values
                    ///
                    /// Context-injected fields:
                    #[doc = #ctx_doc_str]
                    pub fn to_request(self, #(#ctx_params),*) -> super::super::#request_ident {
                        super::super::#request_ident {
                            #(#ctx_fields_assign,)*
                            #(#self_conversion_tokens)*
                        }
                    }
                }
            }
        }
    } else if is_update {
        // Update without context fields: generate to_request with just id
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
        // Create without context fields: simple From impl
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
    let output_path = format!(
        "{}/graphql/{}.rs",
        package_name.replace('.', "/"),
        input_name.to_snake_case()
    );

    Ok(Some(GeneratedFile {
        path: output_path,
        content: formatted,
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
