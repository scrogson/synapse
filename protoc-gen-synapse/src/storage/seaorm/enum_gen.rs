//! Enum code generation for SeaORM
//!
//! Generates SeaORM-compatible enum types from protobuf enum definitions.
//! Supports both string and integer database representations.

use super::options::{parse_enum_options, parse_enum_value_options, storage};
use crate::GeneratorError;
use heck::{ToSnakeCase, ToUpperCamelCase};
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
    let enum_name = enum_desc
        .name
        .as_ref()
        .ok_or_else(|| GeneratorError::CodeGenError("Enum missing name".to_string()))?;

    // Parse seaorm options
    let options = parse_enum_options(enum_desc);

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
    let db_type = if options.db_type == storage::EnumDbType::Integer as i32 {
        DbType::Integer
    } else {
        DbType::String
    };

    // Generate the enum code
    let enum_tokens = generate_enum_tokens(enum_desc, &rust_enum_name, db_type)?;

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
    enum_desc: &EnumDescriptorProto,
    rust_enum_name: &str,
    db_type: DbType,
) -> Result<TokenStream, GeneratorError> {
    let enum_ident = format_ident!("{}", rust_enum_name);

    // Generate variants
    let mut variants = Vec::new();
    for value in &enum_desc.value {
        let value_name = value
            .name
            .as_ref()
            .ok_or_else(|| GeneratorError::CodeGenError("Enum value missing name".to_string()))?;

        let value_number = value.number.unwrap_or(0);

        // Parse enum value options
        let value_options = parse_enum_value_options(value);

        // Determine variant name (synapse storage doesn't have custom name, use converted proto name)
        let variant_name = convert_enum_variant_name(value_name);

        let variant_ident = format_ident!("{}", variant_name);

        // Generate value attribute based on db_type
        let value_attr = match db_type {
            DbType::String => {
                // Determine string value
                let string_val = if let Some(ref opts) = value_options {
                    if !opts.string_value.is_empty() {
                        opts.string_value.clone()
                    } else {
                        value_name.to_snake_case()
                    }
                } else {
                    value_name.to_snake_case()
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

        variants.push(quote! {
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

    Ok(quote! {
        //! SeaORM enum generated from protobuf
        //! @generated

        #![allow(missing_docs)]

        use sea_orm::entity::prelude::*;

        #[derive(Clone, Copy, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
        #type_attrs
        pub enum #enum_ident {
            #(#variants),*
        }
    })
}

/// Convert a protobuf enum value name to a Rust variant name
///
/// Protobuf convention is SCREAMING_SNAKE_CASE (e.g., STATUS_ACTIVE)
/// Rust convention is PascalCase (e.g., StatusActive or Active)
fn convert_enum_variant_name(name: &str) -> String {
    // Convert SCREAMING_SNAKE_CASE to PascalCase
    name.to_upper_camel_case()
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
        // Create synapse.storage.enum_storage option
        let enum_option = UninterpretedOption {
            name: vec![NamePart {
                name_part: "synapse.storage.enum_storage".to_string(),
                is_extension: true,
            }],
            aggregate_value: Some("db_type: ENUM_DB_TYPE_STRING".to_string()),
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
        let tokens = generate_enum_tokens(&enum_desc, "Status", DbType::String).unwrap();
        let code = tokens.to_string();

        assert!(code.contains("DeriveActiveEnum"));
        assert!(code.contains("rs_type = \"String\""));
        assert!(code.contains("string_value"));
    }

    #[test]
    fn test_generate_enum_tokens_integer() {
        let enum_desc = create_test_enum();
        let tokens = generate_enum_tokens(&enum_desc, "Status", DbType::Integer).unwrap();
        let code = tokens.to_string();

        assert!(code.contains("DeriveActiveEnum"));
        assert!(code.contains("rs_type = \"i32\""));
        assert!(code.contains("num_value"));
    }

    #[test]
    fn test_convert_enum_variant_name() {
        assert_eq!(convert_enum_variant_name("STATUS_ACTIVE"), "StatusActive");
        assert_eq!(convert_enum_variant_name("UNKNOWN"), "Unknown");
        assert_eq!(
            convert_enum_variant_name("MY_LONG_VALUE_NAME"),
            "MyLongValueName"
        );
    }
}
