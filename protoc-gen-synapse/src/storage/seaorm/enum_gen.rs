//! Enum code generation for SeaORM
//!
//! Generates SeaORM-compatible enum types from protobuf enum definitions.
//! Supports both string and integer database representations.

use heck::{ToShoutySnakeCase, ToSnakeCase, ToUpperCamelCase};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use synapse_gen::ir::options::EnumStorageType;
use synapse_gen::ir::{Enum, EnumVariant};
use synapse_gen::{CodeGenerator, GeneratedFile, GeneratorContext, GeneratorError};

/// Generator that produces SeaORM enum types from synapse.storage annotations.
pub struct EnumGenerator;

impl CodeGenerator for EnumGenerator {
    fn name(&self) -> &str {
        "enum"
    }

    fn generate_enum(
        &self,
        _ctx: &GeneratorContext,
        enum_ir: &Enum,
    ) -> Result<Vec<GeneratedFile>, GeneratorError> {
        let storage = match &enum_ir.storage {
            Some(opts) if !opts.skip => opts,
            Some(_) => return Ok(vec![]), // explicitly skipped
            None => return Ok(vec![]),    // no storage options
        };

        let rust_enum_name = enum_ir.name.to_upper_camel_case();

        // Generate the enum code
        let enum_tokens =
            generate_enum_tokens(&rust_enum_name, &enum_ir.variants, &storage.storage_type)?;

        // Format the code
        let code = format_code(enum_tokens)?;

        // Determine output file path from raw file descriptor name
        let proto_path = enum_ir.raw_file.name.as_deref().unwrap_or("unknown.proto");
        let output_path =
            proto_path.replace(".proto", &format!("/{}.rs", rust_enum_name.to_snake_case()));

        Ok(vec![GeneratedFile {
            path: output_path,
            content: code,
        }])
    }
}

