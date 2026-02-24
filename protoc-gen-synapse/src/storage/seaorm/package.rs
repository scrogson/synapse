//! Package module generation
//!
//! Generates the package-level module structure that wires together all generated code:
//! - tonic-generated gRPC codes (via include_proto!)
//! - SeaORM entities (in entities/)
//! - Storage traits and implementations (in storage/)
//! - gRPC services (in grpc/)
//! - GraphQL module (in graphql/)

use heck::{ToSnakeCase, ToUpperCamelCase};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use synapse_gen::{CodeGenerator, GeneratedFile, GeneratorContext, GeneratorError};

/// Information about what was generated for a package
pub struct PackageInfo {
    /// Entity modules (snake_case names)
    pub entities: Vec<String>,
    /// Service names
    pub services: Vec<String>,
    /// Domain types (validated request types)
    pub domain_types: Vec<String>,
}

/// Package-level code generator using synapse-gen IR.
pub struct PackageGenerator;

impl PackageGenerator {
    /// Collect package information from the IR context
    fn collect_package_info(ctx: &GeneratorContext) -> PackageInfo {
        let entities: Vec<String> = ctx
            .package
            .entities
            .iter()
            .filter(|e| !e.skip)
            .map(|e| e.name.clone())
            .collect();

        let services: Vec<String> = ctx
            .package
            .services
            .iter()
            .filter(|s| s.storage.as_ref().is_some_and(|o| !o.skip))
            .map(|s| s.name.clone())
            .collect();

        // Collect domain types from both entities and messages
        let mut domain_types: Vec<String> = Vec::new();

        for entity in &ctx.package.entities {
            if let Some(ref v) = entity.validate {
                if v.generate_conversion && !v.name.is_empty() {
                    domain_types.push(v.name.clone());
                }
            }
        }

        for message in &ctx.package.messages {
            if let Some(ref v) = message.validate {
                if v.generate_conversion && !v.name.is_empty() {
                    domain_types.push(v.name.clone());
                }
            }
        }

        PackageInfo {
            entities,
            services,
            domain_types,
        }
    }

