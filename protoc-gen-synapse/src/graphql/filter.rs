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
    pub needs_timestamp_filter: bool,
}

/// Generate all filter-related types for a package
///
/// Always generates GraphQL wrapper types with From impls to convert to/from proto types.
/// Proto types are needed for gRPC/prost, GraphQL wrappers are needed for async-graphql.
pub fn generate_filters_for_package(
    file: &FileDescriptorProto,
    entities: &[&DescriptorProto],
    all_files: &[FileDescriptorProto],
) -> Result<Vec<File>, GeneratorError> {
    let mut files = Vec::new();
    let mut ctx = FilterContext::default();

    // First pass: determine which primitive filters we need
    for entity in entities {
        analyze_entity_fields(entity, &mut ctx);
    }

    // Generate primitive filter types (GraphQL wrappers for proto types)
    if ctx.needs_int_filter {
        files.push(generate_int_filter(file)?);
    }
    if ctx.needs_string_filter {
        files.push(generate_string_filter(file)?);
    }
    if ctx.needs_bool_filter {
        files.push(generate_bool_filter(file)?);
    }
    if ctx.needs_float_filter {
        files.push(generate_float_filter(file)?);
    }
    if ctx.needs_timestamp_filter {
        files.push(generate_timestamp_filter(file)?);
    }

    // Always generate OrderDirection enum (GraphQL wrapper for proto enum)
    files.push(generate_order_direction(file)?);

    // Generate entity-specific filter and orderBy types (GraphQL wrappers)
    for entity in entities {
        let entity_name = entity.name.as_deref().unwrap_or("");
        files.push(generate_entity_filter(file, entity, entity_name, all_files)?);
        files.push(generate_entity_order_by(file, entity, entity_name, all_files)?);
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
            Type::Message => {
                // Check if this is a Timestamp type
                if let Some(type_name) = &field.type_name {
                    if type_name.contains("Timestamp") {
                        ctx.needs_timestamp_filter = true;
                    }
                }
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
            pub neq: Option<i64>,
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
            /// Is null check
            pub is_null: Option<bool>,
        }

        // Convert to proto type in synapse.relay package
        // Path: graphql/ -> package/ -> generated/ -> synapse/relay/
        impl From<IntFilter> for super::super::super::synapse::relay::IntFilter {
            fn from(f: IntFilter) -> Self {
                Self {
                    eq: f.eq,
                    neq: f.neq,
                    gt: f.gt,
                    gte: f.gte,
                    lt: f.lt,
                    lte: f.lte,
                    r#in: f.r#in.unwrap_or_default(),
                    is_null: f.is_null,
                }
            }
        }
    };

    let content = code.to_string();
    let formatted = match syn::parse_file(&content) {
        Ok(parsed) => prettyplease::unparse(&parsed),
        Err(_) => content,
    };

    let package = file.package.as_deref().unwrap_or("");
    let output_path = format!("{}/graphql/int_filter.rs", package.replace('.', "/"));

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
            pub neq: Option<String>,
            /// Greater than
            pub gt: Option<String>,
            /// Greater than or equal
            pub gte: Option<String>,
            /// Less than
            pub lt: Option<String>,
            /// Less than or equal
            pub lte: Option<String>,
            /// In list
            #[graphql(name = "in")]
            pub r#in: Option<Vec<String>>,
            /// SQL LIKE pattern
            pub like: Option<String>,
            /// Case-insensitive LIKE
            pub ilike: Option<String>,
            /// Starts with
            pub starts_with: Option<String>,
            /// Ends with
            pub ends_with: Option<String>,
            /// Contains substring
            pub contains: Option<String>,
            /// Is null check
            pub is_null: Option<bool>,
        }

        // Convert to proto type in synapse.relay package
        // Path: graphql/ -> package/ -> generated/ -> synapse/relay/
        impl From<StringFilter> for super::super::super::synapse::relay::StringFilter {
            fn from(f: StringFilter) -> Self {
                Self {
                    eq: f.eq,
                    neq: f.neq,
                    gt: f.gt,
                    gte: f.gte,
                    lt: f.lt,
                    lte: f.lte,
                    r#in: f.r#in.unwrap_or_default(),
                    like: f.like,
                    ilike: f.ilike,
                    starts_with: f.starts_with,
                    ends_with: f.ends_with,
                    contains: f.contains,
                    is_null: f.is_null,
                }
            }
        }
    };

    let content = code.to_string();
    let formatted = match syn::parse_file(&content) {
        Ok(parsed) => prettyplease::unparse(&parsed),
        Err(_) => content,
    };

    let package = file.package.as_deref().unwrap_or("");
    let output_path = format!("{}/graphql/string_filter.rs", package.replace('.', "/"));

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
            /// Is null check
            pub is_null: Option<bool>,
        }

        // Convert to proto type in synapse.relay package
        // Path: graphql/ -> package/ -> generated/ -> synapse/relay/
        impl From<BoolFilter> for super::super::super::synapse::relay::BoolFilter {
            fn from(f: BoolFilter) -> Self {
                Self {
                    eq: f.eq,
                    is_null: f.is_null,
                }
            }
        }
    };

    let content = code.to_string();
    let formatted = match syn::parse_file(&content) {
        Ok(parsed) => prettyplease::unparse(&parsed),
        Err(_) => content,
    };

    let package = file.package.as_deref().unwrap_or("");
    let output_path = format!("{}/graphql/bool_filter.rs", package.replace('.', "/"));

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

    let package = file.package.as_deref().unwrap_or("");
    let output_path = format!("{}/graphql/float_filter.rs", package.replace('.', "/"));

    Ok(File {
        name: Some(output_path),
        content: Some(formatted),
        ..Default::default()
    })
}

