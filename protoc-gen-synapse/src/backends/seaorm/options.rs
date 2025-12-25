//! Options parsing for Synapse Storage protobuf extensions
//!
//! This module handles parsing of `(synapse.storage.entity)`, `(synapse.storage.column)`,
//! `(synapse.storage.enum_storage)`, `(synapse.storage.enum_value_storage)`,
//! `(synapse.storage.service_storage)`, and `(synapse.storage.method_storage)` options
//! from protobuf descriptors.
//!
//! Custom protobuf extensions are stored as extension fields in the options
//! messages. We use prost-reflect to decode these extensions from the raw
//! protobuf bytes.

pub use crate::options::synapse::storage;
pub use crate::options::synapse::{grpc, validate};
use once_cell::sync::Lazy;
use prost_reflect::{DescriptorPool, DynamicMessage, Value};
use prost_types::{
    DescriptorProto, EnumDescriptorProto, EnumValueDescriptorProto, FieldDescriptorProto,
    ServiceDescriptorProto, UninterpretedOption,
};
use std::collections::HashMap;
use std::sync::RwLock;

/// File descriptor set bytes generated at build time by protoc
static FILE_DESCRIPTOR_SET_BYTES: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/file_descriptor_set.bin"));

/// Extension names for synapse.storage options
const ENTITY_EXTENSION_NAME: &str = "synapse.storage.entity";
const COLUMN_EXTENSION_NAME: &str = "synapse.storage.column";
const ENUM_EXTENSION_NAME: &str = "synapse.storage.enum_storage";
const ENUM_VALUE_EXTENSION_NAME: &str = "synapse.storage.enum_value_storage";
const SERVICE_EXTENSION_NAME: &str = "synapse.storage.service_storage";
const METHOD_EXTENSION_NAME: &str = "synapse.storage.method_storage";

// gRPC extension names
const GRPC_SERVICE_EXTENSION_NAME: &str = "synapse.grpc.service";
const GRPC_METHOD_EXTENSION_NAME: &str = "synapse.grpc.method";

// Validate extension names
const VALIDATE_MESSAGE_EXTENSION_NAME: &str = "synapse.validate.message";

/// Lazily initialized descriptor pool with our extension definitions
static DESCRIPTOR_POOL: Lazy<DescriptorPool> = Lazy::new(|| {
    DescriptorPool::decode(FILE_DESCRIPTOR_SET_BYTES).expect("Failed to decode file descriptor set")
});

/// Global cache of pre-parsed options from raw bytes
static OPTIONS_CACHE: Lazy<RwLock<OptionsCache>> =
    Lazy::new(|| RwLock::new(OptionsCache::default()));

/// Cache structure holding pre-parsed options
#[derive(Default)]
struct OptionsCache {
    /// Entity options: (file_name, message_name) -> EntityOptions
    entity_options: HashMap<(String, String), storage::EntityOptions>,
    /// Column options: (file_name, message_name, field_number) -> ColumnOptions
    column_options: HashMap<(String, String, i32), storage::ColumnOptions>,
    /// Enum options: (file_name, enum_name) -> EnumOptions
    enum_options: HashMap<(String, String), storage::EnumOptions>,
    /// Enum value options: (file_name, enum_name, value_number) -> EnumValueOptions
    enum_value_options: HashMap<(String, String, i32), storage::EnumValueOptions>,
    /// Service options: (file_name, service_name) -> ServiceOptions
    service_options: HashMap<(String, String), storage::ServiceOptions>,
    /// Method options: (file_name, service_name, method_name) -> MethodOptions
    method_options: HashMap<(String, String, String), storage::MethodOptions>,
    /// gRPC service options: (file_name, service_name) -> grpc::ServiceOptions
    grpc_service_options: HashMap<(String, String), grpc::ServiceOptions>,
    /// gRPC method options: (file_name, service_name, method_name) -> grpc::MethodOptions
    grpc_method_options: HashMap<(String, String, String), grpc::MethodOptions>,
    /// Validate message options: (file_name, message_name) -> validate::MessageOptions
    validate_message_options: HashMap<(String, String), validate::MessageOptions>,
}

