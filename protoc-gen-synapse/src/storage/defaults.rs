//! Storage defaults module generation
//!
//! This module generates standalone async functions that contain the default
//! implementations for storage operations. These functions can be:
//! - Called by trait default method implementations
//! - Called by user code that overrides specific methods but wants to delegate
//!   to the default behavior

use super::seaorm::implementation::{
    generate_create_impl, generate_delete_impl, generate_get_impl, generate_list_impl,
    generate_update_impl,
};
use heck::{ToSnakeCase, ToUpperCamelCase};
use proc_macro2::TokenStream;
use prost_types::FileDescriptorProto;
use quote::{format_ident, quote};
use synapse_gen::ir::options::ValidateMessageOptions;
use synapse_gen::ir::Service;
use synapse_gen::{CodeGenerator, GeneratedFile, GeneratorContext, GeneratorError};

/// Generator that produces default storage function implementations.
pub struct StorageDefaultsGenerator;

impl CodeGenerator for StorageDefaultsGenerator {
    fn name(&self) -> &str {
        "storage_defaults"
    }

    fn generate_service(
        &self,
        ctx: &GeneratorContext,
        service: &Service,
    ) -> Result<Vec<GeneratedFile>, GeneratorError> {
        let storage_options = match &service.storage {
            Some(opts) if !opts.skip && opts.generate_storage && opts.generate_implementation => {
                opts
            }
            _ => return Ok(vec![]),
        };

        let service_name = &service.name;

        // Determine trait name
        let trait_name = if storage_options.trait_name.is_empty() {
            format!("{}Storage", service_name)
        } else {
            storage_options.trait_name.clone()
        };

        // Generate the output filename (in storage/ subdirectory)
        let module_name = format!("{}_defaults", trait_name.to_snake_case());
        let output_filename = format!(
            "{}/storage/{}.rs",
            ctx.package.name.replace('.', "/"),
            module_name
        );

        // Collect all raw files from schema for cross-file lookups
        let all_raw_files: Vec<&FileDescriptorProto> = ctx
            .schema
            .packages
            .iter()
            .flat_map(|p| p.raw_files.iter().copied())
            .collect();

        // Generate function implementations
        let functions = generate_default_functions(ctx, service, &all_raw_files)?;

        // Build doc comment
        let module_doc = format!(
            "Default implementations for {} storage operations",
            service_name
        );

        // Import the storage trait module to get StorageError
        let trait_module = format_ident!("{}", trait_name.to_snake_case());

        let code = quote! {
            #![doc = #module_doc]
            //!
            //! These standalone functions contain the default implementations for each
            //! storage operation. They can be called from:
            //! - Trait default method implementations
            //! - Custom implementations that want to delegate to the default behavior
            //!
            //! # Example
            //!
            //! ```rust,ignore
            //! impl UserServiceStorage for MyCustomStorage {
            //!     fn db(&self) -> &DatabaseConnection { &self.db }
            //!
            //!     // Override create_user with custom logic
            //!     async fn create_user(&self, request: CreateUserRequest) -> Result<CreateUserResponse, StorageError> {
            //!         // Custom validation
            //!         validate_email(&request.email)?;
            //!
            //!         // Call the default implementation
            //!         user_service_storage_defaults::create_user(self.db(), request).await
            //!     }
            //!
            //!     // All other methods use trait defaults
            //! }
            //! ```
            //!
            //! @generated

            #![allow(missing_docs)]
            #![allow(unused_imports)]

            use super::super::prelude::*;
            use super::super::entities;
            use super::#trait_module::StorageError;
            use super::conversions::ApplyUpdate;
            // PageInfo is from synapse.relay package
            use super::super::super::synapse::relay::PageInfo;
            use sea_orm::{
                ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait,
                QueryFilter, QueryOrder, Set,
            };

            #(#functions)*
        };

        // Format the generated code
        let content = code.to_string();
        let formatted = match syn::parse_file(&content) {
            Ok(parsed) => prettyplease::unparse(&parsed),
            Err(_) => content,
        };

        Ok(vec![GeneratedFile {
            path: output_filename,
            content: formatted,
        }])
    }
}