/// Generate TimestampFilter type
fn generate_timestamp_filter(file: &FileDescriptorProto) -> Result<File, GeneratorError> {
    let code = quote! {
        //! Auto-generated TimestampFilter type
        //! @generated

        #![allow(missing_docs)]

        use async_graphql::InputObject;

        /// Filter for timestamp fields (using seconds since Unix epoch)
        #[derive(InputObject, Default, Clone)]
        pub struct TimestampFilter {
            /// Equals (seconds since Unix epoch)
            pub eq: Option<i64>,
            /// Not equals (seconds since Unix epoch)
            pub neq: Option<i64>,
            /// Greater than (seconds since Unix epoch)
            pub gt: Option<i64>,
            /// Greater than or equal (seconds since Unix epoch)
            pub gte: Option<i64>,
            /// Less than (seconds since Unix epoch)
            pub lt: Option<i64>,
            /// Less than or equal (seconds since Unix epoch)
            pub lte: Option<i64>,
            /// Is null check
            pub is_null: Option<bool>,
        }

        // Convert to proto type in synapse.relay package
        // Path: graphql/ -> package/ -> generated/ -> synapse/relay/
        impl From<TimestampFilter> for super::super::super::synapse::relay::TimestampFilter {
            fn from(f: TimestampFilter) -> Self {
                Self {
                    eq: f.eq.map(|s| prost_types::Timestamp { seconds: s, nanos: 0 }),
                    neq: f.neq.map(|s| prost_types::Timestamp { seconds: s, nanos: 0 }),
                    gt: f.gt.map(|s| prost_types::Timestamp { seconds: s, nanos: 0 }),
                    gte: f.gte.map(|s| prost_types::Timestamp { seconds: s, nanos: 0 }),
                    lt: f.lt.map(|s| prost_types::Timestamp { seconds: s, nanos: 0 }),
                    lte: f.lte.map(|s| prost_types::Timestamp { seconds: s, nanos: 0 }),
                    is_null: f.is_null,
                }
            }
        }
    };

    let content = code.to_string();
    let formatted = match syn::parse_file(&content) {
        Ok(parsed) => prettyplease::unparse(&parsed),
        Err(_) => content,
    };

    let package = file.package.as_deref().unwrap_or("");
    let output_path = format!("{}/graphql/timestamp_filter.rs", package.replace('.', "/"));

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

    let package = file.package.as_deref().unwrap_or("");
    let output_path = format!("{}/graphql/order_direction.rs", package.replace('.', "/"));

    Ok(File {
        name: Some(output_path),
        content: Some(formatted),
        ..Default::default()
    })
}

