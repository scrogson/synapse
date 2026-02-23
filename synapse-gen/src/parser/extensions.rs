//! Extension extraction from prost-reflect DynamicMessages.
//!
//! Decodes the raw `CodeGeneratorRequest` bytes using prost-reflect's
//! `DescriptorPool` to access Synapse custom extension fields, then
//! collects all extracted options into an `ExtractedOptions` struct.

use std::collections::HashMap;

use once_cell::sync::Lazy;
use prost_reflect::{DescriptorPool, DynamicMessage, Value};

use crate::options::synapse::{graphql, grpc, storage, validate};

// ---------------------------------------------------------------------------
// Static descriptor pool
// ---------------------------------------------------------------------------

static FILE_DESCRIPTOR_SET_BYTES: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/file_descriptor_set.bin"));

static DESCRIPTOR_POOL: Lazy<DescriptorPool> = Lazy::new(|| {
    DescriptorPool::decode(FILE_DESCRIPTOR_SET_BYTES).expect("Failed to decode file descriptor set")
});

// ---------------------------------------------------------------------------
// Extension name constants
// ---------------------------------------------------------------------------

const ENTITY_EXTENSION_NAME: &str = "synapse.storage.entity";
const COLUMN_EXTENSION_NAME: &str = "synapse.storage.column";
const ENUM_EXTENSION_NAME: &str = "synapse.storage.enum_type";
const ENUM_VALUE_EXTENSION_NAME: &str = "synapse.storage.enum_value";
const SERVICE_EXTENSION_NAME: &str = "synapse.storage.service";
const METHOD_EXTENSION_NAME: &str = "synapse.storage.method";

const GRPC_SERVICE_EXTENSION_NAME: &str = "synapse.grpc.service";
const GRPC_METHOD_EXTENSION_NAME: &str = "synapse.grpc.method";
const GRPC_RESPONSE_EXTENSION_NAME: &str = "synapse.grpc.response";

const VALIDATE_MESSAGE_EXTENSION_NAME: &str = "synapse.validate.message";
const VALIDATE_FIELD_EXTENSION_NAME: &str = "synapse.validate.field";

const GRAPHQL_TYPE_EXTENSION_NAME: &str = "synapse.graphql.type";
const GRAPHQL_FIELD_EXTENSION_NAME: &str = "synapse.graphql.field";
const GRAPHQL_SERVICE_EXTENSION_NAME: &str = "synapse.graphql.service";
const GRAPHQL_QUERY_EXTENSION_NAME: &str = "synapse.graphql.query";
const GRAPHQL_MUTATION_EXTENSION_NAME: &str = "synapse.graphql.mutation";
const GRAPHQL_SUBSCRIPTION_EXTENSION_NAME: &str = "synapse.graphql.subscription";

const GRAPHQL_RESOLVER_EXTENSION_NAME: &str = "synapse.graphql.resolver";
const GRAPHQL_FIELD_RESOLVER_EXTENSION_NAME: &str = "synapse.graphql.field_resolver";
const GRAPHQL_METHOD_RESOLVER_EXTENSION_NAME: &str = "synapse.graphql.method_resolver";

// ---------------------------------------------------------------------------
// ExtractedOptions
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct ExtractedOptions {
    /// (file_name, message_name) -> EntityOptions
    pub entity_options: HashMap<(String, String), storage::EntityOptions>,
    /// (file_name, message_name, field_number) -> ColumnOptions
    pub column_options: HashMap<(String, String, i32), storage::ColumnOptions>,
    /// (file_name, enum_name) -> EnumOptions
    pub enum_options: HashMap<(String, String), storage::EnumOptions>,
    /// (file_name, enum_name, value_number) -> EnumValueOptions
    pub enum_value_options: HashMap<(String, String, i32), storage::EnumValueOptions>,
    /// (file_name, service_name) -> ServiceOptions
    pub service_options: HashMap<(String, String), storage::ServiceOptions>,
    /// (file_name, service_name, method_name) -> MethodOptions
    pub method_options: HashMap<(String, String, String), storage::MethodOptions>,
    /// (file_name, service_name) -> grpc::ServiceOptions
    pub grpc_service_options: HashMap<(String, String), grpc::ServiceOptions>,
    /// (file_name, service_name, method_name) -> grpc::MethodOptions
    pub grpc_method_options: HashMap<(String, String, String), grpc::MethodOptions>,
    /// (file_name, message_name) -> grpc::ResponseOptions
    pub grpc_response_options: HashMap<(String, String), grpc::ResponseOptions>,
    /// (file_name, message_name) -> validate::MessageOptions
    pub validate_message_options: HashMap<(String, String), validate::MessageOptions>,
    /// (file_name, message_name, field_number) -> validate::FieldOptions
    pub validate_field_options: HashMap<(String, String, i32), validate::FieldOptions>,
    /// (file_name, message_name) -> graphql::TypeOptions
    pub graphql_type_options: HashMap<(String, String), graphql::TypeOptions>,
    /// (file_name, message_name, field_number) -> graphql::FieldOptions
    pub graphql_field_options: HashMap<(String, String, i32), graphql::FieldOptions>,
    /// (file_name, service_name) -> graphql::ServiceOptions
    pub graphql_service_options: HashMap<(String, String), graphql::ServiceOptions>,
    /// (file_name, service_name, method_name) -> graphql::QueryOptions
    pub graphql_query_options: HashMap<(String, String, String), graphql::QueryOptions>,
    /// (file_name, service_name, method_name) -> graphql::MutationOptions
    pub graphql_mutation_options: HashMap<(String, String, String), graphql::MutationOptions>,
    /// (file_name, service_name, method_name) -> graphql::SubscriptionOptions
    pub graphql_subscription_options:
        HashMap<(String, String, String), graphql::SubscriptionOptions>,
    /// (file_name, message_name) -> graphql::MessageResolverOptions
    pub graphql_resolver_options: HashMap<(String, String), graphql::MessageResolverOptions>,
    /// (file_name, message_name, field_number) -> graphql::FieldResolverOptions
    pub graphql_field_resolver_options:
        HashMap<(String, String, i32), graphql::FieldResolverOptions>,
    /// (file_name, service_name, method_name) -> graphql::MethodResolverOptions
    pub graphql_method_resolver_options:
        HashMap<(String, String, String), graphql::MethodResolverOptions>,
}

