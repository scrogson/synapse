//! Package module generation
//!
//! Generates the package-level module structure that wires together all generated code:
//! - tonic-generated gRPC code (via include_proto!)
//! - SeaORM entities (in entities/)
//! - Storage traits and implementations (in storage/)
//! - gRPC services (in grpc/)
//! - GraphQL module (in graphql/)

use super::options::{get_cached_entity_options, get_cached_service_options};
use crate::error::GeneratorError;
use heck::{ToSnakeCase, ToUpperCamelCase};
use proc_macro2::TokenStream;
use prost_types::compiler::code_generator_response::File;
use prost_types::FileDescriptorProto;
use quote::{format_ident, quote};

/// Information about what was generated for a package
pub struct PackageInfo {
    /// Entity modules (snake_case names)
    pub entities: Vec<String>,
    /// Service names
    pub services: Vec<String>,
}

/// Collect package information from files in the same package
pub fn collect_package_info_all_files(
    all_files: &[FileDescriptorProto],
    main_file: &FileDescriptorProto,
) -> PackageInfo {
    let main_file_name = main_file.name.as_deref().unwrap_or("");
    let main_package = main_file.package.as_deref().unwrap_or("");
    let mut info = PackageInfo {
        entities: Vec::new(),
        services: Vec::new(),
    };

    // Collect entities only from files in the SAME package
    // This prevents generating entity code for imported types from other packages
    for file in all_files {
        let file_package = file.package.as_deref().unwrap_or("");

        // Only process files from the same package
        if file_package != main_package {
            continue;
        }

        let file_name = file.name.as_deref().unwrap_or("");
        for message in &file.message_type {
            let msg_name = message.name.as_deref().unwrap_or("");
            if let Some(opts) = get_cached_entity_options(file_name, msg_name) {
                if !opts.skip {
                    info.entities.push(msg_name.to_string());
                }
            }
        }
    }

    // Collect services from main file only
    for service in &main_file.service {
        let svc_name = service.name.as_deref().unwrap_or("");
        if let Some(opts) = get_cached_service_options(main_file_name, svc_name) {
            if !opts.skip {
                info.services.push(svc_name.to_string());
            }
        }
    }

    info
}

/// Generate all package files (main mod.rs and subdirectory mod.rs files)
pub fn generate_all(
    file: &FileDescriptorProto,
    all_files: &[FileDescriptorProto],
) -> Result<Vec<File>, GeneratorError> {
    let mut files = Vec::new();

    // Generate main mod.rs
    if let Some(main_mod) = generate(file, all_files)? {
        files.push(main_mod);
    }

    // Generate entities/mod.rs
    if let Some(entities_mod) = generate_entities_mod(file, all_files)? {
        files.push(entities_mod);
    }

    // Generate storage/mod.rs
    if let Some(storage_mod) = generate_storage_mod(file, all_files)? {
        files.push(storage_mod);
    }

    // Generate grpc/mod.rs
    if let Some(grpc_mod) = generate_grpc_mod(file, all_files)? {
        files.push(grpc_mod);
    }

    // Generate storage/conversions.rs
    if let Some(conversions) = generate_conversions(file, all_files)? {
        files.push(conversions);
    }

    Ok(files)
}

