//! Relation generation for SeaORM entities
//!
//! This module generates HasOne, HasMany, and BelongsTo relations for SeaORM 2.0.
//!
//! SeaORM 2.0 uses the dense format with relation fields as struct fields.
//!
//! NOTE: Currently disabled - SeaORM 1.x uses a different Relation enum approach.

#![allow(dead_code)]

use synapse_gen::ir::{Relation, RelationType};
use heck::{ToSnakeCase, ToUpperCamelCase};

/// Find the reverse relation name for a self-referential relation
///
/// Given a list of relations and a self-referential relation, find its reverse pair.
/// For example, if we have `parent` (belongs_to) and `replies` (has_many) both pointing
/// to the same entity, they are a reverse pair.
fn find_self_ref_reverse(
    relations: &[Relation],
    current_rel: &Relation,
    current_entity: &str,
) -> Option<String> {
    let is_self_ref = current_rel.related.to_snake_case() == current_entity.to_snake_case();

    if !is_self_ref {
        return None;
    }

    // Find a relation that is:
    // 1. Self-referential (same entity)
    // 2. Different name from current
    // 3. Complementary type (belongs_to <-> has_many, or has_one <-> has_many)
    // 4. Uses the same foreign key (if specified)
    for rel in relations {
        if rel.name == current_rel.name {
            continue; // Skip self
        }

        let rel_is_self_ref = rel.related.to_snake_case() == current_entity.to_snake_case();

        if !rel_is_self_ref {
            continue;
        }

        // Check if they are complementary types
        let is_complementary = matches!(
            (&current_rel.relation_type, &rel.relation_type),
            (RelationType::BelongsTo, RelationType::HasMany)
                | (RelationType::BelongsTo, RelationType::HasOne)
                | (RelationType::HasMany, RelationType::BelongsTo)
                | (RelationType::HasOne, RelationType::BelongsTo)
        );

        if is_complementary {
            // Check if they use the same foreign key
            let same_fk = if !current_rel.foreign_key.is_empty() && !rel.foreign_key.is_empty() {
                current_rel.foreign_key == rel.foreign_key
            } else {
                // If FK not specified, assume they match
                true
            };

            if same_fk {
                return Some(rel.name.to_upper_camel_case());
            }
        }
    }

    None
}

/// Generate all relation fields for a message, properly handling self-referential pairs
pub fn generate_relation_fields(
    relations: &[Relation],
    current_entity: &str,
) -> Vec<proc_macro2::TokenStream> {
    relations
        .iter()
        .filter_map(|rel| {
            let reverse = find_self_ref_reverse(relations, rel, current_entity);
            generate_relation_field_with_reverse(rel, current_entity, reverse.as_deref())
        })
        .collect()
}