/// Generate the TokenStream for a SeaORM enum
fn generate_enum_tokens(
    rust_enum_name: &str,
    variants: &[EnumVariant],
    storage_type: &EnumStorageType,
) -> Result<TokenStream, GeneratorError> {
    let enum_ident = format_ident!("{}", rust_enum_name);

    // Build prefix for stripping from values
    let prefix = format!("{}_", rust_enum_name.to_shouty_snake_case());

    // Generate variants
    let mut variant_tokens = Vec::new();
    let mut has_default = false;

    for variant in variants {
        // Skip if explicitly marked as skip
        if variant.skip {
            continue;
        }

        // Skip UNSPECIFIED/UNKNOWN variants by default
        if variant.name.ends_with("_UNSPECIFIED") || variant.name.ends_with("_UNKNOWN") {
            continue;
        }

        // Check if this is the default variant
        if variant.is_default {
            has_default = true;
        }

        // Determine variant name (strip enum prefix and convert to PascalCase)
        let variant_name = convert_enum_variant_name(&variant.name, rust_enum_name);
        let variant_ident = format_ident!("{}", variant_name);

        // Generate value attribute based on storage_type
        let value_attr = match storage_type {
            EnumStorageType::String | EnumStorageType::Unspecified => {
                let string_val = if !variant.string_value.is_empty() {
                    variant.string_value.clone()
                } else {
                    let stripped = variant.name.strip_prefix(&prefix).unwrap_or(&variant.name);
                    stripped.to_snake_case()
                };
                quote! { #[sea_orm(string_value = #string_val)] }
            }
            EnumStorageType::Integer => {
                let int_val = if variant.int_value != 0 {
                    variant.int_value
                } else {
                    variant.number
                };
                quote! { #[sea_orm(num_value = #int_val)] }
            }
        };

        // Add #[default] if marked
        let default_attr = if variant.is_default {
            quote! { #[default] }
        } else {
            quote! {}
        };

        variant_tokens.push(quote! {
            #default_attr
            #value_attr
            #variant_ident
        });
    }

    // Generate type attributes based on storage_type
    let type_attrs = match storage_type {
        EnumStorageType::String | EnumStorageType::Unspecified => {
            quote! {
                #[sea_orm(rs_type = "String", db_type = "String(StringLen::N(64))")]
            }
        }
        EnumStorageType::Integer => {
            quote! {
                #[sea_orm(rs_type = "i32", db_type = "Integer")]
            }
        }
    };

    // Add Default derive if we have a default variant
    let derives = if has_default {
        quote! { #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, EnumIter, DeriveActiveEnum)] }
    } else {
        quote! { #[derive(Clone, Copy, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum)] }
    };

    Ok(quote! {
        //! SeaORM enum generated from protobuf
        //! @generated

        #![allow(missing_docs)]

        use sea_orm::entity::prelude::*;

        #derives
        #type_attrs
        pub enum #enum_ident {
            #(#variant_tokens),*
        }
    })
}

/// Convert a protobuf enum value name to a Rust variant name
///
/// Protobuf convention is SCREAMING_SNAKE_CASE with enum name prefix (e.g., USER_STATUS_ACTIVE)
/// Rust convention is PascalCase without prefix (e.g., Active)
fn convert_enum_variant_name(name: &str, enum_name: &str) -> String {
    // Build the expected prefix from enum name: "UserStatus" -> "USER_STATUS_"
    let prefix = format!("{}_", enum_name.to_shouty_snake_case());

    // Strip the prefix if present
    let stripped = name.strip_prefix(&prefix).unwrap_or(name);

    // Convert remaining SCREAMING_SNAKE_CASE to PascalCase
    stripped.to_upper_camel_case()
}

/// Format the generated code using prettyplease
fn format_code(tokens: TokenStream) -> Result<String, GeneratorError> {
    let code = tokens.to_string();
    let parsed = syn::parse_file(&code).map_err(|e| {
        GeneratorError::Parse(format!("Failed to parse generated code: {}", e))
    })?;
    Ok(prettyplease::unparse(&parsed))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_variants() -> Vec<EnumVariant> {
        vec![
            EnumVariant {
                name: "STATUS_UNKNOWN".to_string(),
                number: 0,
                string_value: String::new(),
                int_value: 0,
                is_default: false,
                skip: false,
            },
            EnumVariant {
                name: "STATUS_ACTIVE".to_string(),
                number: 1,
                string_value: String::new(),
                int_value: 0,
                is_default: false,
                skip: false,
            },
            EnumVariant {
                name: "STATUS_INACTIVE".to_string(),
                number: 2,
                string_value: String::new(),
                int_value: 0,
                is_default: false,
                skip: false,
            },
        ]
    }

    #[test]
    fn test_generate_enum_tokens_string() {
        let variants = create_test_variants();
        let tokens =
            generate_enum_tokens("Status", &variants, &EnumStorageType::String).unwrap();
        let code = tokens.to_string();

        assert!(code.contains("DeriveActiveEnum"));
        assert!(code.contains("rs_type = \"String\""));
        assert!(code.contains("string_value"));
        // Check that prefix is stripped from string values
        assert!(code.contains("\"active\""));
        assert!(code.contains("\"inactive\""));
        // Check that UNKNOWN is skipped
        assert!(!code.contains("Unknown"));
    }

    #[test]
    fn test_generate_enum_tokens_integer() {
        let variants = create_test_variants();
        let tokens =
            generate_enum_tokens("Status", &variants, &EnumStorageType::Integer).unwrap();
        let code = tokens.to_string();

        assert!(code.contains("DeriveActiveEnum"));
        assert!(code.contains("rs_type = \"i32\""));
        assert!(code.contains("num_value"));
    }

    #[test]
    fn test_convert_enum_variant_name() {
        // With matching prefix - should strip it
        assert_eq!(
            convert_enum_variant_name("USER_STATUS_ACTIVE", "UserStatus"),
            "Active"
        );
        assert_eq!(
            convert_enum_variant_name("USER_STATUS_UNSPECIFIED", "UserStatus"),
            "Unspecified"
        );

        // Without matching prefix - should keep as-is and convert
        assert_eq!(convert_enum_variant_name("UNKNOWN", "Status"), "Unknown");
        assert_eq!(
            convert_enum_variant_name("MY_LONG_VALUE_NAME", "Other"),
            "MyLongValueName"
        );
    }
}