/// Generate default function implementations
fn generate_default_functions(
    ctx: &GeneratorContext,
    service: &Service,
    all_raw_files: &[&FileDescriptorProto],
) -> Result<Vec<TokenStream>, GeneratorError> {
    let mut result = Vec::new();

    for method in &service.methods {
        let storage_opts = method.storage.as_ref();

        // Skip if marked
        if storage_opts.is_some_and(|o| o.skip) {
            continue;
        }

        // Determine Rust method name
        let rust_method_name = storage_opts
            .filter(|o| !o.method_name.is_empty())
            .map(|o| o.method_name.clone())
            .unwrap_or_else(|| method.name.to_snake_case());

        // Extract entity name from method options or infer from method name
        let entity_name = storage_opts
            .filter(|o| !o.entity_name.is_empty())
            .map(|o| o.entity_name.clone())
            .unwrap_or_else(|| infer_entity_name(&method.name));

        // Get the operation type from method options or infer from method name
        let operation = storage_opts
            .filter(|o| !o.operation.is_empty())
            .map(|o| o.operation.clone())
            .unwrap_or_else(|| infer_operation(&method.name).to_string());

        // Extract input/output types - check for domain type first
        let raw_input_type = extract_type_name(&method.input_type);
        let request_type = resolve_domain_type(ctx, &raw_input_type);
        let response_type = extract_type_name(&method.output_type);

        let method_ident = format_ident!("{}", rust_method_name);
        let request_ident = format_ident!("{}", request_type);
        let response_ident = format_ident!("{}", response_type);
        let entity_module = format_ident!("{}", entity_name.to_snake_case());

        // Generate with for_standalone=true to use `db` parameter instead of `self.db`
        let method_body = match operation.as_str() {
            "get" | "Get" | "GET" => {
                generate_get_impl(&entity_module, &response_ident, true)
            }
            "list" | "List" | "LIST" => generate_list_impl(
                service.raw_file,
                &raw_input_type,
                &entity_module,
                &response_ident,
                all_raw_files,
                true,
            ),
            "create" | "Create" | "CREATE" => {
                generate_create_impl(&entity_module, &response_ident, true)
            }
            "update" | "Update" | "UPDATE" => {
                generate_update_impl(&entity_module, &response_ident, true)
            }
            "delete" | "Delete" | "DELETE" => {
                generate_delete_impl(&entity_module, &response_ident, true)
            }
            _ => {
                quote! {
                    todo!("Implement {} for {}", stringify!(#method_ident), stringify!(#entity_module))
                }
            }
        };

        // Generate doc comment
        let doc = format!(
            "Default implementation for `{}`.\n\nCan be called from custom implementations to delegate to the default behavior.",
            rust_method_name
        );

        let function_token = quote! {
            #[doc = #doc]
            pub async fn #method_ident(
                db: &DatabaseConnection,
                request: #request_ident,
            ) -> Result<#response_ident, StorageError> {
                #method_body
            }
        };

        result.push(function_token);
    }

    Ok(result)
}

/// Infer entity name from method name
fn infer_entity_name(method_name: &str) -> String {
    let name = method_name
        .strip_prefix("Get")
        .or_else(|| method_name.strip_prefix("List"))
        .or_else(|| method_name.strip_prefix("Create"))
        .or_else(|| method_name.strip_prefix("Update"))
        .or_else(|| method_name.strip_prefix("Delete"))
        .unwrap_or(method_name);

    let name = if let Some(idx) = name.find("By") {
        &name[..idx]
    } else {
        name
    };

    if method_name.starts_with("List") && name.ends_with('s') {
        name.strip_suffix('s').unwrap_or(name).to_string()
    } else {
        name.to_string()
    }
}

/// Infer operation type from method name
fn infer_operation(method_name: &str) -> &'static str {
    if method_name.starts_with("Get") {
        "get"
    } else if method_name.starts_with("List") {
        "list"
    } else if method_name.starts_with("Create") {
        "create"
    } else if method_name.starts_with("Update") {
        "update"
    } else if method_name.starts_with("Delete") {
        "delete"
    } else {
        "unknown"
    }
}

/// Resolve a message type to its domain type if one exists
fn resolve_domain_type(ctx: &GeneratorContext, message_name: &str) -> String {
    find_validate_options(ctx, message_name)
        .filter(|opts| opts.generate_conversion && !opts.name.is_empty())
        .map(|opts| opts.name.clone())
        .unwrap_or_else(|| message_name.to_string())
}

/// Find ValidateMessageOptions for a message by type name in the current package.
fn find_validate_options<'a>(
    ctx: &'a GeneratorContext,
    type_name: &str,
) -> Option<&'a ValidateMessageOptions> {
    ctx.package
        .messages
        .iter()
        .find(|m| m.name == type_name)
        .and_then(|m| m.validate.as_ref())
        .or_else(|| {
            ctx.package
                .entities
                .iter()
                .find(|e| e.name == type_name)
                .and_then(|e| e.validate.as_ref())
        })
}

/// Extract a Rust type name from a protobuf type path
fn extract_type_name(type_name: &str) -> String {
    let type_part = type_name
        .rsplit('.')
        .next()
        .unwrap_or(type_name)
        .trim_start_matches('.');

    type_part.to_upper_camel_case()
}
