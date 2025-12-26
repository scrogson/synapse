//! Auto-generated filter types for GraphQL
//!
//! Generates:
//! - Primitive filter types (IntFilter, StringFilter, BoolFilter)
//! - Entity-specific filter types (UserFilter, PostFilter, etc.)
//! - Entity-specific order by types (UserOrderBy, PostOrderBy, etc.)
//! - OrderDirection enum

use crate::error::GeneratorError;
use heck::{ToSnakeCase, ToUpperCamelCase};
use prost_types::compiler::code_generator_response::File;
use prost_types::field_descriptor_proto::Type;
use prost_types::{DescriptorProto, FileDescriptorProto};
use quote::{format_ident, quote};

/// Track which primitive types we need to generate filters for
#[derive(Default)]
pub struct FilterContext {
    pub needs_int_filter: bool,
    pub needs_string_filter: bool,
    pub needs_bool_filter: bool,
    pub needs_float_filter: bool,
}

/// Check if a message type exists in the file
fn message_exists(file: &FileDescriptorProto, name: &str) -> bool {
    file.message_type
        .iter()
        .any(|m| m.name.as_deref() == Some(name))
}

/// Generate all filter-related types for a package
///
/// Only generates types that are NOT already defined in proto.
/// Proto-defined types are handled by object.rs which generates proper GraphQL wrappers.
pub fn generate_filters_for_package(
    file: &FileDescriptorProto,
    entities: &[&DescriptorProto],
) -> Result<Vec<File>, GeneratorError> {
    let mut files = Vec::new();
    let mut ctx = FilterContext::default();

    // First pass: determine which primitive filters we need
    for entity in entities {
        analyze_entity_fields(entity, &mut ctx);
    }

    // Generate primitive filter types only if not defined in proto
    if ctx.needs_int_filter && !message_exists(file, "IntFilter") {
        files.push(generate_int_filter(file)?);
    }
    if ctx.needs_string_filter && !message_exists(file, "StringFilter") {
        files.push(generate_string_filter(file)?);
    }
    if ctx.needs_bool_filter && !message_exists(file, "BoolFilter") {
        files.push(generate_bool_filter(file)?);
    }
    if ctx.needs_float_filter && !message_exists(file, "FloatFilter") {
        files.push(generate_float_filter(file)?);
    }

    // Always generate OrderDirection enum (proto enums need GraphQL wrappers)
    files.push(generate_order_direction(file)?);

    // Generate entity-specific filter and orderBy types only if not defined in proto
    for entity in entities {
        let entity_name = entity.name.as_deref().unwrap_or("");
        let filter_name = format!("{}Filter", entity_name.to_upper_camel_case());
        let order_by_name = format!("{}OrderBy", entity_name.to_upper_camel_case());

        if !message_exists(file, &filter_name) {
            files.push(generate_entity_filter(file, entity, entity_name)?);
        }
        if !message_exists(file, &order_by_name) {
            files.push(generate_entity_order_by(file, entity, entity_name)?);
        }
    }

    Ok(files)
}

/// Analyze entity fields to determine which primitive filters are needed
fn analyze_entity_fields(entity: &DescriptorProto, ctx: &mut FilterContext) {
    for field in &entity.field {
        match field.r#type() {
            Type::Int64 | Type::Int32 | Type::Uint64 | Type::Uint32
            | Type::Sint32 | Type::Sint64 | Type::Fixed32 | Type::Fixed64
            | Type::Sfixed32 | Type::Sfixed64 => {
                ctx.needs_int_filter = true;
            }
            Type::String => {
                ctx.needs_string_filter = true;
            }
            Type::Bool => {
                ctx.needs_bool_filter = true;
            }
            Type::Float | Type::Double => {
                ctx.needs_float_filter = true;
            }
            _ => {}
        }
    }
}