/// Pre-process raw CodeGeneratorRequest bytes to extract options using prost-reflect
///
/// This must be called before `generate()` to populate the options cache with
/// extension data that would otherwise be lost when prost decodes the request.
pub fn preprocess_request_bytes(bytes: &[u8]) -> Result<(), String> {
    // Get the CodeGeneratorRequest descriptor
    let request_desc = DESCRIPTOR_POOL
        .get_message_by_name("google.protobuf.compiler.CodeGeneratorRequest")
        .ok_or("CodeGeneratorRequest not found in descriptor pool")?;

    // Decode the request as a DynamicMessage
    let request = DynamicMessage::decode(request_desc, bytes)
        .map_err(|e| format!("Failed to decode CodeGeneratorRequest: {}", e))?;

    let mut cache = OPTIONS_CACHE
        .write()
        .map_err(|e| format!("Lock error: {}", e))?;

    // Get proto_file field
    if let Some(cow) = request.get_field_by_name("proto_file") {
        if let Value::List(files) = cow.as_ref() {
            for file_value in files.iter() {
                if let Some(file_msg) = file_value.as_message() {
                    extract_options_from_file(&mut cache, file_msg)?;
                }
            }
        }
    }

    Ok(())
}

/// Extract options from a FileDescriptorProto DynamicMessage
fn extract_options_from_file(
    cache: &mut OptionsCache,
    file: &DynamicMessage,
) -> Result<(), String> {
    let file_name = file
        .get_field_by_name("name")
        .and_then(|v| v.as_ref().as_str().map(|s| s.to_string()))
        .unwrap_or_default();

    // Extract message options
    if let Some(cow) = file.get_field_by_name("message_type") {
        if let Value::List(messages) = cow.as_ref() {
            for msg_value in messages.iter() {
                if let Some(msg) = msg_value.as_message() {
                    extract_message_options(cache, &file_name, msg, "")?;
                }
            }
        }
    }

    // Extract enum options
    if let Some(cow) = file.get_field_by_name("enum_type") {
        if let Value::List(enums) = cow.as_ref() {
            for enum_value in enums.iter() {
                if let Some(enum_msg) = enum_value.as_message() {
                    extract_enum_options(cache, &file_name, enum_msg)?;
                }
            }
        }
    }

    // Extract service options
    if let Some(cow) = file.get_field_by_name("service") {
        if let Value::List(services) = cow.as_ref() {
            for service_value in services.iter() {
                if let Some(service_msg) = service_value.as_message() {
                    extract_service_options(cache, &file_name, service_msg)?;
                }
            }
        }
    }

    Ok(())
}