/// Generate a relation field with optional relation_reverse for self-referential relations
fn generate_relation_field_with_reverse(
    rel: &Relation,
    current_entity: &str,
    relation_reverse: Option<&str>,
) -> Option<proc_macro2::TokenStream> {
    use quote::{format_ident, quote};

    if rel.name.is_empty() || rel.related.is_empty() {
        return None;
    }

    let field_name = format_ident!("{}", rel.name.to_snake_case());
    let relation_enum_name = rel.name.to_upper_camel_case();

    // Check if this is a self-referential relation
    let is_self_ref = rel.related.to_snake_case() == current_entity.to_snake_case();

    // Check if this is a cross-package relation (contains a dot like "iam.User")
    let is_cross_package = rel.related.contains('.');

    // Generate the target entity path
    let target_entity: syn::Type = if is_self_ref {
        syn::parse_quote!(Entity)
    } else if is_cross_package {
        // Cross-package relation: "iam.User" -> "crate::iam::entities::user::Entity"
        let parts: Vec<&str> = rel.related.split('.').collect();
        if parts.len() == 2 {
            let package = parts[0].to_snake_case();
            let entity = parts[1].to_snake_case();
            syn::parse_str(&format!(
                "crate::{}::entities::{}::Entity",
                package, entity
            ))
            .unwrap_or_else(|_| syn::parse_quote!(Entity))
        } else {
            // Fallback for unexpected format
            syn::parse_str(&format!(
                "super::{}::Entity",
                rel.related.to_snake_case().replace('.', "_")
            ))
            .unwrap_or_else(|_| syn::parse_quote!(Entity))
        }
    } else {
        // Same package relation
        syn::parse_str(&format!(
            "super::{}::Entity",
            rel.related.to_snake_case()
        ))
        .unwrap_or_else(|_| syn::parse_quote!(Entity))
    };

    match rel.relation_type {
        RelationType::HasOne => {
            if is_self_ref {
                if let Some(reverse) = relation_reverse {
                    Some(quote! {
                        #[sea_orm(has_one, self_ref, relation_enum = #relation_enum_name, relation_reverse = #reverse)]
                        pub #field_name: HasOne<#target_entity>
                    })
                } else {
                    Some(quote! {
                        #[sea_orm(has_one, self_ref, relation_enum = #relation_enum_name)]
                        pub #field_name: HasOne<#target_entity>
                    })
                }
            } else {
                Some(quote! {
                    #[sea_orm(has_one)]
                    pub #field_name: HasOne<#target_entity>
                })
            }
        }
        RelationType::HasMany => {
            if !rel.through.is_empty() {
                // Many-to-many via junction table
                // Convert to snake_case module name (treat as message name)
                let via_module = rel.through.to_snake_case();
                if is_self_ref {
                    if let Some(reverse) = relation_reverse {
                        Some(quote! {
                            #[sea_orm(has_many, self_ref, relation_enum = #relation_enum_name, relation_reverse = #reverse, via = #via_module)]
                            pub #field_name: HasMany<#target_entity>
                        })
                    } else {
                        Some(quote! {
                            #[sea_orm(has_many, self_ref, relation_enum = #relation_enum_name, via = #via_module)]
                            pub #field_name: HasMany<#target_entity>
                        })
                    }
                } else {
                    Some(quote! {
                        #[sea_orm(has_many, via = #via_module)]
                        pub #field_name: HasMany<#target_entity>
                    })
                }
            } else if is_self_ref {
                if let Some(reverse) = relation_reverse {
                    Some(quote! {
                        #[sea_orm(has_many, self_ref, relation_enum = #relation_enum_name, relation_reverse = #reverse)]
                        pub #field_name: HasMany<#target_entity>
                    })
                } else {
                    Some(quote! {
                        #[sea_orm(has_many, self_ref, relation_enum = #relation_enum_name)]
                        pub #field_name: HasMany<#target_entity>
                    })
                }
            } else {
                Some(quote! {
                    #[sea_orm(has_many)]
                    pub #field_name: HasMany<#target_entity>
                })
            }
        }
        RelationType::BelongsTo => {
            let from_col = if rel.foreign_key.is_empty() {
                format!("{}_id", rel.related.to_snake_case())
            } else {
                rel.foreign_key.clone()
            };
            let to_col = if rel.references.is_empty() {
                "id".to_string()
            } else {
                rel.references.clone()
            };

            // belongs_to uses HasOne type in SeaORM 2.0 dense format
            if is_self_ref {
                if let Some(reverse) = relation_reverse {
                    Some(quote! {
                        #[sea_orm(self_ref, relation_enum = #relation_enum_name, relation_reverse = #reverse, from = #from_col, to = #to_col)]
                        pub #field_name: HasOne<#target_entity>
                    })
                } else {
                    Some(quote! {
                        #[sea_orm(belongs_to, self_ref, relation_enum = #relation_enum_name, from = #from_col, to = #to_col)]
                        pub #field_name: HasOne<#target_entity>
                    })
                }
            } else {
                Some(quote! {
                    #[sea_orm(belongs_to, from = #from_col, to = #to_col)]
                    pub #field_name: HasOne<#target_entity>
                })
            }
        }
        RelationType::ManyToMany => {
            if !rel.through.is_empty() {
                // Convert to snake_case module name (treat as message name)
                let via_module = rel.through.to_snake_case();
                if is_self_ref {
                    Some(quote! {
                        #[sea_orm(has_many, self_ref, relation_enum = #relation_enum_name, via = #via_module)]
                        pub #field_name: HasMany<#target_entity>
                    })
                } else {
                    Some(quote! {
                        #[sea_orm(has_many, via = #via_module)]
                        pub #field_name: HasMany<#target_entity>
                    })
                }
            } else if is_self_ref {
                Some(quote! {
                    #[sea_orm(has_many, self_ref, relation_enum = #relation_enum_name)]
                    pub #field_name: HasMany<#target_entity>
                })
            } else {
                Some(quote! {
                    #[sea_orm(has_many)]
                    pub #field_name: HasMany<#target_entity>
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_has_many_relation() {
        let rel = Relation {
            name: "posts".to_string(),
            relation_type: RelationType::HasMany,
            related: "post".to_string(),
            foreign_key: String::new(),
            references: String::new(),
            through: String::new(),
        };
        let fields = generate_relation_fields(&[rel], "user");
        assert_eq!(fields.len(), 1);
    }

    #[test]
    fn test_generate_belongs_to_relation() {
        let rel = Relation {
            name: "author".to_string(),
            relation_type: RelationType::BelongsTo,
            related: "user".to_string(),
            foreign_key: "user_id".to_string(),
            references: "id".to_string(),
            through: String::new(),
        };
        let fields = generate_relation_fields(&[rel], "post");
        assert_eq!(fields.len(), 1);
    }

    #[test]
    fn test_empty_relation_def() {
        let rel = Relation {
            name: String::new(),
            relation_type: RelationType::HasOne,
            related: String::new(),
            foreign_key: String::new(),
            references: String::new(),
            through: String::new(),
        };
        let fields = generate_relation_fields(&[rel], "user");
        assert!(fields.is_empty());
    }
}