/// Generate IntFilter type
fn generate_int_filter(file: &FileDescriptorProto) -> Result<File, GeneratorError> {
    let code = quote! {
        //! Auto-generated IntFilter type
        //! @generated

        #![allow(missing_docs)]

        use async_graphql::InputObject;

        /// Filter for integer fields
        #[derive(InputObject, Default, Clone)]
        pub struct IntFilter {
            /// Equals
            pub eq: Option<i64>,
            /// Not equals
            pub ne: Option<i64>,
            /// Greater than
            pub gt: Option<i64>,
            /// Greater than or equal
            pub gte: Option<i64>,
            /// Less than
            pub lt: Option<i64>,
            /// Less than or equal
            pub lte: Option<i64>,
            /// In list
            #[graphql(name = "in")]
            pub r#in: Option<Vec<i64>>,
        }

        impl From<IntFilter> for super::super::IntFilter {
            fn from(f: IntFilter) -> Self {
                Self {
                    eq: f.eq,
                    ne: f.ne,
                    gt: f.gt,
                    gte: f.gte,
                    lt: f.lt,
                    lte: f.lte,
                    r#in: f.r#in.unwrap_or_default(),
                }
            }
        }
    };

    let content = code.to_string();
    let formatted = match syn::parse_file(&content) {
        Ok(parsed) => prettyplease::unparse(&parsed),
        Err(_) => content,
    };

    let proto_path = file.name.as_deref().unwrap_or("unknown.proto");
    let output_path = proto_path.replace(".proto", "/graphql/int_filter.rs");

    Ok(File {
        name: Some(output_path),
        content: Some(formatted),
        ..Default::default()
    })
}

/// Generate StringFilter type
fn generate_string_filter(file: &FileDescriptorProto) -> Result<File, GeneratorError> {
    let code = quote! {
        //! Auto-generated StringFilter type
        //! @generated

        #![allow(missing_docs)]

        use async_graphql::InputObject;

        /// Filter for string fields
        #[derive(InputObject, Default, Clone)]
        pub struct StringFilter {
            /// Equals
            pub eq: Option<String>,
            /// Not equals
            pub ne: Option<String>,
            /// Contains substring
            pub contains: Option<String>,
            /// Starts with
            pub starts_with: Option<String>,
            /// Ends with
            pub ends_with: Option<String>,
        }

        impl From<StringFilter> for super::super::StringFilter {
            fn from(f: StringFilter) -> Self {
                Self {
                    eq: f.eq,
                    ne: f.ne,
                    contains: f.contains,
                    starts_with: f.starts_with,
                    ends_with: f.ends_with,
                }
            }
        }
    };

    let content = code.to_string();
    let formatted = match syn::parse_file(&content) {
        Ok(parsed) => prettyplease::unparse(&parsed),
        Err(_) => content,
    };

    let proto_path = file.name.as_deref().unwrap_or("unknown.proto");
    let output_path = proto_path.replace(".proto", "/graphql/string_filter.rs");

    Ok(File {
        name: Some(output_path),
        content: Some(formatted),
        ..Default::default()
    })
}

/// Generate BoolFilter type
fn generate_bool_filter(file: &FileDescriptorProto) -> Result<File, GeneratorError> {
    let code = quote! {
        //! Auto-generated BoolFilter type
        //! @generated

        #![allow(missing_docs)]

        use async_graphql::InputObject;

        /// Filter for boolean fields
        #[derive(InputObject, Default, Clone)]
        pub struct BoolFilter {
            /// Equals
            pub eq: Option<bool>,
        }

        impl From<BoolFilter> for super::super::BoolFilter {
            fn from(f: BoolFilter) -> Self {
                Self { eq: f.eq }
            }
        }
    };

    let content = code.to_string();
    let formatted = match syn::parse_file(&content) {
        Ok(parsed) => prettyplease::unparse(&parsed),
        Err(_) => content,
    };

    let proto_path = file.name.as_deref().unwrap_or("unknown.proto");
    let output_path = proto_path.replace(".proto", "/graphql/bool_filter.rs");

    Ok(File {
        name: Some(output_path),
        content: Some(formatted),
        ..Default::default()
    })
}

