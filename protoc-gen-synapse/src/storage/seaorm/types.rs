//! Type mapping from Protocol Buffer types to Rust/SeaORM types
//!
//! This module handles the conversion of protobuf field types to their
//! corresponding Rust types for SeaORM entities.

use prost_types::field_descriptor_proto::Type;

/// Represents a mapped Rust type for SeaORM entities
#[derive(Debug, Clone)]
pub struct MappedType {
    /// The Rust type as a string (e.g., "i32", "String", "DateTimeUtc")
    pub rust_type: String,
}

/// Map a protobuf field type to a Rust type
pub fn map_proto_type(proto_type: Type, type_name: Option<&str>) -> MappedType {
    match proto_type {
        Type::Double => MappedType {
            rust_type: "f64".to_string(),
        },
        Type::Float => MappedType {
            rust_type: "f32".to_string(),
        },
        Type::Int64 | Type::Sfixed64 | Type::Sint64 => MappedType {
            rust_type: "i64".to_string(),
        },
        Type::Uint64 | Type::Fixed64 => MappedType {
            rust_type: "u64".to_string(),
        },
        Type::Int32 | Type::Sfixed32 | Type::Sint32 => MappedType {
            rust_type: "i32".to_string(),
        },
        Type::Uint32 | Type::Fixed32 => MappedType {
            rust_type: "u32".to_string(),
        },
        Type::Bool => MappedType {
            rust_type: "bool".to_string(),
        },
        Type::String => MappedType {
            rust_type: "String".to_string(),
        },
        Type::Bytes => MappedType {
            rust_type: "Vec<u8>".to_string(),
        },
        Type::Message => map_message_type(type_name),
        Type::Enum => map_enum_type(type_name),
        Type::Group => MappedType {
            rust_type: "/* unsupported group */".to_string(),
        },
    }
}

/// Map a protobuf message type to a Rust type
/// Handles well-known types like google.protobuf.Timestamp
fn map_message_type(type_name: Option<&str>) -> MappedType {
    match type_name {
        Some(".google.protobuf.Timestamp") => MappedType {
            rust_type: "DateTimeUtc".to_string(),
        },
        Some(".google.protobuf.Duration") => MappedType {
            rust_type: "TimeDelta".to_string(),
        },
        Some(".google.type.Date") => MappedType {
            rust_type: "Date".to_string(),
        },
        Some(".google.protobuf.StringValue") => MappedType {
            rust_type: "Option<String>".to_string(),
        },
        Some(".google.protobuf.Int32Value") => MappedType {
            rust_type: "Option<i32>".to_string(),
        },
        Some(".google.protobuf.Int64Value") => MappedType {
            rust_type: "Option<i64>".to_string(),
        },
        Some(".google.protobuf.BoolValue") => MappedType {
            rust_type: "Option<bool>".to_string(),
        },
        Some(name) => {
            // For other message types, use the type directly
            // In SeaORM 2.0, the type should derive FromJsonQueryResult for JSON storage
            let type_name = name.rsplit('.').next().unwrap_or(name);
            MappedType {
                rust_type: type_name.to_string(),
            }
        }
        None => MappedType {
            rust_type: "/* unknown message */".to_string(),
        },
    }
}

/// Map a protobuf enum type to a Rust type
fn map_enum_type(type_name: Option<&str>) -> MappedType {
    match type_name {
        Some(name) => {
            let enum_name = name.rsplit('.').next().unwrap_or(name);
            MappedType {
                rust_type: enum_name.to_string(),
            }
        }
        None => MappedType {
            rust_type: "i32".to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_primitive_types() {
        assert_eq!(map_proto_type(Type::Int32, None).rust_type, "i32");
        assert_eq!(map_proto_type(Type::Int64, None).rust_type, "i64");
        assert_eq!(map_proto_type(Type::String, None).rust_type, "String");
        assert_eq!(map_proto_type(Type::Bool, None).rust_type, "bool");
    }

    #[test]
    fn test_timestamp_mapping() {
        let mapped = map_proto_type(Type::Message, Some(".google.protobuf.Timestamp"));
        assert_eq!(mapped.rust_type, "DateTimeUtc");
    }
}
