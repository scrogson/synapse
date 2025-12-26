//! Proto file parsing
//!
//! Parses proto files to extract entity definitions with their fields.

use std::path::Path;

/// Information about an entity extracted from proto
#[derive(Debug, Clone)]
pub struct EntityInfo {
    /// Entity name (e.g., "User")
    pub name: String,
    /// Package name (e.g., "blog")
    pub package: String,
    /// Table name from entity options (reserved for future use)
    #[allow(dead_code)]
    pub table_name: String,
    /// Fields in the entity
    pub fields: Vec<FieldInfo>,
}

/// Information about a field in an entity
#[derive(Debug, Clone)]
pub struct FieldInfo {
    /// Field name (e.g., "email")
    pub name: String,
    /// Proto field type
    pub proto_type: ProtoType,
    /// Whether the field is optional (reserved for future use)
    #[allow(dead_code)]
    pub optional: bool,
    /// Whether this is a primary key (reserved for future use)
    #[allow(dead_code)]
    pub primary_key: bool,
}

/// Supported proto types for filter generation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ProtoType {
    Int32,
    Int64,
    Uint32,
    Uint64,
    Float,
    Double,
    Bool,
    String,
    Bytes,
    Timestamp,
    Message,
    Enum,
}

impl ProtoType {
    /// Get the filter type name for this proto type
    pub fn filter_type(&self) -> Option<&'static str> {
        match self {
            ProtoType::Int32 | ProtoType::Int64 | ProtoType::Uint32 | ProtoType::Uint64 => {
                Some("IntFilter")
            }
            ProtoType::Float | ProtoType::Double => Some("FloatFilter"),
            ProtoType::Bool => Some("BoolFilter"),
            ProtoType::String => Some("StringFilter"),
            ProtoType::Timestamp => Some("TimestampFilter"),
            _ => None,
        }
    }

    /// Whether this type supports ordering
    pub fn supports_ordering(&self) -> bool {
        matches!(
            self,
            ProtoType::Int32
                | ProtoType::Int64
                | ProtoType::Uint32
                | ProtoType::Uint64
                | ProtoType::Float
                | ProtoType::Double
                | ProtoType::String
                | ProtoType::Timestamp
        )
    }
}

/// Parse proto files and extract entity information
pub fn parse_proto_files(
    input_files: &[impl AsRef<Path>],
    include_paths: &[impl AsRef<Path>],
) -> Result<Vec<EntityInfo>, Box<dyn std::error::Error>> {
    let mut entities = Vec::new();

    // Build include paths
    let mut includes: Vec<&Path> = include_paths.iter().map(|p| p.as_ref()).collect();

    // Add default proto path
    let proto_path = Path::new("proto");
    if proto_path.exists() {
        includes.push(proto_path);
    }

    // Add current directory
    includes.push(Path::new("."));

    for input_file in input_files {
        let input_path = input_file.as_ref();

        // Read and parse the proto file
        let content = std::fs::read_to_string(input_path)?;

        // Extract package name
        let package = extract_package(&content).unwrap_or_default();

        // Parse entities from the file
        let file_entities = parse_entities_from_content(&content, &package)?;
        entities.extend(file_entities);
    }

    Ok(entities)
}

/// Extract package name from proto content
fn extract_package(content: &str) -> Option<String> {
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("package ") {
            let pkg = line
                .trim_start_matches("package ")
                .trim_end_matches(';')
                .trim();
            return Some(pkg.to_string());
        }
    }
    None
}