/// Generate FloatFilter type
fn generate_float_filter(file: &FileDescriptorProto) -> Result<File, GeneratorError> {
    let code = quote! {
        //! Auto-generated FloatFilter type
        //! @generated

        #![allow(missing_docs)]

        use async_graphql::InputObject;

        /// Filter for float/double fields
        #[derive(InputObject, Default, Clone)]
        pub struct FloatFilter {
            /// Equals
            pub eq: Option<f64>,
            /// Not equals
            pub ne: Option<f64>,
            /// Greater than
            pub gt: Option<f64>,
            /// Greater than or equal
            pub gte: Option<f64>,
            /// Less than
            pub lt: Option<f64>,
            /// Less than or equal
            pub lte: Option<f64>,
        }
    };

    let content = code.to_string();
    let formatted = match syn::parse_file(&content) {
        Ok(parsed) => prettyplease::unparse(&parsed),
        Err(_) => content,
    };

    let proto_path = file.name.as_deref().unwrap_or("unknown.proto");
    let output_path = proto_path.replace(".proto", "/graphql/float_filter.rs");

    Ok(File {
        name: Some(output_path),
        content: Some(formatted),
        ..Default::default()
    })
}

/// Generate OrderDirection enum
fn generate_order_direction(file: &FileDescriptorProto) -> Result<File, GeneratorError> {
    let code = quote! {
        //! Auto-generated OrderDirection enum
        //! @generated

        #![allow(missing_docs)]

        use async_graphql::Enum;

        /// Order direction for sorting
        #[derive(Enum, Clone, Copy, PartialEq, Eq, Default)]
        pub enum OrderDirection {
            #[default]
            /// Ascending order
            Asc,
            /// Descending order
            Desc,
        }

        impl From<OrderDirection> for i32 {
            fn from(d: OrderDirection) -> Self {
                match d {
                    OrderDirection::Asc => 1,  // ASC = 1 in proto
                    OrderDirection::Desc => 2, // DESC = 2 in proto
                }
            }
        }

        impl From<i32> for OrderDirection {
            fn from(v: i32) -> Self {
                match v {
                    1 => Self::Asc,
                    2 => Self::Desc,
                    _ => Self::Asc,
                }
            }
        }
    };

    let content = code.to_string();
    let formatted = match syn::parse_file(&content) {
        Ok(parsed) => prettyplease::unparse(&parsed),
        Err(_) => content,
    };

    let proto_path = file.name.as_deref().unwrap_or("unknown.proto");
    let output_path = proto_path.replace(".proto", "/graphql/order_direction.rs");

    Ok(File {
        name: Some(output_path),
        content: Some(formatted),
        ..Default::default()
    })
}