/// Extract options from a DescriptorProto DynamicMessage
fn extract_message_options(
    cache: &mut OptionsCache,
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

    // Extract entity options (synapse.storage.entity)
    if let Some(cow) = msg.get_field_by_name("options") {
        if let Some(opts_msg) = cow.as_ref().as_message() {
            if let Some(ext_field) = DESCRIPTOR_POOL.get_extension_by_name(ENTITY_EXTENSION_NAME) {
                if opts_msg.has_extension(&ext_field) {
                    let ext_value = opts_msg.get_extension(&ext_field);
                    if let Some(entity_opts) = convert_to_entity_options(&ext_value) {
                        cache
                            .entity_options
                            .insert((file_name.to_string(), full_name.clone()), entity_opts);
                    }
                }
            }

            // Extract validate message options (synapse.validate.message)
            if let Some(ext_field) =
                DESCRIPTOR_POOL.get_extension_by_name(VALIDATE_MESSAGE_EXTENSION_NAME)
            {
                if opts_msg.has_extension(&ext_field) {
                    let ext_value = opts_msg.get_extension(&ext_field);
                    if let Some(validate_opts) = convert_to_validate_message_options(&ext_value) {
                        cache.validate_message_options.insert(
                            (file_name.to_string(), full_name.clone()),
                            validate_opts,
                        );
                    }
                }
            }
        }
    }

    // Extract field-level options (synapse.storage.column)
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
                            // Extract synapse.storage.column options
                            if let Some(ext_field) =
                                DESCRIPTOR_POOL.get_extension_by_name(COLUMN_EXTENSION_NAME)
                            {
                                if opts_msg.has_extension(&ext_field) {
                                    let ext_value = opts_msg.get_extension(&ext_field);
                                    if let Some(col_opts) = convert_to_column_options(&ext_value) {
                                        cache.column_options.insert(
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
                        }
                    }
                }
            }
        }
    }

    // Process nested messages
    if let Some(cow) = msg.get_field_by_name("nested_type") {
        if let Value::List(nested) = cow.as_ref() {
            for nested_value in nested.iter() {
                if let Some(nested_msg) = nested_value.as_message() {
                    extract_message_options(cache, file_name, nested_msg, &full_name)?;
                }
            }
        }
    }

    // Process nested enums
    if let Some(cow) = msg.get_field_by_name("enum_type") {
        if let Value::List(enums) = cow.as_ref() {
            for enum_value in enums.iter() {
                if let Some(enum_msg) = enum_value.as_message() {
                    extract_enum_options_nested(cache, file_name, enum_msg, &full_name)?;
                }
            }
        }
    }

    Ok(())
}

/// Extract options from an EnumDescriptorProto DynamicMessage
fn extract_enum_options(
    cache: &mut OptionsCache,
    file_name: &str,
    enum_msg: &DynamicMessage,
) -> Result<(), String> {
    extract_enum_options_nested(cache, file_name, enum_msg, "")
}

