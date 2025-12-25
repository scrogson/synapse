//! Enum code generation for SeaORM
//!
//! Generates SeaORM-compatible enum types from protobuf enum definitions.
//! Supports both string and integer database representations.

use super::options::{
    get_cached_enum_options, get_cached_enum_value_options, parse_enum_options,
    parse_enum_value_options, storage,
};
use crate::GeneratorError;
use heck::{ToShoutySnakeCase, ToSnakeCase, ToUpperCamelCase};
use proc_macro2::TokenStream;
use prost_types::compiler::code_generator_response::File;
use prost_types::{EnumDescriptorProto, FileDescriptorProto};
use quote::{format_ident, quote};

/// Database type for enum storage
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DbType {
    /// Store as string (default)
    #[default]
    String,
    /// Store as integer
    Integer,
}

/// Generate a SeaORM enum from a protobuf enum definition
///
/// Returns None if the enum should be skipped
pub fn generate(
    file: &FileDescriptorProto,
    enum_desc: &EnumDescriptorProto,
) -> Result<Option<File>, GeneratorError> {
    let file_name = file.name.as_deref().unwrap_or("");
    let enum_name = enum_desc
        .name
        .as_ref()
        .ok_or_else(|| GeneratorError::CodeGenError("Enum missing name".to_string()))?;

    // Try cached options first, then fall back to parsing uninterpreted options
    let options = get_cached_enum_options(file_name, enum_name)
        .or_else(|| parse_enum_options(enum_desc));

    // Check if explicitly skipped
    if let Some(ref opts) = options {
        if opts.skip {
            return Ok(None);
        }
    }

    // If no seaorm options, skip this enum (only generate annotated enums)
    if options.is_none() {
        return Ok(None);
    }

    let options = options.unwrap();

    // Determine the Rust enum name (synapse storage doesn't have custom name option)
    let rust_enum_name = enum_name.to_upper_camel_case();

    // Determine database type from enum value
    let db_type = if options.storage_type == storage::EnumStorageType::Integer as i32 {
        DbType::Integer
    } else {
        DbType::String
    };

    // Generate the enum code
    let enum_tokens = generate_enum_tokens(file_name, enum_desc, &rust_enum_name, db_type)?;

    // Format the code
    let code = format_code(enum_tokens)?;

    // Determine output file path
    let proto_path = file.name.as_deref().unwrap_or("unknown.proto");
    let output_path =
        proto_path.replace(".proto", &format!("/{}.rs", rust_enum_name.to_snake_case()));

    Ok(Some(File {
        name: Some(output_path),
        content: Some(code),
        ..Default::default()
    }))
}