// ---------------------------------------------------------------------------
// Top-level extraction
// ---------------------------------------------------------------------------

pub fn extract_options(bytes: &[u8]) -> Result<ExtractedOptions, String> {
    let request_desc = DESCRIPTOR_POOL
        .get_message_by_name("google.protobuf.compiler.CodeGeneratorRequest")
        .ok_or("CodeGeneratorRequest not found in descriptor pool")?;

    let request = DynamicMessage::decode(request_desc, bytes)
        .map_err(|e| format!("Failed to decode CodeGeneratorRequest: {}", e))?;

    let mut options = ExtractedOptions::default();

    if let Some(cow) = request.get_field_by_name("proto_file") {
        if let Value::List(files) = cow.as_ref() {
            for file_value in files.iter() {
                if let Some(file_msg) = file_value.as_message() {
                    extract_options_from_file(&mut options, file_msg)?;
                }
            }
        }
    }

    Ok(options)
}

// ---------------------------------------------------------------------------
// File-level extraction
// ---------------------------------------------------------------------------

fn extract_options_from_file(
    opts: &mut ExtractedOptions,
    file: &DynamicMessage,
) -> Result<(), String> {
    let file_name = file
        .get_field_by_name("name")
        .and_then(|v| v.as_ref().as_str().map(|s| s.to_string()))
        .unwrap_or_default();

    // Messages
    if let Some(cow) = file.get_field_by_name("message_type") {
        if let Value::List(messages) = cow.as_ref() {
            for msg_value in messages.iter() {
                if let Some(msg) = msg_value.as_message() {
                    extract_message_options(opts, &file_name, msg, "")?;
                }
            }
        }
    }

    // Enums
    if let Some(cow) = file.get_field_by_name("enum_type") {
        if let Value::List(enums) = cow.as_ref() {
            for enum_value in enums.iter() {
                if let Some(enum_msg) = enum_value.as_message() {
                    extract_enum_options(opts, &file_name, enum_msg)?;
                }
            }
        }
    }

    // Services
    if let Some(cow) = file.get_field_by_name("service") {
        if let Value::List(services) = cow.as_ref() {
            for service_value in services.iter() {
                if let Some(service_msg) = service_value.as_message() {
                    extract_service_options(opts, &file_name, service_msg)?;
                }
            }
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Message-level extraction
// ---------------------------------------------------------------------------

fn extract_message_options(
    opts: &mut ExtractedOptions,
    file_name: &str,
    msg: &DynamicMessage,
    parent_prefix: &str,
) -> Result<(), String> {
    let msg_name = msg
        .get_field_by_name("name")
        .and_then(|v| v.as_ref().as_str().map(|s| s.to_string()))
        .unwrap_or_default();

    let full_name = if parent_prefix.is_empty() {
        msg_name.clone()
    } else {
        format!("{}.{}", parent_prefix, msg_name)
    };

    // Message-level options
    if let Some(cow) = msg.get_field_by_name("options") {
        if let Some(opts_msg) = cow.as_ref().as_message() {
            // synapse.storage.entity
            if let Some(ext_field) = DESCRIPTOR_POOL.get_extension_by_name(ENTITY_EXTENSION_NAME) {
                if opts_msg.has_extension(&ext_field) {
                    let ext_value = opts_msg.get_extension(&ext_field);
                    if let Some(entity_opts) = convert_to_entity_options(&ext_value) {
                        opts.entity_options
                            .insert((file_name.to_string(), full_name.clone()), entity_opts);
                    }
                }
            }

            // synapse.validate.message
            if let Some(ext_field) =
                DESCRIPTOR_POOL.get_extension_by_name(VALIDATE_MESSAGE_EXTENSION_NAME)
            {
                if opts_msg.has_extension(&ext_field) {
                    let ext_value = opts_msg.get_extension(&ext_field);
                    if let Some(validate_opts) = convert_to_validate_message_options(&ext_value) {
                        opts.validate_message_options
                            .insert((file_name.to_string(), full_name.clone()), validate_opts);
                    }
                }
            }

            // synapse.grpc.response
            if let Some(ext_field) =
                DESCRIPTOR_POOL.get_extension_by_name(GRPC_RESPONSE_EXTENSION_NAME)
            {
                if opts_msg.has_extension(&ext_field) {
                    let ext_value = opts_msg.get_extension(&ext_field);
                    if let Some(response_opts) = convert_to_grpc_response_options(&ext_value) {
                        opts.grpc_response_options
                            .insert((file_name.to_string(), full_name.clone()), response_opts);
                    }
                }
            }

            // synapse.graphql.type
            if let Some(ext_field) =
                DESCRIPTOR_POOL.get_extension_by_name(GRAPHQL_TYPE_EXTENSION_NAME)
            {
                if opts_msg.has_extension(&ext_field) {
                    let ext_value = opts_msg.get_extension(&ext_field);
                    if let Some(graphql_opts) = convert_to_graphql_type_options(&ext_value) {
                        opts.graphql_type_options
                            .insert((file_name.to_string(), full_name.clone()), graphql_opts);
                    }
                }
            }

            // synapse.graphql.resolver
            if let Some(ext_field) =
                DESCRIPTOR_POOL.get_extension_by_name(GRAPHQL_RESOLVER_EXTENSION_NAME)
            {
                if opts_msg.has_extension(&ext_field) {
                    let ext_value = opts_msg.get_extension(&ext_field);
                    if let Some(resolver_opts) = convert_to_message_resolver_options(&ext_value) {
                        opts.graphql_resolver_options
                            .insert((file_name.to_string(), full_name.clone()), resolver_opts);
                    }
                }
            }
        }
    }

    // Field-level options
    if let Some(cow) = msg.get_field_by_name("field") {
        if let Value::List(fields) = cow.as_ref() {
            for field_value in fields.iter() {
                if let Some(field_msg) = field_value.as_message() {
                    let field_number = field_msg
                        .get_field_by_name("number")
                        .and_then(|v| {
                            if let Value::I32(n) = v.as_ref() {
                                Some(*n)
                            } else {
                                None
                            }
                        })
                        .unwrap_or(0);

                    if let Some(opts_cow) = field_msg.get_field_by_name("options") {
                        if let Some(opts_msg) = opts_cow.as_ref().as_message() {
                            // synapse.storage.column
                            if let Some(ext_field) =
                                DESCRIPTOR_POOL.get_extension_by_name(COLUMN_EXTENSION_NAME)
                            {
                                if opts_msg.has_extension(&ext_field) {
                                    let ext_value = opts_msg.get_extension(&ext_field);
                                    if let Some(col_opts) = convert_to_column_options(&ext_value) {
                                        opts.column_options.insert(
                                            (
                                                file_name.to_string(),
                                                full_name.clone(),
                                                field_number,
                                            ),
                                            col_opts,
                                        );
                                    }
                                }
                            }

                            // synapse.validate.field
                            if let Some(ext_field) =
                                DESCRIPTOR_POOL.get_extension_by_name(VALIDATE_FIELD_EXTENSION_NAME)
                            {
                                if opts_msg.has_extension(&ext_field) {
                                    let ext_value = opts_msg.get_extension(&ext_field);
                                    if let Some(field_opts) =
                                        convert_to_validate_field_options(&ext_value)
                                    {
                                        opts.validate_field_options.insert(
                                            (
                                                file_name.to_string(),
                                                full_name.clone(),
                                                field_number,
                                            ),
                                            field_opts,
                                        );
                                    }
                                }
                            }

                            // synapse.graphql.field
                            if let Some(ext_field) =
                                DESCRIPTOR_POOL.get_extension_by_name(GRAPHQL_FIELD_EXTENSION_NAME)
                            {
                                if opts_msg.has_extension(&ext_field) {
                                    let ext_value = opts_msg.get_extension(&ext_field);
                                    if let Some(field_opts) =
                                        convert_to_graphql_field_options(&ext_value)
                                    {
                                        opts.graphql_field_options.insert(
                                            (
                                                file_name.to_string(),
                                                full_name.clone(),
                                                field_number,
                                            ),
                                            field_opts,
                                        );
                                    }
                                }
                            }

                            // synapse.graphql.field_resolver
                            if let Some(ext_field) = DESCRIPTOR_POOL
                                .get_extension_by_name(GRAPHQL_FIELD_RESOLVER_EXTENSION_NAME)
                            {
                                if opts_msg.has_extension(&ext_field) {
                                    let ext_value = opts_msg.get_extension(&ext_field);
                                    if let Some(resolver_opts) =
                                        convert_to_field_resolver_options(&ext_value)
                                    {
                                        opts.graphql_field_resolver_options.insert(
                                            (
                                                file_name.to_string(),
                                                full_name.clone(),
                                                field_number,
                                            ),
                                            resolver_opts,
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Nested messages
    if let Some(cow) = msg.get_field_by_name("nested_type") {
        if let Value::List(nested) = cow.as_ref() {
            for nested_value in nested.iter() {
                if let Some(nested_msg) = nested_value.as_message() {
                    extract_message_options(opts, file_name, nested_msg, &full_name)?;
                }
            }
        }
    }

    // Nested enums
    if let Some(cow) = msg.get_field_by_name("enum_type") {
        if let Value::List(enums) = cow.as_ref() {
            for enum_value in enums.iter() {
                if let Some(enum_msg) = enum_value.as_message() {
                    extract_enum_options_nested(opts, file_name, enum_msg, &full_name)?;
                }
            }
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Enum-level extraction
// ---------------------------------------------------------------------------

fn extract_enum_options(
    opts: &mut ExtractedOptions,
    file_name: &str,
    enum_msg: &DynamicMessage,
) -> Result<(), String> {
    extract_enum_options_nested(opts, file_name, enum_msg, "")
}

fn extract_enum_options_nested(
    opts: &mut ExtractedOptions,
    file_name: &str,
    enum_msg: &DynamicMessage,
    parent_prefix: &str,
) -> Result<(), String> {
    let enum_name = enum_msg
        .get_field_by_name("name")
        .and_then(|v| v.as_ref().as_str().map(|s| s.to_string()))
        .unwrap_or_default();

    let full_name = if parent_prefix.is_empty() {
        enum_name.clone()
    } else {
        format!("{}.{}", parent_prefix, enum_name)
    };

    // synapse.storage.enum_type
    if let Some(cow) = enum_msg.get_field_by_name("options") {
        if let Some(opts_msg) = cow.as_ref().as_message() {
            if let Some(ext_field) = DESCRIPTOR_POOL.get_extension_by_name(ENUM_EXTENSION_NAME) {
                if opts_msg.has_extension(&ext_field) {
                    let ext_value = opts_msg.get_extension(&ext_field);
                    if let Some(enum_opts) = convert_to_enum_options(&ext_value) {
                        opts.enum_options
                            .insert((file_name.to_string(), full_name.clone()), enum_opts);
                    }
                }
            }
        }
    }

    // Enum value options
    if let Some(cow) = enum_msg.get_field_by_name("value") {
        if let Value::List(values) = cow.as_ref() {
            for value_val in values.iter() {
                if let Some(value_msg) = value_val.as_message() {
                    let value_number = value_msg
                        .get_field_by_name("number")
                        .and_then(|v| {
                            if let Value::I32(n) = v.as_ref() {
                                Some(*n)
                            } else {
                                None
                            }
                        })
                        .unwrap_or(0);

                    if let Some(opts_cow) = value_msg.get_field_by_name("options") {
                        if let Some(opts_msg) = opts_cow.as_ref().as_message() {
                            if let Some(ext_field) =
                                DESCRIPTOR_POOL.get_extension_by_name(ENUM_VALUE_EXTENSION_NAME)
                            {
                                if opts_msg.has_extension(&ext_field) {
                                    let ext_value = opts_msg.get_extension(&ext_field);
                                    if let Some(value_opts) =
                                        convert_to_enum_value_options(&ext_value)
                                    {
                                        opts.enum_value_options.insert(
                                            (
                                                file_name.to_string(),
                                                full_name.clone(),
                                                value_number,
                                            ),
                                            value_opts,
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Service-level extraction
// ---------------------------------------------------------------------------

fn extract_service_options(
    opts: &mut ExtractedOptions,
    file_name: &str,
    service: &DynamicMessage,
) -> Result<(), String> {
    let service_name = service
        .get_field_by_name("name")
        .and_then(|v| v.as_ref().as_str().map(|s| s.to_string()))
        .unwrap_or_default();

    // Service-level options
    if let Some(cow) = service.get_field_by_name("options") {
        if let Some(opts_msg) = cow.as_ref().as_message() {
            // synapse.storage.service
            if let Some(ext_field) = DESCRIPTOR_POOL.get_extension_by_name(SERVICE_EXTENSION_NAME) {
                if opts_msg.has_extension(&ext_field) {
                    let ext_value = opts_msg.get_extension(&ext_field);
                    if let Some(service_opts) = convert_to_service_options(&ext_value) {
                        opts.service_options
                            .insert((file_name.to_string(), service_name.clone()), service_opts);
                    }
                }
            }

            // synapse.grpc.service
            if let Some(ext_field) =
                DESCRIPTOR_POOL.get_extension_by_name(GRPC_SERVICE_EXTENSION_NAME)
            {
                if opts_msg.has_extension(&ext_field) {
                    let ext_value = opts_msg.get_extension(&ext_field);
                    if let Some(grpc_opts) = convert_to_grpc_service_options(&ext_value) {
                        opts.grpc_service_options
                            .insert((file_name.to_string(), service_name.clone()), grpc_opts);
                    }
                }
            }

            // synapse.graphql.service
            if let Some(ext_field) =
                DESCRIPTOR_POOL.get_extension_by_name(GRAPHQL_SERVICE_EXTENSION_NAME)
            {
                if opts_msg.has_extension(&ext_field) {
                    let ext_value = opts_msg.get_extension(&ext_field);
                    if let Some(graphql_opts) = convert_to_graphql_service_options(&ext_value) {
                        opts.graphql_service_options
                            .insert((file_name.to_string(), service_name.clone()), graphql_opts);
                    }
                }
            }
        }
    }

    // Method-level options
    if let Some(cow) = service.get_field_by_name("method") {
        if let Value::List(methods) = cow.as_ref() {
            for method_value in methods.iter() {
                if let Some(method_msg) = method_value.as_message() {
                    let method_name = method_msg
                        .get_field_by_name("name")
                        .and_then(|v| v.as_ref().as_str().map(|s| s.to_string()))
                        .unwrap_or_default();

                    if let Some(opts_cow) = method_msg.get_field_by_name("options") {
                        if let Some(opts_msg) = opts_cow.as_ref().as_message() {
                            // synapse.storage.method
                            if let Some(ext_field) =
                                DESCRIPTOR_POOL.get_extension_by_name(METHOD_EXTENSION_NAME)
                            {
                                if opts_msg.has_extension(&ext_field) {
                                    let ext_value = opts_msg.get_extension(&ext_field);
                                    if let Some(method_opts) =
                                        convert_to_method_options(&ext_value)
                                    {
                                        opts.method_options.insert(
                                            (
                                                file_name.to_string(),
                                                service_name.clone(),
                                                method_name.clone(),
                                            ),
                                            method_opts,
                                        );
                                    }
                                }
                            }

                            // synapse.grpc.method
                            if let Some(ext_field) =
                                DESCRIPTOR_POOL.get_extension_by_name(GRPC_METHOD_EXTENSION_NAME)
                            {
                                if opts_msg.has_extension(&ext_field) {
                                    let ext_value = opts_msg.get_extension(&ext_field);
                                    if let Some(grpc_method_opts) =
                                        convert_to_grpc_method_options(&ext_value)
                                    {
                                        opts.grpc_method_options.insert(
                                            (
                                                file_name.to_string(),
                                                service_name.clone(),
                                                method_name.clone(),
                                            ),
                                            grpc_method_opts,
                                        );
                                    }
                                }
                            }

                            // synapse.graphql.query
                            if let Some(ext_field) =
                                DESCRIPTOR_POOL.get_extension_by_name(GRAPHQL_QUERY_EXTENSION_NAME)
                            {
                                if opts_msg.has_extension(&ext_field) {
                                    let ext_value = opts_msg.get_extension(&ext_field);
                                    if let Some(query_opts) =
                                        convert_to_graphql_query_options(&ext_value)
                                    {
                                        opts.graphql_query_options.insert(
                                            (
                                                file_name.to_string(),
                                                service_name.clone(),
                                                method_name.clone(),
                                            ),
                                            query_opts,
                                        );
                                    }
                                }
                            }

                            // synapse.graphql.mutation
                            if let Some(ext_field) = DESCRIPTOR_POOL
                                .get_extension_by_name(GRAPHQL_MUTATION_EXTENSION_NAME)
                            {
                                if opts_msg.has_extension(&ext_field) {
                                    let ext_value = opts_msg.get_extension(&ext_field);
                                    if let Some(mutation_opts) =
                                        convert_to_graphql_mutation_options(&ext_value)
                                    {
                                        opts.graphql_mutation_options.insert(
                                            (
                                                file_name.to_string(),
                                                service_name.clone(),
                                                method_name.clone(),
                                            ),
                                            mutation_opts,
                                        );
                                    }
                                }
                            }

                            // synapse.graphql.subscription
                            if let Some(ext_field) = DESCRIPTOR_POOL
                                .get_extension_by_name(GRAPHQL_SUBSCRIPTION_EXTENSION_NAME)
                            {
                                if opts_msg.has_extension(&ext_field) {
                                    let ext_value = opts_msg.get_extension(&ext_field);
                                    if let Some(subscription_opts) =
                                        convert_to_graphql_subscription_options(&ext_value)
                                    {
                                        opts.graphql_subscription_options.insert(
                                            (
                                                file_name.to_string(),
                                                service_name.clone(),
                                                method_name.clone(),
                                            ),
                                            subscription_opts,
                                        );
                                    }
                                }
                            }

                            // synapse.graphql.method_resolver
                            if let Some(ext_field) = DESCRIPTOR_POOL
                                .get_extension_by_name(GRAPHQL_METHOD_RESOLVER_EXTENSION_NAME)
                            {
                                if opts_msg.has_extension(&ext_field) {
                                    let ext_value = opts_msg.get_extension(&ext_field);
                                    if let Some(method_resolver_opts) =
                                        convert_to_method_resolver_options(&ext_value)
                                    {
                                        opts.graphql_method_resolver_options.insert(
                                            (
                                                file_name.to_string(),
                                                service_name.clone(),
                                                method_name.clone(),
                                            ),
                                            method_resolver_opts,
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

// =============================================================================
// Value conversion helpers
// =============================================================================

fn convert_to_entity_options(value: &Value) -> Option<storage::EntityOptions> {
    let msg = value.as_message()?;
    let mut result = storage::EntityOptions::default();

    if let Some(cow) = msg.get_field_by_name("table_name") {
        if let Value::String(s) = cow.as_ref() {
            result.table_name = s.clone();
        }
    }

    if let Some(cow) = msg.get_field_by_name("skip") {
        if let Value::Bool(b) = cow.as_ref() {
            result.skip = *b;
        }
    }

    if let Some(cow) = msg.get_field_by_name("relations") {
        if let Value::List(list) = cow.as_ref() {
            for item in list.iter() {
                if let Some(rel) = convert_to_relation_def(item) {
                    result.relations.push(rel);
                }
            }
        }
    }

    Some(result)
}

fn convert_to_relation_def(value: &Value) -> Option<storage::RelationDef> {
    let msg = value.as_message()?;
    let mut result = storage::RelationDef::default();

    if let Some(cow) = msg.get_field_by_name("name") {
        if let Value::String(s) = cow.as_ref() {
            result.name = s.clone();
        }
    }

    if let Some(cow) = msg.get_field_by_name("type") {
        if let Value::EnumNumber(n) = cow.as_ref() {
            result.r#type = *n;
        }
    }

    if let Some(cow) = msg.get_field_by_name("related") {
        if let Value::String(s) = cow.as_ref() {
            result.related = s.clone();
        }
    }

    if let Some(cow) = msg.get_field_by_name("foreign_key") {
        if let Value::String(s) = cow.as_ref() {
            result.foreign_key = s.clone();
        }
    }

    if let Some(cow) = msg.get_field_by_name("references") {
        if let Value::String(s) = cow.as_ref() {
            result.references = s.clone();
        }
    }

    if let Some(cow) = msg.get_field_by_name("through") {
        if let Value::String(s) = cow.as_ref() {
            result.through = s.clone();
        }
    }

    Some(result)
}

fn convert_to_column_options(value: &Value) -> Option<storage::ColumnOptions> {
    let msg = value.as_message()?;
    let mut result = storage::ColumnOptions::default();

    if let Some(cow) = msg.get_field_by_name("primary_key") {
        if let Value::Bool(b) = cow.as_ref() {
            result.primary_key = *b;
        }
    }

    if let Some(cow) = msg.get_field_by_name("auto_increment") {
        if let Value::Bool(b) = cow.as_ref() {
            result.auto_increment = *b;
        }
    }

    if let Some(cow) = msg.get_field_by_name("unique") {
        if let Value::Bool(b) = cow.as_ref() {
            result.unique = *b;
        }
    }

    if let Some(cow) = msg.get_field_by_name("column_name") {
        if let Value::String(s) = cow.as_ref() {
            result.column_name = s.clone();
        }
    }

    if let Some(cow) = msg.get_field_by_name("default_value") {
        if let Value::String(s) = cow.as_ref() {
            result.default_value = s.clone();
        }
    }

    if let Some(cow) = msg.get_field_by_name("embed") {
        if let Value::Bool(b) = cow.as_ref() {
            result.embed = *b;
        }
    }

    if let Some(cow) = msg.get_field_by_name("column_type") {
        if let Value::String(s) = cow.as_ref() {
            result.column_type = s.clone();
        }
    }

    if let Some(cow) = msg.get_field_by_name("default_expr") {
        if let Value::String(s) = cow.as_ref() {
            result.default_expr = s.clone();
        }
    }

    Some(result)
}

fn convert_to_enum_options(value: &Value) -> Option<storage::EnumOptions> {
    let msg = value.as_message()?;
    let mut result = storage::EnumOptions::default();

    if let Some(cow) = msg.get_field_by_name("storage_type") {
        if let Value::EnumNumber(n) = cow.as_ref() {
            result.storage_type = *n;
        }
    }

    if let Some(cow) = msg.get_field_by_name("skip") {
        if let Value::Bool(b) = cow.as_ref() {
            result.skip = *b;
        }
    }

    Some(result)
}

fn convert_to_enum_value_options(value: &Value) -> Option<storage::EnumValueOptions> {
    let msg = value.as_message()?;
    let mut result = storage::EnumValueOptions::default();

    if let Some(cow) = msg.get_field_by_name("string_value") {
        if let Value::String(s) = cow.as_ref() {
            result.string_value = s.clone();
        }
    }

    if let Some(cow) = msg.get_field_by_name("int_value") {
        if let Value::I32(n) = cow.as_ref() {
            result.int_value = *n;
        }
    }

    if let Some(cow) = msg.get_field_by_name("default") {
        if let Value::Bool(b) = cow.as_ref() {
            result.default = *b;
        }
    }

    if let Some(cow) = msg.get_field_by_name("skip") {
        if let Value::Bool(b) = cow.as_ref() {
            result.skip = *b;
        }
    }

    Some(result)
}

fn convert_to_service_options(value: &Value) -> Option<storage::ServiceOptions> {
    let msg = value.as_message()?;
    let mut result = storage::ServiceOptions::default();

    if let Some(cow) = msg.get_field_by_name("generate_storage") {
        if let Value::Bool(b) = cow.as_ref() {
            result.generate_storage = *b;
        }
    }

    if let Some(cow) = msg.get_field_by_name("trait_name") {
        if let Value::String(s) = cow.as_ref() {
            result.trait_name = s.clone();
        }
    }

    if let Some(cow) = msg.get_field_by_name("skip") {
        if let Value::Bool(b) = cow.as_ref() {
            result.skip = *b;
        }
    }

    if let Some(cow) = msg.get_field_by_name("generate_implementation") {
        if let Value::Bool(b) = cow.as_ref() {
            result.generate_implementation = *b;
        }
    }

    Some(result)
}

fn convert_to_method_options(value: &Value) -> Option<storage::MethodOptions> {
    let msg = value.as_message()?;
    let mut result = storage::MethodOptions::default();

    if let Some(cow) = msg.get_field_by_name("skip") {
        if let Value::Bool(b) = cow.as_ref() {
            result.skip = *b;
        }
    }

    if let Some(cow) = msg.get_field_by_name("method_name") {
        if let Value::String(s) = cow.as_ref() {
            result.method_name = s.clone();
        }
    }

    if let Some(cow) = msg.get_field_by_name("entity_name") {
        if let Value::String(s) = cow.as_ref() {
            result.entity_name = s.clone();
        }
    }

    if let Some(cow) = msg.get_field_by_name("operation") {
        if let Value::String(s) = cow.as_ref() {
            result.operation = s.clone();
        }
    }

    Some(result)
}

fn convert_to_grpc_service_options(value: &Value) -> Option<grpc::ServiceOptions> {
    let msg = value.as_message()?;
    let mut result = grpc::ServiceOptions::default();

    if let Some(cow) = msg.get_field_by_name("skip") {
        if let Value::Bool(b) = cow.as_ref() {
            result.skip = *b;
        }
    }

    if let Some(cow) = msg.get_field_by_name("struct_name") {
        if let Value::String(s) = cow.as_ref() {
            result.struct_name = s.clone();
        }
    }

    if let Some(cow) = msg.get_field_by_name("storage_trait") {
        if let Value::String(s) = cow.as_ref() {
            result.storage_trait = s.clone();
        }
    }

    Some(result)
}

fn convert_to_grpc_method_options(value: &Value) -> Option<grpc::MethodOptions> {
    let msg = value.as_message()?;
    let mut result = grpc::MethodOptions::default();

    if let Some(cow) = msg.get_field_by_name("skip") {
        if let Value::Bool(b) = cow.as_ref() {
            result.skip = *b;
        }
    }

    if let Some(cow) = msg.get_field_by_name("method_name") {
        if let Value::String(s) = cow.as_ref() {
            result.method_name = s.clone();
        }
    }

    if let Some(cow) = msg.get_field_by_name("input_type") {
        if let Value::String(s) = cow.as_ref() {
            result.input_type = s.clone();
        }
    }

    Some(result)
}

fn convert_to_grpc_response_options(value: &Value) -> Option<grpc::ResponseOptions> {
    let msg = value.as_message()?;
    let mut result = grpc::ResponseOptions::default();

    if let Some(cow) = msg.get_field_by_name("rich_errors") {
        if let Value::Bool(b) = cow.as_ref() {
            result.rich_errors = *b;
        }
    }

    Some(result)
}

fn convert_to_graphql_type_options(value: &Value) -> Option<graphql::TypeOptions> {
    let msg = value.as_message()?;
    let mut result = graphql::TypeOptions::default();

    if let Some(cow) = msg.get_field_by_name("skip") {
        if let Value::Bool(b) = cow.as_ref() {
            result.skip = *b;
        }
    }

    if let Some(cow) = msg.get_field_by_name("name") {
        if let Value::String(s) = cow.as_ref() {
            result.name = s.clone();
        }
    }

    if let Some(cow) = msg.get_field_by_name("input") {
        if let Value::Bool(b) = cow.as_ref() {
            result.input = *b;
        }
    }

    if let Some(cow) = msg.get_field_by_name("node") {
        if let Value::Bool(b) = cow.as_ref() {
            result.node = *b;
        }
    }

    Some(result)
}

fn convert_to_graphql_field_options(value: &Value) -> Option<graphql::FieldOptions> {
    let msg = value.as_message()?;
    let mut result = graphql::FieldOptions::default();

    if let Some(cow) = msg.get_field_by_name("skip") {
        if let Value::Bool(b) = cow.as_ref() {
            result.skip = *b;
        }
    }

    if let Some(cow) = msg.get_field_by_name("name") {
        if let Value::String(s) = cow.as_ref() {
            result.name = s.clone();
        }
    }

    if let Some(cow) = msg.get_field_by_name("deprecated") {
        if let Some(dep_msg) = cow.as_ref().as_message() {
            if let Some(reason_cow) = dep_msg.get_field_by_name("reason") {
                if let Value::String(s) = reason_cow.as_ref() {
                    result.deprecated = Some(graphql::Deprecated {
                        reason: s.clone(),
                    });
                }
            }
        }
    }

    if let Some(cow) = msg.get_field_by_name("from_context") {
        if let Some(ctx_msg) = cow.as_ref().as_message() {
            let mut ctx_source = graphql::ContextSource::default();

            if let Some(path_cow) = ctx_msg.get_field_by_name("path") {
                if let Value::String(s) = path_cow.as_ref() {
                    ctx_source.path = s.clone();
                }
            }

            if let Some(req_cow) = ctx_msg.get_field_by_name("required") {
                if let Value::Bool(b) = req_cow.as_ref() {
                    ctx_source.required = *b;
                }
            }

            if let Some(err_cow) = ctx_msg.get_field_by_name("error_message") {
                if let Value::String(s) = err_cow.as_ref() {
                    ctx_source.error_message = s.clone();
                }
            }

            if !ctx_source.path.is_empty() {
                result.from_context = Some(ctx_source);
            }
        }
    }

    Some(result)
}

fn convert_to_graphql_service_options(value: &Value) -> Option<graphql::ServiceOptions> {
    let msg = value.as_message()?;
    let mut result = graphql::ServiceOptions::default();

    if let Some(cow) = msg.get_field_by_name("skip") {
        if let Value::Bool(b) = cow.as_ref() {
            result.skip = *b;
        }
    }

    Some(result)
}

fn convert_to_graphql_query_options(value: &Value) -> Option<graphql::QueryOptions> {
    let msg = value.as_message()?;
    let mut result = graphql::QueryOptions::default();

    if let Some(cow) = msg.get_field_by_name("skip") {
        if let Value::Bool(b) = cow.as_ref() {
            result.skip = *b;
        }
    }

    if let Some(cow) = msg.get_field_by_name("name") {
        if let Value::String(s) = cow.as_ref() {
            result.name = s.clone();
        }
    }

    if let Some(cow) = msg.get_field_by_name("output_type") {
        if let Value::String(s) = cow.as_ref() {
            result.output_type = s.clone();
        }
    }

    if let Some(cow) = msg.get_field_by_name("output_field") {
        if let Value::String(s) = cow.as_ref() {
            result.output_field = s.clone();
        }
    }

    Some(result)
}

fn convert_to_graphql_mutation_options(value: &Value) -> Option<graphql::MutationOptions> {
    let msg = value.as_message()?;
    let mut result = graphql::MutationOptions::default();

    if let Some(cow) = msg.get_field_by_name("skip") {
        if let Value::Bool(b) = cow.as_ref() {
            result.skip = *b;
        }
    }

    if let Some(cow) = msg.get_field_by_name("name") {
        if let Value::String(s) = cow.as_ref() {
            result.name = s.clone();
        }
    }

    if let Some(cow) = msg.get_field_by_name("input_type") {
        if let Value::String(s) = cow.as_ref() {
            result.input_type = s.clone();
        }
    }

    if let Some(cow) = msg.get_field_by_name("output_type") {
        if let Value::String(s) = cow.as_ref() {
            result.output_type = s.clone();
        }
    }

    if let Some(cow) = msg.get_field_by_name("output_field") {
        if let Value::String(s) = cow.as_ref() {
            result.output_field = s.clone();
        }
    }

    Some(result)
}

fn convert_to_graphql_subscription_options(value: &Value) -> Option<graphql::SubscriptionOptions> {
    let msg = value.as_message()?;
    let mut result = graphql::SubscriptionOptions::default();

    if let Some(cow) = msg.get_field_by_name("skip") {
        if let Value::Bool(b) = cow.as_ref() {
            result.skip = *b;
        }
    }

    if let Some(cow) = msg.get_field_by_name("name") {
        if let Value::String(s) = cow.as_ref() {
            result.name = s.clone();
        }
    }

    if let Some(cow) = msg.get_field_by_name("output_type") {
        if let Value::String(s) = cow.as_ref() {
            result.output_type = s.clone();
        }
    }

    Some(result)
}

fn convert_to_deno_config(value: &Value) -> Option<graphql::DenoConfig> {
    let msg = value.as_message()?;
    let mut result = graphql::DenoConfig::default();

    if let Some(cow) = msg.get_field_by_name("module") {
        if let Value::String(s) = cow.as_ref() {
            result.module = s.clone();
        }
    }

    if let Some(cow) = msg.get_field_by_name("function") {
        if let Value::String(s) = cow.as_ref() {
            result.function = Some(s.clone());
        }
    }

    if let Some(cow) = msg.get_field_by_name("timeout_ms") {
        if let Value::U32(n) = cow.as_ref() {
            result.timeout_ms = Some(*n);
        }
    }

    if let Some(cow) = msg.get_field_by_name("permissions") {
        if let Some(perm_msg) = cow.as_ref().as_message() {
            let mut perms = graphql::DenoPermissions::default();

            if let Some(net_cow) = perm_msg.get_field_by_name("net") {
                if let Value::List(items) = net_cow.as_ref() {
                    perms.net = items
                        .iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect();
                }
            }

            if let Some(read_cow) = perm_msg.get_field_by_name("read") {
                if let Value::List(items) = read_cow.as_ref() {
                    perms.read = items
                        .iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect();
                }
            }

            if let Some(env_cow) = perm_msg.get_field_by_name("env") {
                if let Value::List(items) = env_cow.as_ref() {
                    perms.env = items
                        .iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect();
                }
            }

            result.permissions = Some(perms);
        }
    }

    Some(result)
}

fn convert_to_virtual_field(value: &Value) -> Option<graphql::VirtualField> {
    let msg = value.as_message()?;
    let mut result = graphql::VirtualField::default();

    if let Some(cow) = msg.get_field_by_name("name") {
        if let Value::String(s) = cow.as_ref() {
            result.name = s.clone();
        }
    }

    if let Some(cow) = msg.get_field_by_name("type") {
        if let Value::String(s) = cow.as_ref() {
            result.r#type = s.clone();
        }
    }

    if let Some(cow) = msg.get_field_by_name("description") {
        if let Value::String(s) = cow.as_ref() {
            result.description = Some(s.clone());
        }
    }

    if let Some(cow) = msg.get_field_by_name("arguments") {
        if let Value::List(items) = cow.as_ref() {
            result.arguments = items
                .iter()
                .filter_map(|v| {
                    let arg_msg = v.as_message()?;
                    let mut arg = graphql::FieldArgument::default();

                    if let Some(name_cow) = arg_msg.get_field_by_name("name") {
                        if let Value::String(s) = name_cow.as_ref() {
                            arg.name = s.clone();
                        }
                    }

                    if let Some(type_cow) = arg_msg.get_field_by_name("type") {
                        if let Value::String(s) = type_cow.as_ref() {
                            arg.r#type = s.clone();
                        }
                    }

                    if let Some(default_cow) = arg_msg.get_field_by_name("default_value") {
                        if let Value::String(s) = default_cow.as_ref() {
                            arg.default_value = Some(s.clone());
                        }
                    }

                    if let Some(desc_cow) = arg_msg.get_field_by_name("description") {
                        if let Value::String(s) = desc_cow.as_ref() {
                            arg.description = Some(s.clone());
                        }
                    }

                    Some(arg)
                })
                .collect();
        }
    }

    if let Some(cow) = msg.get_field_by_name("deno") {
        result.deno = convert_to_deno_config(cow.as_ref());
    }

    Some(result)
}

fn convert_to_message_resolver_options(value: &Value) -> Option<graphql::MessageResolverOptions> {
    let msg = value.as_message()?;
    let mut result = graphql::MessageResolverOptions::default();

    if let Some(cow) = msg.get_field_by_name("fields") {
        if let Value::List(items) = cow.as_ref() {
            result.fields = items
                .iter()
                .filter_map(|v| convert_to_virtual_field(v))
                .collect();
        }
    }

    if let Some(cow) = msg.get_field_by_name("deno") {
        result.deno = convert_to_deno_config(cow.as_ref());
    }

    Some(result)
}

fn convert_to_field_resolver_options(value: &Value) -> Option<graphql::FieldResolverOptions> {
    let msg = value.as_message()?;
    let mut result = graphql::FieldResolverOptions::default();

    if let Some(cow) = msg.get_field_by_name("deno") {
        result.deno = convert_to_deno_config(cow.as_ref());
    }

    Some(result)
}

fn convert_to_method_resolver_options(value: &Value) -> Option<graphql::MethodResolverOptions> {
    let msg = value.as_message()?;
    let mut result = graphql::MethodResolverOptions::default();

    if let Some(cow) = msg.get_field_by_name("deno") {
        result.deno = convert_to_deno_config(cow.as_ref());
    }

    Some(result)
}

fn convert_to_validate_message_options(value: &Value) -> Option<validate::MessageOptions> {
    let msg = value.as_message()?;
    let mut result = validate::MessageOptions::default();

    if let Some(cow) = msg.get_field_by_name("skip") {
        if let Value::Bool(b) = cow.as_ref() {
            result.skip = *b;
        }
    }

    if let Some(cow) = msg.get_field_by_name("name") {
        if let Value::String(s) = cow.as_ref() {
            result.name = s.clone();
        }
    }

    if let Some(cow) = msg.get_field_by_name("generate_conversion") {
        if let Value::Bool(b) = cow.as_ref() {
            result.generate_conversion = *b;
        }
    }

    Some(result)
}

fn convert_to_validate_field_options(value: &Value) -> Option<validate::FieldOptions> {
    let msg = value.as_message()?;
    let mut result = validate::FieldOptions::default();

    if let Some(cow) = msg.get_field_by_name("skip") {
        if let Value::Bool(b) = cow.as_ref() {
            result.skip = *b;
        }
    }

    if let Some(cow) = msg.get_field_by_name("rename") {
        if let Value::String(s) = cow.as_ref() {
            result.rename = s.clone();
        }
    }

    if let Some(cow) = msg.get_field_by_name("type") {
        if let Value::String(s) = cow.as_ref() {
            result.r#type = s.clone();
        }
    }

    if let Some(cow) = msg.get_field_by_name("rules") {
        if let Some(rules_msg) = cow.as_ref().as_message() {
            result.rules = Some(convert_to_validate_rules(rules_msg));
        }
    }

    Some(result)
}

fn convert_to_validate_rules(msg: &DynamicMessage) -> validate::Rules {
    let mut result = validate::Rules::default();

    if let Some(cow) = msg.get_field_by_name("required") {
        if let Value::Bool(b) = cow.as_ref() {
            result.required = *b;
        }
    }

    if let Some(cow) = msg.get_field_by_name("email") {
        if let Value::Bool(b) = cow.as_ref() {
            result.email = *b;
        }
    }

    if let Some(cow) = msg.get_field_by_name("url") {
        if let Value::Bool(b) = cow.as_ref() {
            result.url = *b;
        }
    }

    if let Some(cow) = msg.get_field_by_name("uuid") {
        if let Value::Bool(b) = cow.as_ref() {
            result.uuid = *b;
        }
    }

    if let Some(cow) = msg.get_field_by_name("ascii") {
        if let Value::Bool(b) = cow.as_ref() {
            result.ascii = *b;
        }
    }

    if let Some(cow) = msg.get_field_by_name("alphanumeric") {
        if let Value::Bool(b) = cow.as_ref() {
            result.alphanumeric = *b;
        }
    }

    if let Some(cow) = msg.get_field_by_name("ip") {
        if let Value::Bool(b) = cow.as_ref() {
            result.ip = *b;
        }
    }

    if let Some(cow) = msg.get_field_by_name("ipv4") {
        if let Value::Bool(b) = cow.as_ref() {
            result.ipv4 = *b;
        }
    }

    if let Some(cow) = msg.get_field_by_name("ipv6") {
        if let Value::Bool(b) = cow.as_ref() {
            result.ipv6 = *b;
        }
    }

    if let Some(cow) = msg.get_field_by_name("credit_card") {
        if let Value::Bool(b) = cow.as_ref() {
            result.credit_card = *b;
        }
    }

    if let Some(cow) = msg.get_field_by_name("phone") {
        if let Value::Bool(b) = cow.as_ref() {
            result.phone = *b;
        }
    }

    if let Some(cow) = msg.get_field_by_name("pattern") {
        if let Value::String(s) = cow.as_ref() {
            result.pattern = s.clone();
        }
    }

    if let Some(cow) = msg.get_field_by_name("length") {
        if let Some(len_msg) = cow.as_ref().as_message() {
            result.length = Some(convert_to_length_constraint(len_msg));
        }
    }

    if let Some(cow) = msg.get_field_by_name("range") {
        if let Some(range_msg) = cow.as_ref().as_message() {
            result.range = Some(convert_to_range_constraint(range_msg));
        }
    }

    if let Some(cow) = msg.get_field_by_name("unique_items") {
        if let Value::Bool(b) = cow.as_ref() {
            result.unique_items = *b;
        }
    }

    if let Some(cow) = msg.get_field_by_name("dive") {
        if let Value::Bool(b) = cow.as_ref() {
            result.dive = *b;
        }
    }

    if let Some(cow) = msg.get_field_by_name("custom") {
        if let Value::String(s) = cow.as_ref() {
            result.custom = s.clone();
        }
    }

    if let Some(cow) = msg.get_field_by_name("message") {
        if let Value::String(s) = cow.as_ref() {
            result.message = s.clone();
        }
    }

    if let Some(cow) = msg.get_field_by_name("required_if") {
        if let Value::String(s) = cow.as_ref() {
            result.required_if = s.clone();
        }
    }

    if let Some(cow) = msg.get_field_by_name("required_unless") {
        if let Value::String(s) = cow.as_ref() {
            result.required_unless = s.clone();
        }
    }

    result
}

fn convert_to_length_constraint(msg: &DynamicMessage) -> validate::LengthConstraint {
    let mut result = validate::LengthConstraint::default();

    if let Some(cow) = msg.get_field_by_name("min") {
        if let Value::U64(n) = cow.as_ref() {
            result.min = Some(*n);
        }
    }

    if let Some(cow) = msg.get_field_by_name("max") {
        if let Value::U64(n) = cow.as_ref() {
            result.max = Some(*n);
        }
    }

    if let Some(cow) = msg.get_field_by_name("equal") {
        if let Value::U64(n) = cow.as_ref() {
            result.equal = Some(*n);
        }
    }

    result
}

fn convert_to_range_constraint(msg: &DynamicMessage) -> validate::RangeConstraint {
    let mut result = validate::RangeConstraint::default();

    if let Some(cow) = msg.get_field_by_name("min") {
        if let Value::F64(n) = cow.as_ref() {
            result.min = Some(*n);
        }
    }

    if let Some(cow) = msg.get_field_by_name("max") {
        if let Value::F64(n) = cow.as_ref() {
            result.max = Some(*n);
        }
    }

    if let Some(cow) = msg.get_field_by_name("greater_than") {
        if let Value::F64(n) = cow.as_ref() {
            result.greater_than = Some(*n);
        }
    }

    if let Some(cow) = msg.get_field_by_name("less_than") {
        if let Value::F64(n) = cow.as_ref() {
            result.less_than = Some(*n);
        }
    }

    if let Some(cow) = msg.get_field_by_name("exclusive_min") {
        if let Value::Bool(b) = cow.as_ref() {
            result.exclusive_min = *b;
        }
    }

    if let Some(cow) = msg.get_field_by_name("exclusive_max") {
        if let Value::Bool(b) = cow.as_ref() {
            result.exclusive_max = *b;
        }
    }

    result
}