/// Extract options from an EnumDescriptorProto with optional parent prefix
fn extract_enum_options_nested(
    cache: &mut OptionsCache,
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

    // Extract enum-level options (synapse.storage.enum_storage)
    if let Some(cow) = enum_msg.get_field_by_name("options") {
        if let Some(opts_msg) = cow.as_ref().as_message() {
            if let Some(ext_field) = DESCRIPTOR_POOL.get_extension_by_name(ENUM_EXTENSION_NAME) {
                if opts_msg.has_extension(&ext_field) {
                    let ext_value = opts_msg.get_extension(&ext_field);
                    if let Some(enum_opts) = convert_to_enum_options(&ext_value) {
                        cache
                            .enum_options
                            .insert((file_name.to_string(), full_name.clone()), enum_opts);
                    }
                }
            }
        }
    }

    // Extract enum value options (synapse.storage.enum_value_storage)
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
                                        cache.enum_value_options.insert(
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

/// Extract options from a ServiceDescriptorProto DynamicMessage
fn extract_service_options(
    cache: &mut OptionsCache,
    file_name: &str,
    service: &DynamicMessage,
) -> Result<(), String> {
    let service_name = service
        .get_field_by_name("name")
        .and_then(|v| v.as_ref().as_str().map(|s| s.to_string()))
        .unwrap_or_default();

    // Extract service-level options (synapse.storage.service_storage)
    if let Some(cow) = service.get_field_by_name("options") {
        if let Some(opts_msg) = cow.as_ref().as_message() {
            if let Some(ext_field) = DESCRIPTOR_POOL.get_extension_by_name(SERVICE_EXTENSION_NAME) {
                if opts_msg.has_extension(&ext_field) {
                    let ext_value = opts_msg.get_extension(&ext_field);
                    if let Some(service_opts) = convert_to_service_options(&ext_value) {
                        cache
                            .service_options
                            .insert((file_name.to_string(), service_name.clone()), service_opts);
                    }
                }
            }

            // Extract gRPC service options (synapse.grpc.service)
            if let Some(ext_field) =
                DESCRIPTOR_POOL.get_extension_by_name(GRPC_SERVICE_EXTENSION_NAME)
            {
                if opts_msg.has_extension(&ext_field) {
                    let ext_value = opts_msg.get_extension(&ext_field);
                    if let Some(grpc_opts) = convert_to_grpc_service_options(&ext_value) {
                        cache.grpc_service_options.insert(
                            (file_name.to_string(), service_name.clone()),
                            grpc_opts,
                        );
                    }
                }
            }
        }
    }

    // Extract method-level options (synapse.storage.method_storage)
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
                            // Extract storage method options
                            if let Some(ext_field) =
                                DESCRIPTOR_POOL.get_extension_by_name(METHOD_EXTENSION_NAME)
                            {
                                if opts_msg.has_extension(&ext_field) {
                                    let ext_value = opts_msg.get_extension(&ext_field);
                                    if let Some(method_opts) =
                                        convert_to_method_options(&ext_value)
                                    {
                                        cache.method_options.insert(
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

                            // Extract gRPC method options (synapse.grpc.method)
                            if let Some(ext_field) =
                                DESCRIPTOR_POOL.get_extension_by_name(GRPC_METHOD_EXTENSION_NAME)
                            {
                                if opts_msg.has_extension(&ext_field) {
                                    let ext_value = opts_msg.get_extension(&ext_field);
                                    if let Some(grpc_method_opts) =
                                        convert_to_grpc_method_options(&ext_value)
                                    {
                                        cache.grpc_method_options.insert(
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
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

// =============================================================================
// Cached option lookups
// =============================================================================

/// Look up cached entity options for a given file and message name
pub fn get_cached_entity_options(
    file_name: &str,
    msg_name: &str,
) -> Option<storage::EntityOptions> {
    OPTIONS_CACHE.read().ok().and_then(|cache| {
        cache
            .entity_options
            .get(&(file_name.to_string(), msg_name.to_string()))
            .cloned()
    })
}

/// Look up cached column options for a given file, message name, and field number
pub fn get_cached_column_options(
    file_name: &str,
    msg_name: &str,
    field_number: i32,
) -> Option<storage::ColumnOptions> {
    OPTIONS_CACHE.read().ok().and_then(|cache| {
        cache
            .column_options
            .get(&(file_name.to_string(), msg_name.to_string(), field_number))
            .cloned()
    })
}

/// Look up cached service options for a given file and service name
pub fn get_cached_service_options(
    file_name: &str,
    service_name: &str,
) -> Option<storage::ServiceOptions> {
    OPTIONS_CACHE.read().ok().and_then(|cache| {
        cache
            .service_options
            .get(&(file_name.to_string(), service_name.to_string()))
            .cloned()
    })
}

/// Parse storage service options from a ServiceDescriptorProto
pub fn parse_service_options(service: &ServiceDescriptorProto) -> Option<storage::ServiceOptions> {
    let opts = service.options.as_ref()?;

    // Fallback to uninterpreted_option (main path for unit tests)
    parse_service_options_from_uninterpreted(&opts.uninterpreted_option)
}

/// Look up cached method options for a given file, service name, and method name
pub fn get_cached_rpc_method_options(
    file_name: &str,
    service_name: &str,
    method_name: &str,
) -> Option<storage::MethodOptions> {
    OPTIONS_CACHE.read().ok().and_then(|cache| {
        cache
            .method_options
            .get(&(
                file_name.to_string(),
                service_name.to_string(),
                method_name.to_string(),
            ))
            .cloned()
    })
}

/// Look up cached gRPC service options for a given file and service name
pub fn get_cached_grpc_service_options(
    file_name: &str,
    service_name: &str,
) -> Option<grpc::ServiceOptions> {
    OPTIONS_CACHE.read().ok().and_then(|cache| {
        cache
            .grpc_service_options
            .get(&(file_name.to_string(), service_name.to_string()))
            .cloned()
    })
}

/// Look up cached gRPC method options for a given file, service, and method name
pub fn get_cached_grpc_method_options(
    file_name: &str,
    service_name: &str,
    method_name: &str,
) -> Option<grpc::MethodOptions> {
    OPTIONS_CACHE.read().ok().and_then(|cache| {
        cache
            .grpc_method_options
            .get(&(
                file_name.to_string(),
                service_name.to_string(),
                method_name.to_string(),
            ))
            .cloned()
    })
}

/// Look up cached validate message options for a given file and message name
pub fn get_cached_validate_message_options(
    file_name: &str,
    msg_name: &str,
) -> Option<validate::MessageOptions> {
    OPTIONS_CACHE.read().ok().and_then(|cache| {
        cache
            .validate_message_options
            .get(&(file_name.to_string(), msg_name.to_string()))
            .cloned()
    })
}

// =============================================================================
// Parse from descriptor (for tests and fallback)
// =============================================================================

/// Parse entity options from a DescriptorProto
pub fn parse_entity_options(desc: &DescriptorProto) -> Option<storage::EntityOptions> {
    let opts = desc.options.as_ref()?;
    parse_entity_options_from_uninterpreted(&opts.uninterpreted_option)
}

/// Parse column options from a FieldDescriptorProto
pub fn parse_column_options(field: &FieldDescriptorProto) -> Option<storage::ColumnOptions> {
    let opts = field.options.as_ref()?;
    parse_column_options_from_uninterpreted(&opts.uninterpreted_option)
}

/// Parse enum options from an EnumDescriptorProto
pub fn parse_enum_options(enum_desc: &EnumDescriptorProto) -> Option<storage::EnumOptions> {
    let opts = enum_desc.options.as_ref()?;
    parse_enum_options_from_uninterpreted(&opts.uninterpreted_option)
}

/// Parse enum value options from an EnumValueDescriptorProto
pub fn parse_enum_value_options(
    value: &EnumValueDescriptorProto,
) -> Option<storage::EnumValueOptions> {
    let opts = value.options.as_ref()?;
    parse_enum_value_options_from_uninterpreted(&opts.uninterpreted_option)
}

// =============================================================================
// Value conversion helpers
// =============================================================================

/// Convert a prost-reflect Value to EntityOptions
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

/// Convert a prost-reflect Value to a RelationDef
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

/// Convert a prost-reflect Value to ColumnOptions
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

/// Convert a prost-reflect Value to EnumOptions
fn convert_to_enum_options(value: &Value) -> Option<storage::EnumOptions> {
    let msg = value.as_message()?;
    let mut result = storage::EnumOptions::default();

    if let Some(cow) = msg.get_field_by_name("db_type") {
        if let Value::EnumNumber(n) = cow.as_ref() {
            result.db_type = *n;
        }
    }

    if let Some(cow) = msg.get_field_by_name("skip") {
        if let Value::Bool(b) = cow.as_ref() {
            result.skip = *b;
        }
    }

    Some(result)
}

/// Convert a prost-reflect Value to EnumValueOptions
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

    Some(result)
}

/// Convert a prost-reflect Value to ServiceOptions
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

    Some(result)
}

/// Convert a prost-reflect Value to MethodOptions
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

    Some(result)
}

/// Convert a prost-reflect Value to grpc::ServiceOptions
fn convert_to_grpc_service_options(value: &Value) -> Option<grpc::ServiceOptions> {
    let msg = value.as_message()?;
    let mut result = grpc::ServiceOptions::default();

    if let Some(cow) = msg.get_field_by_name("generate_tonic") {
        if let Value::Bool(b) = cow.as_ref() {
            result.generate_tonic = *b;
        }
    }

    if let Some(cow) = msg.get_field_by_name("struct_name") {
        if let Value::String(s) = cow.as_ref() {
            result.struct_name = s.clone();
        }
    }

    if let Some(cow) = msg.get_field_by_name("skip") {
        if let Value::Bool(b) = cow.as_ref() {
            result.skip = *b;
        }
    }

    if let Some(cow) = msg.get_field_by_name("storage_trait") {
        if let Value::String(s) = cow.as_ref() {
            result.storage_trait = s.clone();
        }
    }

    Some(result)
}

/// Convert a prost-reflect Value to grpc::MethodOptions
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

/// Convert a prost-reflect Value to validate::MessageOptions
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

// =============================================================================
// Fallback: Uninterpreted option parsing (for older protoc versions and tests)
// =============================================================================

/// Check if an uninterpreted option matches our extension name
fn is_extension_option(opt: &UninterpretedOption, extension_name: &str) -> bool {
    if opt.name.is_empty() {
        return false;
    }

    let first = &opt.name[0];
    if !first.is_extension {
        return false;
    }

    first.name_part == extension_name
}

/// Get the sub-field name from an uninterpreted option
fn get_subfield_name(opt: &UninterpretedOption) -> Option<&str> {
    if opt.name.len() >= 2 {
        Some(opt.name[1].name_part.as_str())
    } else {
        None
    }
}

/// Parse EntityOptions from uninterpreted options
fn parse_entity_options_from_uninterpreted(
    uninterpreted: &[UninterpretedOption],
) -> Option<storage::EntityOptions> {
    let mut result = storage::EntityOptions::default();
    let mut found = false;

    for opt in uninterpreted {
        if is_extension_option(opt, ENTITY_EXTENSION_NAME) {
            found = true;
            apply_entity_option(&mut result, opt);
        }
    }

    if found {
        Some(result)
    } else {
        None
    }
}

/// Parse ColumnOptions from uninterpreted options
fn parse_column_options_from_uninterpreted(
    uninterpreted: &[UninterpretedOption],
) -> Option<storage::ColumnOptions> {
    let mut result = storage::ColumnOptions::default();
    let mut found = false;

    for opt in uninterpreted {
        if is_extension_option(opt, COLUMN_EXTENSION_NAME) {
            found = true;
            apply_column_option(&mut result, opt);
        }
    }

    if found {
        Some(result)
    } else {
        None
    }
}

/// Parse EnumOptions from uninterpreted options
fn parse_enum_options_from_uninterpreted(
    uninterpreted: &[UninterpretedOption],
) -> Option<storage::EnumOptions> {
    let mut result = storage::EnumOptions::default();
    let mut found = false;

    for opt in uninterpreted {
        if is_extension_option(opt, ENUM_EXTENSION_NAME) {
            found = true;
            apply_enum_option(&mut result, opt);
        }
    }

    if found {
        Some(result)
    } else {
        None
    }
}

/// Parse EnumValueOptions from uninterpreted options
fn parse_enum_value_options_from_uninterpreted(
    uninterpreted: &[UninterpretedOption],
) -> Option<storage::EnumValueOptions> {
    let mut result = storage::EnumValueOptions::default();
    let mut found = false;

    for opt in uninterpreted {
        if is_extension_option(opt, ENUM_VALUE_EXTENSION_NAME) {
            found = true;
            apply_enum_value_option(&mut result, opt);
        }
    }

    if found {
        Some(result)
    } else {
        None
    }
}

/// Parse ServiceOptions from uninterpreted options
fn parse_service_options_from_uninterpreted(
    uninterpreted: &[UninterpretedOption],
) -> Option<storage::ServiceOptions> {
    let mut result = storage::ServiceOptions::default();
    let mut found = false;

    for opt in uninterpreted {
        if is_extension_option(opt, SERVICE_EXTENSION_NAME) {
            found = true;
            apply_service_option(&mut result, opt);
        }
    }

    if found {
        Some(result)
    } else {
        None
    }
}

// =============================================================================
// Apply options helpers
// =============================================================================

fn apply_entity_option(result: &mut storage::EntityOptions, opt: &UninterpretedOption) {
    if let Some(aggregate) = opt.aggregate_value.as_ref() {
        parse_aggregate_into_entity_options(result, aggregate);
    } else if let Some(field_name) = get_subfield_name(opt) {
        match field_name {
            "table_name" => result.table_name = parse_string_option(opt),
            "skip" => result.skip = parse_bool_option(opt),
            _ => {}
        }
    }
}

fn apply_column_option(result: &mut storage::ColumnOptions, opt: &UninterpretedOption) {
    if let Some(aggregate) = opt.aggregate_value.as_ref() {
        parse_aggregate_into_column_options(result, aggregate);
    } else if let Some(field_name) = get_subfield_name(opt) {
        match field_name {
            "primary_key" => result.primary_key = parse_bool_option(opt),
            "auto_increment" => result.auto_increment = parse_bool_option(opt),
            "unique" => result.unique = parse_bool_option(opt),
            "column_name" => result.column_name = parse_string_option(opt),
            "default_value" => result.default_value = parse_string_option(opt),
            "embed" => result.embed = parse_bool_option(opt),
            "column_type" => result.column_type = parse_string_option(opt),
            "default_expr" => result.default_expr = parse_string_option(opt),
            _ => {}
        }
    }
}

fn apply_enum_option(result: &mut storage::EnumOptions, opt: &UninterpretedOption) {
    if let Some(aggregate) = opt.aggregate_value.as_ref() {
        parse_aggregate_into_enum_options(result, aggregate);
    } else if let Some(field_name) = get_subfield_name(opt) {
        match field_name {
            "db_type" => {
                // Parse enum value
                let s = parse_string_option(opt);
                result.db_type = match s.as_str() {
                    "ENUM_DB_TYPE_STRING" => storage::EnumDbType::String as i32,
                    "ENUM_DB_TYPE_INTEGER" => storage::EnumDbType::Integer as i32,
                    _ => 0,
                };
            }
            "skip" => result.skip = parse_bool_option(opt),
            _ => {}
        }
    }
}

fn apply_enum_value_option(result: &mut storage::EnumValueOptions, opt: &UninterpretedOption) {
    if let Some(aggregate) = opt.aggregate_value.as_ref() {
        parse_aggregate_into_enum_value_options(result, aggregate);
    } else if let Some(field_name) = get_subfield_name(opt) {
        match field_name {
            "string_value" => result.string_value = parse_string_option(opt),
            "int_value" => result.int_value = parse_int_option(opt),
            _ => {}
        }
    }
}

fn apply_service_option(result: &mut storage::ServiceOptions, opt: &UninterpretedOption) {
    if let Some(aggregate) = opt.aggregate_value.as_ref() {
        parse_aggregate_into_service_options(result, aggregate);
    } else if let Some(field_name) = get_subfield_name(opt) {
        match field_name {
            "generate_storage" => result.generate_storage = parse_bool_option(opt),
            "trait_name" => result.trait_name = parse_string_option(opt),
            "skip" => result.skip = parse_bool_option(opt),
            _ => {}
        }
    }
}

// =============================================================================
// Aggregate parsing helpers
// =============================================================================

fn parse_bool_option(opt: &UninterpretedOption) -> bool {
    if let Some(ref v) = opt.identifier_value {
        return v == "true";
    }
    if let Some(v) = opt.positive_int_value {
        return v != 0;
    }
    false
}

fn parse_string_option(opt: &UninterpretedOption) -> String {
    if let Some(ref s) = opt.string_value {
        return String::from_utf8_lossy(s).to_string();
    }
    if let Some(ref s) = opt.identifier_value {
        return s.clone();
    }
    String::new()
}

fn parse_int_option(opt: &UninterpretedOption) -> i32 {
    if let Some(v) = opt.positive_int_value {
        return v as i32;
    }
    if let Some(v) = opt.negative_int_value {
        return v as i32;
    }
    0
}

fn split_aggregate_parts(aggregate: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut brace_depth: i32 = 0;
    let mut bracket_depth: i32 = 0;

    for (i, c) in aggregate.char_indices() {
        match c {
            '{' => brace_depth += 1,
            '}' => brace_depth = brace_depth.saturating_sub(1),
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            ',' if brace_depth == 0 && bracket_depth == 0 => {
                parts.push(&aggregate[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }

    if start < aggregate.len() {
        parts.push(&aggregate[start..]);
    }

    parts
}

fn parse_quoted_string(s: &str) -> String {
    let s = s.trim();
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

fn parse_aggregate_into_entity_options(result: &mut storage::EntityOptions, aggregate: &str) {
    for part in split_aggregate_parts(aggregate) {
        let (key, value) = match part.split_once(':') {
            Some((k, v)) => (k.trim(), v.trim()),
            None => continue,
        };

        match key {
            "table_name" => result.table_name = parse_quoted_string(value),
            "skip" => result.skip = value == "true",
            _ => {}
        }
    }
}

fn parse_aggregate_into_column_options(result: &mut storage::ColumnOptions, aggregate: &str) {
    for part in split_aggregate_parts(aggregate) {
        let (key, value) = match part.split_once(':') {
            Some((k, v)) => (k.trim(), v.trim()),
            None => continue,
        };

        match key {
            "primary_key" => result.primary_key = value == "true",
            "auto_increment" => result.auto_increment = value == "true",
            "unique" => result.unique = value == "true",
            "column_name" => result.column_name = parse_quoted_string(value),
            "default_value" => result.default_value = parse_quoted_string(value),
            "embed" => result.embed = value == "true",
            "column_type" => result.column_type = parse_quoted_string(value),
            "default_expr" => result.default_expr = parse_quoted_string(value),
            _ => {}
        }
    }
}

fn parse_aggregate_into_enum_options(result: &mut storage::EnumOptions, aggregate: &str) {
    for part in split_aggregate_parts(aggregate) {
        let (key, value) = match part.split_once(':') {
            Some((k, v)) => (k.trim(), v.trim()),
            None => continue,
        };

        match key {
            "db_type" => {
                result.db_type = match value {
                    "ENUM_DB_TYPE_STRING" => storage::EnumDbType::String as i32,
                    "ENUM_DB_TYPE_INTEGER" => storage::EnumDbType::Integer as i32,
                    _ => 0,
                };
            }
            "skip" => result.skip = value == "true",
            _ => {}
        }
    }
}

fn parse_aggregate_into_enum_value_options(result: &mut storage::EnumValueOptions, aggregate: &str) {
    for part in split_aggregate_parts(aggregate) {
        let (key, value) = match part.split_once(':') {
            Some((k, v)) => (k.trim(), v.trim()),
            None => continue,
        };

        match key {
            "string_value" => result.string_value = parse_quoted_string(value),
            "int_value" => {
                if let Ok(v) = value.parse::<i32>() {
                    result.int_value = v;
                }
            }
            _ => {}
        }
    }
}

fn parse_aggregate_into_service_options(result: &mut storage::ServiceOptions, aggregate: &str) {
    for part in split_aggregate_parts(aggregate) {
        let (key, value) = match part.split_once(':') {
            Some((k, v)) => (k.trim(), v.trim()),
            None => continue,
        };

        match key {
            "generate_storage" => result.generate_storage = value == "true",
            "trait_name" => result.trait_name = parse_quoted_string(value),
            "skip" => result.skip = value == "true",
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_quoted_string() {
        assert_eq!(parse_quoted_string("\"hello\""), "hello");
        assert_eq!(parse_quoted_string("'world'"), "world");
        assert_eq!(parse_quoted_string("unquoted"), "unquoted");
    }

    #[test]
    fn test_split_aggregate_parts() {
        let parts = split_aggregate_parts("key1: value1, key2: value2");
        assert_eq!(parts.len(), 2);
    }
}
