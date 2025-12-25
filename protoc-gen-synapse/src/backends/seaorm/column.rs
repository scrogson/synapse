//! Column attribute generation for SeaORM entities
//!
//! This module generates the #[sea_orm(...)] attributes for entity fields.

use super::options::storage::ColumnOptions;
use super::types::MappedType;

/// Generated SeaORM column attributes for a field
pub struct ColumnAttributes {
    /// The #[sea_orm(...)] attribute contents
    pub attributes: Vec<String>,
}

/// Generate column attributes from column options and mapped type
pub fn generate_attributes(
    column_options: Option<&ColumnOptions>,
    _mapped_type: &MappedType,
    _is_nullable: bool,
) -> ColumnAttributes {
    let mut attributes = Vec::new();

    if let Some(opts) = column_options {
        // Primary key with auto_increment handling
        // SeaORM defaults to auto_increment = true for primary keys,
        // so we only need to specify when it's false
        if opts.primary_key {
            if opts.auto_increment {
                attributes.push("primary_key".to_string());
            } else {
                attributes.push("primary_key, auto_increment = false".to_string());
            }
        }

        // Unique constraint
        if opts.unique {
            attributes.push("unique".to_string());
        }

        // Custom column name
        if !opts.column_name.is_empty() {
            attributes.push(format!("column_name = \"{}\"", opts.column_name));
        }

        // Column type handling
        // - embed implies JsonBinary if not explicitly set
        // - normalize JsonB/Jsonb/jsonb to JsonBinary for SeaORM 2.0
        if !opts.column_type.is_empty() {
            let column_type = normalize_column_type(&opts.column_type);
            attributes.push(format!("column_type = \"{}\"", column_type));
        } else if opts.embed {
            attributes.push("column_type = \"JsonBinary\"".to_string());
        }

        // Default value
        if !opts.default_value.is_empty() {
            attributes.push(format!("default_value = \"{}\"", opts.default_value));
        }

        // Default expression
        if !opts.default_expr.is_empty() {
            attributes.push(format!("default_expr = \"{}\"", opts.default_expr));
        }
    }

    ColumnAttributes { attributes }
}

/// Normalize column type names for SeaORM 2.0
fn normalize_column_type(column_type: &str) -> &str {
    match column_type {
        "JsonB" | "Jsonb" | "jsonb" => "JsonBinary",
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_mapped_type() -> MappedType {
        MappedType {
            rust_type: "String".to_string(),
        }
    }

    #[test]
    fn test_no_options() {
        let result = generate_attributes(None, &default_mapped_type(), false);
        assert!(result.attributes.is_empty());
    }

    #[test]
    fn test_primary_key_without_auto_increment() {
        // When primary_key is true but auto_increment is false,
        // we need to explicitly disable auto_increment
        let opts = ColumnOptions {
            primary_key: true,
            auto_increment: false,
            ..Default::default()
        };
        let result = generate_attributes(Some(&opts), &default_mapped_type(), false);
        assert!(result
            .attributes
            .contains(&"primary_key, auto_increment = false".to_string()));
    }

    #[test]
    fn test_primary_key_with_auto_increment() {
        // When both are true, just primary_key is enough (SeaORM defaults to auto_increment)
        let opts = ColumnOptions {
            primary_key: true,
            auto_increment: true,
            ..Default::default()
        };
        let result = generate_attributes(Some(&opts), &default_mapped_type(), false);
        assert!(result.attributes.contains(&"primary_key".to_string()));
        assert!(!result
            .attributes
            .iter()
            .any(|a| a.contains("auto_increment")));
    }

    #[test]
    fn test_unique() {
        let opts = ColumnOptions {
            unique: true,
            ..Default::default()
        };
        let result = generate_attributes(Some(&opts), &default_mapped_type(), false);
        assert!(result.attributes.contains(&"unique".to_string()));
    }

    #[test]
    fn test_column_name() {
        let opts = ColumnOptions {
            column_name: "custom_name".to_string(),
            ..Default::default()
        };
        let result = generate_attributes(Some(&opts), &default_mapped_type(), false);
        assert!(result
            .attributes
            .contains(&"column_name = \"custom_name\"".to_string()));
    }

    #[test]
    fn test_column_type() {
        let opts = ColumnOptions {
            column_type: "Uuid".to_string(),
            ..Default::default()
        };
        let result = generate_attributes(Some(&opts), &default_mapped_type(), false);
        assert!(result
            .attributes
            .contains(&"column_type = \"Uuid\"".to_string()));
    }

    #[test]
    fn test_default_value() {
        let opts = ColumnOptions {
            default_value: "0".to_string(),
            ..Default::default()
        };
        let result = generate_attributes(Some(&opts), &default_mapped_type(), false);
        assert!(result
            .attributes
            .contains(&"default_value = \"0\"".to_string()));
    }

    #[test]
    fn test_default_expr() {
        let opts = ColumnOptions {
            default_expr: "Expr::current_timestamp()".to_string(),
            ..Default::default()
        };
        let result = generate_attributes(Some(&opts), &default_mapped_type(), false);
        assert!(result
            .attributes
            .contains(&"default_expr = \"Expr::current_timestamp()\"".to_string()));
    }

    #[test]
    fn test_combined_attributes() {
        let opts = ColumnOptions {
            primary_key: true,
            auto_increment: true,
            column_name: "user_id".to_string(),
            ..Default::default()
        };
        let mapped = MappedType {
            rust_type: "i64".to_string(),
        };
        let result = generate_attributes(Some(&opts), &mapped, false);
        assert_eq!(result.attributes.len(), 2);
        assert!(result.attributes.contains(&"primary_key".to_string()));
        assert!(result
            .attributes
            .contains(&"column_name = \"user_id\"".to_string()));
    }

    #[test]
    fn test_embed_implies_json_binary() {
        let opts = ColumnOptions {
            embed: true,
            ..Default::default()
        };
        let result = generate_attributes(Some(&opts), &default_mapped_type(), false);
        assert!(result
            .attributes
            .contains(&"column_type = \"JsonBinary\"".to_string()));
    }

    #[test]
    fn test_jsonb_normalized_to_json_binary() {
        let opts = ColumnOptions {
            column_type: "Jsonb".to_string(),
            ..Default::default()
        };
        let result = generate_attributes(Some(&opts), &default_mapped_type(), false);
        assert!(result
            .attributes
            .contains(&"column_type = \"JsonBinary\"".to_string()));
    }
}