/// Generate the TokenStream for a SeaORM enum
fn generate_enum_tokens(
    file_name: &str,
    enum_desc: &EnumDescriptorProto,
    rust_enum_name: &str,
    db_type: DbType,
) -> Result<TokenStream, GeneratorError> {
    let enum_ident = format_ident!("{}", rust_enum_name);
    let proto_enum_name = enum_desc.name.as_deref().unwrap_or("");

    // Build prefix for stripping from values
    let prefix = format!("{}_", rust_enum_name.to_shouty_snake_case());

    // Generate variants
    let mut variants = Vec::new();
    let mut has_default = false;

    for value in &enum_desc.value {
        let value_name = value
            .name
            .as_ref()
            .ok_or_else(|| GeneratorError::CodeGenError("Enum value missing name".to_string()))?;

        let value_number = value.number.unwrap_or(0);

        // Try cached options first, then fall back to parsing
        let value_options = get_cached_enum_value_options(file_name, proto_enum_name, value_number)
            .or_else(|| parse_enum_value_options(value));

        // Skip if explicitly marked as skip
        if value_options.as_ref().is_some_and(|opts| opts.skip) {
            continue;
        }

        // Skip UNSPECIFIED/UNKNOWN variants by default
        if value_name.ends_with("_UNSPECIFIED") || value_name.ends_with("_UNKNOWN") {
            continue;
        }

        // Check if this is the default variant
        let is_default = value_options.as_ref().is_some_and(|opts| opts.default);
        if is_default {
            has_default = true;
        }

        // Determine variant name (strip enum prefix and convert to PascalCase)
        let variant_name = convert_enum_variant_name(value_name, rust_enum_name);
        let variant_ident = format_ident!("{}", variant_name);

        // Generate value attribute based on db_type
        let value_attr = match db_type {
            DbType::String => {
                // Determine string value - strip prefix like we do for variant names
                let string_val = if let Some(ref opts) = value_options {
                    if !opts.string_value.is_empty() {
                        opts.string_value.clone()
                    } else {
                        // Strip prefix and convert to snake_case
                        let stripped = value_name.strip_prefix(&prefix).unwrap_or(value_name);
                        stripped.to_snake_case()
                    }
                } else {
                    let stripped = value_name.strip_prefix(&prefix).unwrap_or(value_name);
                    stripped.to_snake_case()
                };
                quote! { #[sea_orm(string_value = #string_val)] }
            }
            DbType::Integer => {
                // Determine int value
                let int_val = if let Some(ref opts) = value_options {
                    if opts.int_value != 0 {
                        opts.int_value
                    } else {
                        value_number
                    }
                } else {
                    value_number
                };
                quote! { #[sea_orm(num_value = #int_val)] }
            }
        };

        // Add #[default] if marked
        let default_attr = if is_default {
            quote! { #[default] }
        } else {
            quote! {}
        };

        variants.push(quote! {
            #default_attr
            #value_attr
            #variant_ident
        });
    }

    // Generate type attributes based on db_type
    let type_attrs = match db_type {
        DbType::String => {
            quote! {
                #[sea_orm(rs_type = "String", db_type = "String(StringLen::N(64))")]
            }
        }
        DbType::Integer => {
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
            #(#variants),*
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
        GeneratorError::CodeGenError(format!("Failed to parse generated code: {}", e))
    })?;
    Ok(prettyplease::unparse(&parsed))
}

#[cfg(test)]
mod tests {
    use super::*;
    use prost_types::uninterpreted_option::NamePart;
    use prost_types::{EnumOptions, EnumValueDescriptorProto, UninterpretedOption};

    fn create_test_enum() -> EnumDescriptorProto {
        // Create synapse.storage.enum_type option
        let enum_option = UninterpretedOption {
            name: vec![NamePart {
                name_part: "synapse.storage.enum_type".to_string(),
                is_extension: true,
            }],
            aggregate_value: Some("storage_type: ENUM_STORAGE_TYPE_STRING".to_string()),
            ..Default::default()
        };

        EnumDescriptorProto {
            name: Some("Status".to_string()),
            value: vec![
                EnumValueDescriptorProto {
                    name: Some("STATUS_UNKNOWN".to_string()),
                    number: Some(0),
                    ..Default::default()
                },
                EnumValueDescriptorProto {
                    name: Some("STATUS_ACTIVE".to_string()),
                    number: Some(1),
                    ..Default::default()
                },
                EnumValueDescriptorProto {
                    name: Some("STATUS_INACTIVE".to_string()),
                    number: Some(2),
                    ..Default::default()
                },
            ],
            options: Some(EnumOptions {
                uninterpreted_option: vec![enum_option],
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    #[test]
    fn test_generate_enum_tokens_string() {
        let enum_desc = create_test_enum();
        let tokens = generate_enum_tokens("test.proto", &enum_desc, "Status", DbType::String).unwrap();
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
        let enum_desc = create_test_enum();
        let tokens = generate_enum_tokens("test.proto", &enum_desc, "Status", DbType::Integer).unwrap();
        let code = tokens.to_string();

        assert!(code.contains("DeriveActiveEnum"));
        assert!(code.contains("rs_type = \"i32\""));
        assert!(code.contains("num_value"));
    }

    #[test]
    fn test_convert_enum_variant_name() {
        // With matching prefix - should strip it
        assert_eq!(convert_enum_variant_name("USER_STATUS_ACTIVE", "UserStatus"), "Active");
        assert_eq!(convert_enum_variant_name("USER_STATUS_UNSPECIFIED", "UserStatus"), "Unspecified");

        // Without matching prefix - should keep as-is and convert
        assert_eq!(convert_enum_variant_name("UNKNOWN", "Status"), "Unknown");
        assert_eq!(convert_enum_variant_name("MY_LONG_VALUE_NAME", "Other"), "MyLongValueName");
    }
}