/// Generate the main package mod.rs file
pub fn generate(
    file: &FileDescriptorProto,
    all_files: &[FileDescriptorProto],
) -> Result<Option<File>, GeneratorError> {
    let package = file.package.as_deref().unwrap_or("");
    if package.is_empty() {
        return Ok(None);
    }

    let info = collect_package_info_all_files(all_files, file);

    // Skip if no entities or services
    if info.entities.is_empty() && info.services.is_empty() {
        return Ok(None);
    }

    // Generate module declarations for subdirectories
    let mut mod_declarations = Vec::new();
    let mut pub_uses = Vec::new();

    // Subdirectory modules
    if !info.entities.is_empty() {
        mod_declarations.push(quote! { pub mod entities; });
    }
    if !info.services.is_empty() {
        mod_declarations.push(quote! { pub mod storage; });
        mod_declarations.push(quote! { pub mod grpc; });
    }

    // GraphQL module
    mod_declarations.push(quote! { pub mod graphql; });

    // Re-exports for entities (from entities module)
    for entity in &info.entities {
        let entity_camel = entity.to_upper_camel_case();
        let model_alias = format_ident!("{}Model", entity_camel);
        let entity_mod = format_ident!("{}", entity.to_snake_case());
        pub_uses.push(quote! {
            pub use entities::#entity_mod::Model as #model_alias;
        });
    }

    // Re-exports for storage traits and implementations
    for svc in &info.services {
        let svc_camel = svc.to_upper_camel_case();
        let trait_name = format_ident!("{}Storage", svc_camel);
        let impl_name = format_ident!("SeaOrm{}Storage", svc_camel);
        let grpc_name = format_ident!("{}GrpcService", svc_camel);

        pub_uses.push(quote! { pub use storage::#trait_name; });
        pub_uses.push(quote! { pub use storage::#impl_name; });
        pub_uses.push(quote! { pub use grpc::#grpc_name; });
    }

    let code = quote! {
        //! Package module - combines tonic and synapse generated code
        //!
        //! This module is auto-generated by protoc-gen-synapse.
        //! @generated

        #![allow(missing_docs)]
        #![allow(unused_imports)]
        #![allow(clippy::all)]
        #![allow(dead_code)]

        // Include tonic-generated gRPC code (from OUT_DIR)
        tonic::include_proto!(#package);

        // Prelude for synapse-generated code
        pub mod prelude {
            pub use sea_orm::entity::prelude::*;
            pub use sea_orm::{DatabaseConnection, DbErr};

            // Re-export proto types from parent
            pub use super::*;

            /// Storage error type
            #[derive(Debug, thiserror::Error)]
            pub enum StorageError {
                #[error("database error: {0}")]
                Database(#[from] sea_orm::DbErr),
                #[error("not found: {0}")]
                NotFound(String),
                #[error("invalid argument: {0}")]
                InvalidArgument(String),
            }
        }

        // Sub-modules
        #(#mod_declarations)*

        // Re-exports
        #(#pub_uses)*
    };

    // Format the generated code
    let content = code.to_string();
    let formatted = match syn::parse_file(&content) {
        Ok(parsed) => prettyplease::unparse(&parsed),
        Err(_) => content,
    };

    // Output path
    let output_path = format!("{}/mod.rs", package.replace('.', "/"));

    Ok(Some(File {
        name: Some(output_path),
        content: Some(formatted),
        ..Default::default()
    }))
}

/// Generate the entities/mod.rs file
pub fn generate_entities_mod(
    file: &FileDescriptorProto,
    all_files: &[FileDescriptorProto],
) -> Result<Option<File>, GeneratorError> {
    let package = file.package.as_deref().unwrap_or("");
    if package.is_empty() {
        return Ok(None);
    }

    let info = collect_package_info_all_files(all_files, file);

    if info.entities.is_empty() {
        return Ok(None);
    }

    // Generate module declarations for entities
    let mod_declarations: Vec<_> = info
        .entities
        .iter()
        .map(|entity| {
            let mod_name = format_ident!("{}", entity.to_snake_case());
            quote! { pub mod #mod_name; }
        })
        .collect();

    // Re-export Models with aliases
    let pub_uses: Vec<_> = info
        .entities
        .iter()
        .map(|entity| {
            let mod_name = format_ident!("{}", entity.to_snake_case());
            let model_alias = format_ident!("{}Model", entity.to_upper_camel_case());
            quote! { pub use #mod_name::Model as #model_alias; }
        })
        .collect();

    let code = quote! {
        //! SeaORM entity definitions
        //!
        //! This module is auto-generated by protoc-gen-synapse.
        //! @generated

        #![allow(missing_docs)]
        #![allow(unused_imports)]

        #(#mod_declarations)*

        // Re-exports
        #(#pub_uses)*
    };

    // Format the generated code
    let content = code.to_string();
    let formatted = match syn::parse_file(&content) {
        Ok(parsed) => prettyplease::unparse(&parsed),
        Err(_) => content,
    };

    // Output path
    let output_path = format!("{}/entities/mod.rs", package.replace('.', "/"));

    Ok(Some(File {
        name: Some(output_path),
        content: Some(formatted),
        ..Default::default()
    }))
}

/// Generate the storage/mod.rs file
pub fn generate_storage_mod(
    file: &FileDescriptorProto,
    all_files: &[FileDescriptorProto],
) -> Result<Option<File>, GeneratorError> {
    let package = file.package.as_deref().unwrap_or("");
    if package.is_empty() {
        return Ok(None);
    }

    let info = collect_package_info_all_files(all_files, file);

    if info.services.is_empty() {
        return Ok(None);
    }

    let mut mod_declarations = Vec::new();
    let mut pub_uses = Vec::new();

    // Storage trait modules - only export StorageError once from the first one
    let mut storage_error_exported = false;
    for svc in &info.services {
        let trait_mod = format_ident!("{}_storage", svc.to_snake_case());
        mod_declarations.push(quote! { pub mod #trait_mod; });

        let trait_name = format_ident!("{}Storage", svc.to_upper_camel_case());
        pub_uses.push(quote! { pub use #trait_mod::#trait_name; });
        if !storage_error_exported {
            pub_uses.push(quote! { pub use #trait_mod::StorageError; });
            storage_error_exported = true;
        }
    }

    // Storage implementation modules
    for svc in &info.services {
        let impl_mod = format_ident!("sea_orm_{}_storage", svc.to_snake_case());
        mod_declarations.push(quote! { pub mod #impl_mod; });

        let impl_name = format_ident!("SeaOrm{}Storage", svc.to_upper_camel_case());
        pub_uses.push(quote! { pub use #impl_mod::#impl_name; });
    }

    // Conversions module
    mod_declarations.push(quote! { pub mod conversions; });
    pub_uses.push(quote! { pub use conversions::ApplyUpdate; });

    let code = quote! {
        //! Storage traits and implementations
        //!
        //! This module is auto-generated by protoc-gen-synapse.
        //! @generated

        #![allow(missing_docs)]
        #![allow(unused_imports)]

        #(#mod_declarations)*

        // Re-exports
        #(#pub_uses)*
    };

    // Format the generated code
    let content = code.to_string();
    let formatted = match syn::parse_file(&content) {
        Ok(parsed) => prettyplease::unparse(&parsed),
        Err(_) => content,
    };

    // Output path
    let output_path = format!("{}/storage/mod.rs", package.replace('.', "/"));

    Ok(Some(File {
        name: Some(output_path),
        content: Some(formatted),
        ..Default::default()
    }))
}

/// Generate the grpc/mod.rs file
pub fn generate_grpc_mod(
    file: &FileDescriptorProto,
    all_files: &[FileDescriptorProto],
) -> Result<Option<File>, GeneratorError> {
    let package = file.package.as_deref().unwrap_or("");
    if package.is_empty() {
        return Ok(None);
    }

    let info = collect_package_info_all_files(all_files, file);

    if info.services.is_empty() {
        return Ok(None);
    }

    let mut mod_declarations = Vec::new();
    let mut pub_uses = Vec::new();

    // gRPC service modules
    for svc in &info.services {
        let svc_mod = format_ident!("{}", svc.to_snake_case());
        mod_declarations.push(quote! { pub mod #svc_mod; });

        let grpc_name = format_ident!("{}GrpcService", svc.to_upper_camel_case());
        pub_uses.push(quote! { pub use #svc_mod::#grpc_name; });
    }

    let code = quote! {
        //! gRPC service implementations
        //!
        //! This module is auto-generated by protoc-gen-synapse.
        //! @generated

        #![allow(missing_docs)]
        #![allow(unused_imports)]

        #(#mod_declarations)*

        // Re-exports
        #(#pub_uses)*
    };

    // Format the generated code
    let content = code.to_string();
    let formatted = match syn::parse_file(&content) {
        Ok(parsed) => prettyplease::unparse(&parsed),
        Err(_) => content,
    };

    // Output path
    let output_path = format!("{}/grpc/mod.rs", package.replace('.', "/"));

    Ok(Some(File {
        name: Some(output_path),
        content: Some(formatted),
        ..Default::default()
    }))
}

/// Generate the storage/conversions.rs file with all From implementations
pub fn generate_conversions(
    file: &FileDescriptorProto,
    all_files: &[FileDescriptorProto],
) -> Result<Option<File>, GeneratorError> {
    let package = file.package.as_deref().unwrap_or("");

    if package.is_empty() {
        return Ok(None);
    }

    let info = collect_package_info_all_files(all_files, file);

    if info.entities.is_empty() {
        return Ok(None);
    }

    let mut conversions = Vec::new();

    for entity in &info.entities {
        let entity_mod = format_ident!("{}", entity.to_snake_case());
        let proto_type = format_ident!("{}", entity.to_upper_camel_case());
        let create_request = format_ident!("Create{}Request", entity.to_upper_camel_case());
        let update_request = format_ident!("Update{}Request", entity.to_upper_camel_case());

        // Find the entity message across all files
        let message = all_files
            .iter()
            .flat_map(|f| f.message_type.iter())
            .find(|m| m.name.as_deref() == Some(entity));

        if let Some(msg) = message {
            // Generate Model -> Proto conversion
            let model_fields = generate_model_to_proto_fields(msg);

            conversions.push(quote! {
                /// Convert SeaORM Model to proto message
                impl From<super::super::entities::#entity_mod::Model> for #proto_type {
                    fn from(model: super::super::entities::#entity_mod::Model) -> Self {
                        Self {
                            #(#model_fields)*
                        }
                    }
                }
            });

            // Find CreateRequest message across all files
            let create_msg = all_files
                .iter()
                .flat_map(|f| f.message_type.iter())
                .find(|m| m.name.as_deref() == Some(&format!("Create{}Request", entity)));

            if let Some(create) = create_msg {
                let create_fields = generate_create_fields(create);
                conversions.push(quote! {
                    /// Convert CreateRequest to SeaORM ActiveModel
                    impl From<#create_request> for super::super::entities::#entity_mod::ActiveModel {
                        fn from(request: #create_request) -> Self {
                            use sea_orm::ActiveValue::Set;
                            Self {
                                #(#create_fields)*
                                ..Default::default()
                            }
                        }
                    }
                });
            }

            // Find UpdateRequest message across all files
            let update_msg = all_files
                .iter()
                .flat_map(|f| f.message_type.iter())
                .find(|m| m.name.as_deref() == Some(&format!("Update{}Request", entity)));

            if let Some(update) = update_msg {
                let update_fields = generate_update_fields(update, msg);
                conversions.push(quote! {
                    /// Apply UpdateRequest to SeaORM ActiveModel
                    impl ApplyUpdate<&#update_request> for super::super::entities::#entity_mod::ActiveModel {
                        fn apply_update(&mut self, request: &#update_request) {
                            use sea_orm::ActiveValue::Set;
                            #(#update_fields)*
                        }
                    }
                });
            }
        }
    }

    let code = quote! {
        //! Conversions between proto types and SeaORM entities
        //!
        //! This module is auto-generated by protoc-gen-synapse.
        //! @generated

        #![allow(missing_docs)]
        #![allow(unused_imports)]

        use super::super::prelude::*;

        /// Extension trait for applying updates to an active model
        pub trait ApplyUpdate<T> {
            fn apply_update(&mut self, input: T);
        }

        #(#conversions)*
    };

    // Format the generated code
    let content = code.to_string();
    let formatted = match syn::parse_file(&content) {
        Ok(parsed) => prettyplease::unparse(&parsed),
        Err(_) => content,
    };

    // Output path (now in storage/ subdirectory)
    let output_path = format!("{}/storage/conversions.rs", package.replace('.', "/"));

    Ok(Some(File {
        name: Some(output_path),
        content: Some(formatted),
        ..Default::default()
    }))
}

/// Generate field assignments for Model -> Proto conversion
fn generate_model_to_proto_fields(message: &prost_types::DescriptorProto) -> Vec<TokenStream> {
    let mut fields = Vec::new();

    for field in &message.field {
        let field_name = field.name.as_deref().unwrap_or("");
        let rust_field = format_ident!("{}", field_name.to_snake_case());

        // Check if this is a Timestamp field
        let is_timestamp = field
            .type_name
            .as_ref()
            .map(|t| t.contains("Timestamp"))
            .unwrap_or(false);

        if is_timestamp {
            // SeaORM uses DateTime<Utc> which has .timestamp() directly
            fields.push(quote! {
                #rust_field: Some(prost_types::Timestamp {
                    seconds: model.#rust_field.timestamp(),
                    nanos: model.#rust_field.timestamp_subsec_nanos() as i32,
                }),
            });
        } else {
            fields.push(quote! {
                #rust_field: model.#rust_field,
            });
        }
    }

    fields
}

/// Generate field assignments for CreateRequest -> ActiveModel conversion
fn generate_create_fields(message: &prost_types::DescriptorProto) -> Vec<TokenStream> {
    let mut fields = Vec::new();

    for field in &message.field {
        let field_name = field.name.as_deref().unwrap_or("");
        let rust_field = format_ident!("{}", field_name.to_snake_case());

        fields.push(quote! {
            #rust_field: Set(request.#rust_field),
        });
    }

    fields
}

/// Generate field update assignments for UpdateRequest -> ActiveModel
fn generate_update_fields(
    update_message: &prost_types::DescriptorProto,
    entity_message: &prost_types::DescriptorProto,
) -> Vec<TokenStream> {
    let mut fields = Vec::new();

    for field in &update_message.field {
        let field_name = field.name.as_deref().unwrap_or("");

        // Skip the id field - it's used to find the entity, not update it
        if field_name == "id" {
            continue;
        }

        let rust_field = format_ident!("{}", field_name.to_snake_case());

        // Check if the corresponding field in the entity is optional
        let entity_field = entity_message
            .field
            .iter()
            .find(|f| f.name.as_deref() == Some(field_name));

        let is_optional_in_entity = entity_field
            .map(|f| f.proto3_optional.unwrap_or(false))
            .unwrap_or(false);

        // For optional entity fields, wrap value in Some()
        if is_optional_in_entity {
            fields.push(quote! {
                if let Some(ref value) = request.#rust_field {
                    self.#rust_field = Set(Some(value.clone()));
                }
            });
        } else {
            fields.push(quote! {
                if let Some(ref value) = request.#rust_field {
                    self.#rust_field = Set(value.clone());
                }
            });
        }
    }

    fields
}
