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
use super::seaorm::options::{
    get_cached_entity_options, get_cached_rpc_method_options, get_cached_service_options,
    get_cached_validate_message_options, storage,
};
use crate::error::GeneratorError;
use heck::{ToSnakeCase, ToUpperCamelCase};
use proc_macro2::TokenStream;
use prost_types::compiler::code_generator_response::File;
use prost_types::{FileDescriptorProto, MethodDescriptorProto, ServiceDescriptorProto};
use quote::{format_ident, quote};

/// Generate a defaults module with standalone functions from a protobuf service
pub fn generate(
    file: &FileDescriptorProto,
    service: &ServiceDescriptorProto,
    all_files: &[FileDescriptorProto],
) -> Result<Option<File>, GeneratorError> {
    let file_name = file.name.as_deref().unwrap_or("");
    let service_name = service.name.as_deref().unwrap_or("");

    // Check if this service has storage options with generate_implementation
    let service_options = match get_cached_service_options(file_name, service_name) {
        Some(opts) => opts,
        None => return Ok(None),
    };

    // Skip if explicitly marked or if generate_storage is false
    if service_options.skip || !service_options.generate_storage {
        return Ok(None);
    }

    // Skip if generate_implementation is false
    if !service_options.generate_implementation {
        return Ok(None);
    }

    // Determine trait name
    let trait_name = if service_options.trait_name.is_empty() {
        format!("{}Storage", service_name)
    } else {
        service_options.trait_name.clone()
    };

    // Generate the output filename (in storage/ subdirectory)
    let module_name = format!("{}_defaults", trait_name.to_snake_case());
    let output_filename = format!(
        "{}/storage/{}.rs",
        file.package.as_deref().unwrap_or("").replace('.', "/"),
        module_name
    );

    // Generate function implementations
    let functions = generate_default_functions(file, service, all_files)?;

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

    // Try to format with prettyplease if we can parse it
    let formatted = match syn::parse_file(&content) {
        Ok(parsed) => prettyplease::unparse(&parsed),
        Err(_) => content, // If parsing fails, use unformatted
    };

    Ok(Some(File {
        name: Some(output_filename),
        content: Some(formatted),
        ..Default::default()
    }))
}

/// Generate default function implementations
fn generate_default_functions(
    file: &FileDescriptorProto,
    service: &ServiceDescriptorProto,
    all_files: &[FileDescriptorProto],
) -> Result<Vec<TokenStream>, GeneratorError> {
    let file_name = file.name.as_deref().unwrap_or("");
    let service_name = service.name.as_deref().unwrap_or("");
    let mut result = Vec::new();

    for method in &service.method {
        let method_name = method.name.as_deref().unwrap_or("");

        // Check for method-level options
        let method_options = get_cached_rpc_method_options(file_name, service_name, method_name);

        // Skip if marked
        if method_options.as_ref().map(|o| o.skip).unwrap_or(false) {
            continue;
        }

        // Determine Rust method name
        let rust_method_name = method_options
            .as_ref()
            .filter(|o| !o.method_name.is_empty())
            .map(|o| o.method_name.clone())
            .unwrap_or_else(|| method_name.to_snake_case());

        // Extract entity name from method options or infer from method name
        let entity_name = method_options
            .as_ref()
            .filter(|o| !o.entity_name.is_empty())
            .map(|o| o.entity_name.clone())
            .unwrap_or_else(|| infer_entity_name(method_name));

        // Generate the function implementation
        let function_impl = generate_function_impl(
            file,
            method,
            &rust_method_name,
            &entity_name,
            &method_options,
            all_files,
        )?;

        if let Some(impl_tokens) = function_impl {
            result.push(impl_tokens);
        }
    }

    Ok(result)
}

/// Generate a standalone function implementation
fn generate_function_impl(
    file: &FileDescriptorProto,
    method: &MethodDescriptorProto,
    rust_method_name: &str,
    entity_name: &str,
    method_options: &Option<storage::MethodOptions>,
    all_files: &[FileDescriptorProto],
) -> Result<Option<TokenStream>, GeneratorError> {
    let file_name = file.name.as_deref().unwrap_or("");

    // Get the operation type from method options or infer from method name
    let method_name = method.name.as_deref().unwrap_or("");
    let operation = method_options
        .as_ref()
        .filter(|o| !o.operation.is_empty())
        .map(|o| o.operation.as_str())
        .unwrap_or_else(|| infer_operation(method_name));

    // Extract input/output types - check for domain type first
    let raw_input_type = extract_type_name(method.input_type.as_deref());
    let request_type = resolve_domain_type(file_name, &raw_input_type);
    let response_type = extract_type_name(method.output_type.as_deref());

    let method_ident = format_ident!("{}", rust_method_name);
    let request_ident = format_ident!("{}", request_type);
    let response_ident = format_ident!("{}", response_type);
    let entity_module = format_ident!("{}", entity_name.to_snake_case());

    // Check if we have entity options for this entity
    let entity_options = get_cached_entity_options(file_name, &entity_name.to_upper_camel_case());

    // Generate with for_standalone=true to use `db` parameter instead of `self.db`
    let method_body = match operation {
        "get" | "Get" | "GET" => {
            generate_get_impl(&entity_module, &response_ident, entity_options.as_ref(), true)
        }
        "list" | "List" | "LIST" => {
            generate_list_impl(file, &request_type, &entity_module, &response_ident, all_files, true)
        }
        "create" | "Create" | "CREATE" => {
            generate_create_impl(&entity_module, &response_ident, entity_options.as_ref(), true)
        }
        "update" | "Update" | "UPDATE" => {
            generate_update_impl(&entity_module, &response_ident, entity_options.as_ref(), true)
        }
        "delete" | "Delete" | "DELETE" => generate_delete_impl(&entity_module, &response_ident, true),
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

    Ok(Some(function_token))
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

/// Extract a Rust type name from a protobuf type path
fn extract_type_name(type_name: Option<&str>) -> String {
    match type_name {
        Some(name) => {
            let type_part = name
                .rsplit('.')
                .next()
                .unwrap_or(name)
                .trim_start_matches('.');

            type_part.to_upper_camel_case()
        }
        None => "()".to_string(),
    }
}

/// Resolve a message type to its domain type if one exists
///
/// If the message has synapse.validate.message options with generate_conversion=true
/// and a non-empty name, that name is used as the domain type. Otherwise, the
/// original message name is returned.
fn resolve_domain_type(file_name: &str, message_name: &str) -> String {
    if let Some(opts) = get_cached_validate_message_options(file_name, message_name) {
        if opts.generate_conversion && !opts.name.is_empty() {
            return opts.name.clone();
        }
    }
    message_name.to_string()
}