    /// Generate the main package mod.rs file
    fn generate_main_mod(
        ctx: &GeneratorContext,
        info: &PackageInfo,
    ) -> Result<Option<GeneratedFile>, GeneratorError> {
        let package = &ctx.package.name;
        if package.is_empty() {
            return Ok(None);
        }

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

        // Domain type modules (validated request types)
        for domain_type in &info.domain_types {
            let mod_name = format_ident!("{}", domain_type.to_snake_case());
            mod_declarations.push(quote! { pub mod #mod_name; });
        }

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

        // Re-exports for domain types (validated request types)
        for domain_type in &info.domain_types {
            let mod_name = format_ident!("{}", domain_type.to_snake_case());
            let type_name = format_ident!("{}", domain_type.to_upper_camel_case());
            pub_uses.push(quote! { pub use #mod_name::#type_name; });
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

        Ok(Some(GeneratedFile {
            path: output_path,
            content: formatted,
        }))
    }

    /// Generate the entities/mod.rs file
    fn generate_entities_mod(
        ctx: &GeneratorContext,
        info: &PackageInfo,
    ) -> Result<Option<GeneratedFile>, GeneratorError> {
        let package = &ctx.package.name;
        if package.is_empty() {
            return Ok(None);
        }

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

        Ok(Some(GeneratedFile {
            path: output_path,
            content: formatted,
        }))
    }

    /// Generate the storage/mod.rs file
    fn generate_storage_mod(
        ctx: &GeneratorContext,
        info: &PackageInfo,
    ) -> Result<Option<GeneratedFile>, GeneratorError> {
        let package = &ctx.package.name;
        if package.is_empty() {
            return Ok(None);
        }

        if info.services.is_empty() {
            return Ok(None);
        }

        let mut mod_declarations = Vec::new();
        let mut pub_uses = Vec::new();

        // Storage defaults modules (standalone functions for partial overrides)
        for svc in &info.services {
            let defaults_mod = format_ident!("{}_storage_defaults", svc.to_snake_case());
            mod_declarations.push(quote! { pub mod #defaults_mod; });
        }

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
            //!
            //! The storage layer supports partial overrides:
            //! - `*_defaults` modules contain standalone functions with default implementations
            //! - Storage traits have default methods that call these defaults
            //! - Custom implementations can override specific methods and delegate to defaults for others
            //!
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

        Ok(Some(GeneratedFile {
            path: output_path,
            content: formatted,
        }))
    }

    /// Generate the grpc/mod.rs file
    fn generate_grpc_mod(
        ctx: &GeneratorContext,
        info: &PackageInfo,
    ) -> Result<Option<GeneratedFile>, GeneratorError> {
        let package = &ctx.package.name;
        if package.is_empty() {
            return Ok(None);
        }

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

        Ok(Some(GeneratedFile {
            path: output_path,
            content: formatted,
        }))
    }

    /// Generate the storage/conversions.rs file with all From implementations
    fn generate_conversions(
        ctx: &GeneratorContext,
        info: &PackageInfo,
    ) -> Result<Option<GeneratedFile>, GeneratorError> {
        let package = &ctx.package.name;

        if package.is_empty() {
            return Ok(None);
        }

        if info.entities.is_empty() {
            return Ok(None);
        }

        let mut conversions = Vec::new();

        for entity_name in &info.entities {
            let entity_mod = format_ident!("{}", entity_name.to_snake_case());
            let proto_type = format_ident!("{}", entity_name.to_upper_camel_case());
            let create_request = format_ident!("Create{}Request", entity_name.to_upper_camel_case());
            let update_request = format_ident!("Update{}Request", entity_name.to_upper_camel_case());

            // Find the entity in the IR
            let entity = ctx
                .package
                .entities
                .iter()
                .find(|e| &e.name == entity_name);

            if let Some(entity) = entity {
                // Generate Model -> Proto conversion
                let model_fields = generate_model_to_proto_fields(entity.raw);

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

                // Find CreateRequest message in IR messages
                let create_request_name = format!("Create{}Request", entity_name);
                let create_msg = ctx
                    .package
                    .messages
                    .iter()
                    .find(|m| m.name == create_request_name);

                if let Some(create) = create_msg {
                    let create_fields = generate_create_fields(create.raw);

                    // Check if there's a domain type for this request
                    if let Some(ref opts) = create.validate {
                        if opts.generate_conversion && !opts.name.is_empty() {
                            // Use domain type
                            let domain_type = format_ident!("{}", opts.name);
                            conversions.push(quote! {
                                /// Convert validated domain type to SeaORM ActiveModel
                                impl From<super::super::#domain_type> for super::super::entities::#entity_mod::ActiveModel {
                                    fn from(request: super::super::#domain_type) -> Self {
                                        use sea_orm::ActiveValue::Set;
                                        Self {
                                            #(#create_fields)*
                                            ..Default::default()
                                        }
                                    }
                                }
                            });
                        } else {
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
                    } else {
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
                }

                // Find UpdateRequest message in IR messages
                let update_request_name = format!("Update{}Request", entity_name);
                let update_msg = ctx
                    .package
                    .messages
                    .iter()
                    .find(|m| m.name == update_request_name);

                if let Some(update) = update_msg {
                    let update_fields = generate_update_fields(update.raw, entity.raw);

                    // Check if there's a domain type for this request
                    if let Some(ref opts) = update.validate {
                        if opts.generate_conversion && !opts.name.is_empty() {
                            // Use domain type
                            let domain_type = format_ident!("{}", opts.name);
                            conversions.push(quote! {
                                /// Apply validated domain type to SeaORM ActiveModel
                                impl ApplyUpdate<&super::super::#domain_type> for super::super::entities::#entity_mod::ActiveModel {
                                    fn apply_update(&mut self, request: &super::super::#domain_type) {
                                        use sea_orm::ActiveValue::Set;
                                        #(#update_fields)*
                                    }
                                }
                            });
                        } else {
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
                    } else {
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

        Ok(Some(GeneratedFile {
            path: output_path,
            content: formatted,
        }))
    }
}

impl CodeGenerator for PackageGenerator {
    fn name(&self) -> &str {
        "seaorm-package"
    }

    fn finalize_package(
        &self,
        ctx: &GeneratorContext,
    ) -> Result<Vec<GeneratedFile>, GeneratorError> {
        let mut files = Vec::new();
        let info = Self::collect_package_info(ctx);

        // Generate main mod.rs
        if let Some(main_mod) = Self::generate_main_mod(ctx, &info)? {
            files.push(main_mod);
        }

        // Generate entities/mod.rs
        if let Some(entities_mod) = Self::generate_entities_mod(ctx, &info)? {
            files.push(entities_mod);
        }

        // Generate storage/mod.rs
        if let Some(storage_mod) = Self::generate_storage_mod(ctx, &info)? {
            files.push(storage_mod);
        }

        // Generate grpc/mod.rs
        if let Some(grpc_mod) = Self::generate_grpc_mod(ctx, &info)? {
            files.push(grpc_mod);
        }

        // Generate storage/conversions.rs
        if let Some(conversions) = Self::generate_conversions(ctx, &info)? {
            files.push(conversions);
        }

        Ok(files)
    }
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