/// Generate entity-specific filter type (e.g., UserFilter)
///
/// If proto defines a Filter type, uses its fields. Otherwise uses entity fields.
fn generate_entity_filter(
    file: &FileDescriptorProto,
    entity: &DescriptorProto,
    entity_name: &str,
    all_files: &[FileDescriptorProto],
) -> Result<File, GeneratorError> {
    let filter_name = format!("{}Filter", entity_name.to_upper_camel_case());
    let filter_ident = format_ident!("{}", filter_name);

    let mut field_tokens = Vec::new();
    let mut conversion_tokens = Vec::new();

    // Check if proto defines this Filter type (search across all files including imports)
    let proto_filter = all_files
        .iter()
        .flat_map(|f| f.message_type.iter())
        .find(|m| m.name.as_deref() == Some(&filter_name));

    // Use proto fields if defined, otherwise derive from entity fields
    let fields_to_use: Vec<_> = if let Some(proto_f) = proto_filter {
        proto_f.field.iter().collect()
    } else {
        entity.field.iter().collect()
    };

    // Track if we need logical operators (and, or, not)
    let mut has_and = false;
    let mut has_or = false;
    let mut has_not = false;

    for field in fields_to_use {
        let field_name = field.name.as_deref().unwrap_or("");
        let field_ident = format_ident!("{}", field_name.to_snake_case());

        // Handle logical operators specially - they reference the filter type itself
        if field_name == "and" {
            has_and = true;
            continue;
        }
        if field_name == "or" {
            has_or = true;
            continue;
        }
        if field_name == "not" {
            has_not = true;
            continue;
        }

        // Skip timestamp fields when using entity fields
        if proto_filter.is_none() {
            if let Some(type_name) = &field.type_name {
                if type_name.contains("Timestamp") {
                    continue;
                }
            }
        }

        // Determine filter type based on field type
        // For proto Filter, fields reference filter types (IntFilter, StringFilter, etc.)
        // For entity fields, derive from the primitive type
        let filter_type = if proto_filter.is_some() {
            // Proto Filter field - get the referenced type name
            field.type_name.as_ref().and_then(|type_name| {
                let simple_name = type_name.rsplit('.').next().unwrap_or(type_name);
                let type_ident = format_ident!("{}", simple_name);
                Some(quote! { #type_ident })
            })
        } else {
            // Entity field - derive filter type from primitive type
            match field.r#type() {
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
            }
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

    // Add logical operators if present in proto
    if has_and {
        field_tokens.push(quote! {
            /// All conditions must match
            pub and: Option<Vec<Box<#filter_ident>>>,
        });
        conversion_tokens.push(quote! {
            and: f.and.map(|v| v.into_iter().map(|f| (*f).into()).collect()).unwrap_or_default(),
        });
    }
    if has_or {
        field_tokens.push(quote! {
            /// Any condition must match
            pub or: Option<Vec<Box<#filter_ident>>>,
        });
        conversion_tokens.push(quote! {
            or: f.or.map(|v| v.into_iter().map(|f| (*f).into()).collect()).unwrap_or_default(),
        });
    }
    if has_not {
        field_tokens.push(quote! {
            /// Negate the condition
            pub not: Option<Box<#filter_ident>>,
        });
        conversion_tokens.push(quote! {
            not: f.not.map(|f| Box::new((*f).into())),
        });
    }

    let code = quote! {
        //! Auto-generated filter type for entity
        //! @generated

        #![allow(missing_docs)]

        use async_graphql::InputObject;
        use super::{IntFilter, StringFilter, BoolFilter, TimestampFilter};

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

    let package = file.package.as_deref().unwrap_or("");
    let output_path = format!(
        "{}/graphql/{}_filter.rs",
        package.replace('.', "/"),
        entity_name.to_snake_case()
    );

    Ok(File {
        name: Some(output_path),
        content: Some(formatted),
        ..Default::default()
    })
}

/// Generate entity-specific order by type (e.g., UserOrderBy)
///
/// If proto defines an OrderBy type, uses its fields. Otherwise uses entity fields.
fn generate_entity_order_by(
    file: &FileDescriptorProto,
    entity: &DescriptorProto,
    entity_name: &str,
    all_files: &[FileDescriptorProto],
) -> Result<File, GeneratorError> {
    let order_by_name = format!("{}OrderBy", entity_name.to_upper_camel_case());
    let order_by_ident = format_ident!("{}", order_by_name);

    let mut field_tokens = Vec::new();
    let mut conversion_tokens = Vec::new();

    // Check if proto defines this OrderBy type (search across all files including imports)
    let proto_order_by = all_files
        .iter()
        .flat_map(|f| f.message_type.iter())
        .find(|m| m.name.as_deref() == Some(&order_by_name));

    // Use proto fields if defined, otherwise derive from entity fields
    let fields_to_use: Vec<_> = if let Some(proto_ob) = proto_order_by {
        proto_ob.field.iter().collect()
    } else {
        entity.field.iter().collect()
    };

    for field in fields_to_use {
        let field_name = field.name.as_deref().unwrap_or("");
        let field_ident = format_ident!("{}", field_name.to_snake_case());

        // If using proto OrderBy, all fields are sortable (they're in the proto)
        // If using entity, include sortable fields (primitives and timestamps)
        let is_sortable = if proto_order_by.is_some() {
            true // Proto-defined fields are explicitly sortable
        } else {
            match field.r#type() {
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
            }
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

    let package = file.package.as_deref().unwrap_or("");
    let output_path = format!(
        "{}/graphql/{}_order_by.rs",
        package.replace('.', "/"),
        entity_name.to_snake_case()
    );

    Ok(File {
        name: Some(output_path),
        content: Some(formatted),
        ..Default::default()
    })
}