/// Parse entities from proto file content
fn parse_entities_from_content(
    content: &str,
    package: &str,
) -> Result<Vec<EntityInfo>, Box<dyn std::error::Error>> {
    let mut entities = Vec::new();
    let mut current_message: Option<String> = None;
    let mut current_fields: Vec<FieldInfo> = Vec::new();
    let mut is_entity = false;
    let mut table_name = String::new();
    let mut message_brace_depth = 0;
    let mut in_multi_line_option = false;

    for line in content.lines() {
        let line = line.trim();

        // Track multi-line options/fields (those that don't end with ; or { on same line)
        if in_multi_line_option {
            // Check if we're closing options with ];
            if line.contains("];") || line.ends_with("};") {
                in_multi_line_option = false;
            }
            continue;
        }

        // Track message blocks
        if line.starts_with("message ") && line.contains('{') {
            let name = line
                .trim_start_matches("message ")
                .split('{')
                .next()
                .unwrap_or("")
                .trim();

            // Skip filter/connection/request/response types - we'll generate these
            if name.ends_with("Filter")
                || name.ends_with("OrderBy")
                || name.ends_with("Edge")
                || name.ends_with("Connection")
                || name.ends_with("Request")
                || name.ends_with("Response")
                || name == "PageInfo"
                || name == "StringFilter"
                || name == "IntFilter"
                || name == "BoolFilter"
                || name == "FloatFilter"
            {
                message_brace_depth += 1;
                continue;
            }

            current_message = Some(name.to_string());
            current_fields.clear();
            is_entity = false;
            table_name.clear();
            message_brace_depth = 1;
            continue;
        }

        if current_message.is_some() {
            // Check for entity option
            if line.contains("synapse.storage.entity") {
                is_entity = true;

                // Try to extract table_name
                if let Some(start) = line.find("table_name:") {
                    let rest = &line[start + 11..];
                    if let Some(name) = extract_quoted_string(rest) {
                        table_name = name;
                    }
                }
            }

            // Skip option lines and their continuations
            // Be careful: "optional" fields also start with "option"!
            if line.starts_with("option ") || line.starts_with("option(") {
                if !line.ends_with(';') && !line.ends_with("};") {
                    in_multi_line_option = true;
                }
                continue;
            }

            // Check for closing brace of message (only at line start, not in options)
            if line == "}" || line == "};" {
                message_brace_depth -= 1;
                if message_brace_depth == 0 {
                    if is_entity {
                        let name = current_message.take().unwrap();
                        let tbl = if table_name.is_empty() {
                            heck::AsSnakeCase(&name).to_string()
                        } else {
                            table_name.clone()
                        };

                        entities.push(EntityInfo {
                            name,
                            package: package.to_string(),
                            table_name: tbl,
                            fields: std::mem::take(&mut current_fields),
                        });
                    }
                    current_message = None;
                }
                continue;
            }

            // Skip comments
            if line.starts_with("//") {
                continue;
            }

            // Parse field - must be at message level (depth 1)
            // Check if this is a field line (contains '=' and a type)
            if message_brace_depth == 1 && line.contains('=') {
                // Handle multi-line field options
                if line.contains("[(") && !line.contains("];") {
                    // Multi-line field with options - parse the field part before [
                    if let Some(field_part) = line.split("[(").next() {
                        if let Some(field) = parse_field(&format!("{};", field_part.trim())) {
                            current_fields.push(field);
                        }
                    }
                    in_multi_line_option = true;
                } else if let Some(field) = parse_field(line) {
                    current_fields.push(field);
                }
            }
        }
    }

    Ok(entities)
}

/// Extract a quoted string from text
fn extract_quoted_string(text: &str) -> Option<String> {
    let start = text.find('"')?;
    let rest = &text[start + 1..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

/// Parse a field definition line
fn parse_field(line: &str) -> Option<FieldInfo> {
    let line = line.trim();

    // Skip empty lines, comments, options, reserved
    // Note: "optional" fields also start with "option", so be specific
    if line.is_empty()
        || line.starts_with("//")
        || line.starts_with("option ")
        || line.starts_with("option(")
        || line.starts_with("reserved")
        || line.starts_with("}")
        || line.starts_with("{")
    {
        return None;
    }

    // Check for optional prefix
    let (is_optional, rest) = if line.starts_with("optional ") {
        (true, line.trim_start_matches("optional ").trim())
    } else if line.starts_with("repeated ") {
        // Skip repeated fields for now
        return None;
    } else {
        (false, line)
    };

    // Parse type and name: "type name = N [options];"
    // Split on whitespace but handle the type which may have dots (e.g., google.protobuf.Timestamp)
    let parts: Vec<&str> = rest.splitn(3, char::is_whitespace).collect();
    if parts.len() < 2 {
        return None;
    }

    let type_str = parts[0];

    // The name is before the '=' sign
    let name_part = parts[1];
    let name = name_part
        .split('=')
        .next()
        .unwrap_or(name_part)
        .trim();

    // Skip if name is empty or starts with a number
    if name.is_empty() || name.chars().next().map(|c| c.is_numeric()).unwrap_or(true) {
        return None;
    }

    let proto_type = match type_str {
        "int32" | "sint32" | "sfixed32" => ProtoType::Int32,
        "int64" | "sint64" | "sfixed64" => ProtoType::Int64,
        "uint32" | "fixed32" => ProtoType::Uint32,
        "uint64" | "fixed64" => ProtoType::Uint64,
        "float" => ProtoType::Float,
        "double" => ProtoType::Double,
        "bool" => ProtoType::Bool,
        "string" => ProtoType::String,
        "bytes" => ProtoType::Bytes,
        t if t.contains("Timestamp") => ProtoType::Timestamp,
        _ => ProtoType::Message, // Assume unknown types are messages
    };

    // Check for primary key
    let primary_key = line.contains("primary_key: true") || line.contains("primary_key:true");

    Some(FieldInfo {
        name: name.to_string(),
        proto_type,
        optional: is_optional,
        primary_key,
    })
}