/// Generate entity-specific filter type (e.g., UserFilter)
fn generate_entity_filter(
    file: &FileDescriptorProto,
    entity: &DescriptorProto,
    entity_name: &str,
) -> Result<File, GeneratorError> {
    let filter_name = format!("{}Filter", entity_name.to_upper_camel_case());
    let filter_ident = format_ident!("{}", filter_name);

    let mut field_tokens = Vec::new();
    let mut conversion_tokens = Vec::new();

    for field in &entity.field {
        let field_name = field.name.as_deref().unwrap_or("");
        let field_ident = format_ident!("{}", field_name.to_snake_case());

        // Skip timestamp fields and relation fields for now
        if let Some(type_name) = &field.type_name {
            if type_name.contains("Timestamp") {
                continue;
            }
        }

        // Determine filter type based on field type
        let filter_type = match field.r#type() {
            Type::Int64 | Type::Int32 | Type::Uint64 | Type::Uint32
            | Type::Sint32 | Type::Sint64 | Type::Fixed32 | Type::Fixed64
            | Type::Sfixed32 | Type::Sfixed64 => {
                Some(quote! { IntFilter })
            }
            Type::String => {
                Some(quote! { StringFilter })
            }
            Type::Bool => {
                Some(quote! { BoolFilter })
            }
            Type::Float | Type::Double => {
                Some(quote! { FloatFilter })
            }
            _ => None,
        };

        if let Some(filter_ty) = filter_type {
            field_tokens.push(quote! {
                pub #field_ident: Option<#filter_ty>,
            });
            conversion_tokens.push(quote! {
                #field_ident: f.#field_ident.map(Into::into),
            });
        }
    }

    let code = quote! {
        //! Auto-generated filter type for entity
        //! @generated

        #![allow(missing_docs)]

        use async_graphql::InputObject;
        use super::{IntFilter, StringFilter, BoolFilter};

        /// Filter for entity queries
        #[derive(InputObject, Default, Clone)]
        pub struct #filter_ident {
            #(#field_tokens)*
        }

        impl From<#filter_ident> for super::super::#filter_ident {
            fn from(f: #filter_ident) -> Self {
                Self {
                    #(#conversion_tokens)*
                }
            }
        }
    };

    let content = code.to_string();
    let formatted = match syn::parse_file(&content) {
        Ok(parsed) => prettyplease::unparse(&parsed),
        Err(_) => content,
    };

    let proto_path = file.name.as_deref().unwrap_or("unknown.proto");
    let output_path = proto_path.replace(
        ".proto",
        &format!("/graphql/{}_filter.rs", entity_name.to_snake_case()),
    );

    Ok(File {
        name: Some(output_path),
        content: Some(formatted),
        ..Default::default()
    })
}

/// Generate entity-specific order by type (e.g., UserOrderBy)
fn generate_entity_order_by(
    file: &FileDescriptorProto,
    entity: &DescriptorProto,
    entity_name: &str,
) -> Result<File, GeneratorError> {
    let order_by_name = format!("{}OrderBy", entity_name.to_upper_camel_case());
    let order_by_ident = format_ident!("{}", order_by_name);

    let mut field_tokens = Vec::new();
    let mut conversion_tokens = Vec::new();

    for field in &entity.field {
        let field_name = field.name.as_deref().unwrap_or("");
        let field_ident = format_ident!("{}", field_name.to_snake_case());

        // Include sortable fields (primitives and timestamps)
        let is_sortable = match field.r#type() {
            Type::Int64 | Type::Int32 | Type::Uint64 | Type::Uint32
            | Type::Sint32 | Type::Sint64 | Type::Fixed32 | Type::Fixed64
            | Type::Sfixed32 | Type::Sfixed64 | Type::String | Type::Bool
            | Type::Float | Type::Double => true,
            Type::Message => {
                // Include Timestamp fields
                field.type_name.as_ref()
                    .map(|t| t.contains("Timestamp"))
                    .unwrap_or(false)
            }
            _ => false,
        };

        if is_sortable {
            field_tokens.push(quote! {
                pub #field_ident: Option<OrderDirection>,
            });
            conversion_tokens.push(quote! {
                #field_ident: o.#field_ident.map(|d| d.into()),
            });
        }
    }

    let code = quote! {
        //! Auto-generated order by type for entity
        //! @generated

        #![allow(missing_docs)]

        use async_graphql::InputObject;
        use super::OrderDirection;

        /// Order by options for entity queries
        #[derive(InputObject, Default, Clone)]
        pub struct #order_by_ident {
            #(#field_tokens)*
        }

        impl From<#order_by_ident> for super::super::#order_by_ident {
            fn from(o: #order_by_ident) -> Self {
                Self {
                    #(#conversion_tokens)*
                }
            }
        }
    };

    let content = code.to_string();
    let formatted = match syn::parse_file(&content) {
        Ok(parsed) => prettyplease::unparse(&parsed),
        Err(_) => content,
    };

    let proto_path = file.name.as_deref().unwrap_or("unknown.proto");
    let output_path = proto_path.replace(
        ".proto",
        &format!("/graphql/{}_order_by.rs", entity_name.to_snake_case()),
    );

    Ok(File {
        name: Some(output_path),
        content: Some(formatted),
        ..Default::default()
    })
}
