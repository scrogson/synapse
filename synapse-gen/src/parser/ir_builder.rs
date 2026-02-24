//! Build Schema IR from decoded CodeGeneratorRequest and extracted options.

use std::collections::{HashMap, HashSet};

use prost_types::compiler::CodeGeneratorRequest;
use prost_types::{
    DescriptorProto, EnumDescriptorProto, FieldDescriptorProto, FileDescriptorProto,
    MethodDescriptorProto, ServiceDescriptorProto,
};

use super::extensions::ExtractedOptions;
use crate::ir::options::*;
use crate::ir::*;
use crate::options::synapse::storage as proto_storage;

// ---------------------------------------------------------------------------
// Top-level builder
// ---------------------------------------------------------------------------

pub fn build_schema<'a>(
    request: &'a CodeGeneratorRequest,
    options: &ExtractedOptions,
) -> Schema<'a> {
    let mut package_map: HashMap<String, Package<'a>> = HashMap::new();

    // Compute which packages have at least one file in file_to_generate.
    // We include ALL files from these packages so that imported files
    // (e.g. entities.proto imported by services.proto) are processed too.
    let packages_to_generate: HashSet<String> = request
        .proto_file
        .iter()
        .filter(|f| {
            let file_name = f.name.clone().unwrap_or_default();
            request.file_to_generate.contains(&file_name)
        })
        .map(|f| f.package.clone().unwrap_or_default())
        .collect();

    for file in &request.proto_file {
        let pkg_name = file.package.clone().unwrap_or_default();
        let file_name = file.name.clone().unwrap_or_default();

        if !packages_to_generate.contains(&pkg_name) {
            continue;
        }

        let package = package_map
            .entry(pkg_name.clone())
            .or_insert_with(|| Package {
                name: pkg_name.clone(),
                entities: vec![],
                services: vec![],
                enums: vec![],
                messages: vec![],
                raw_files: vec![],
            });

        package.raw_files.push(file);

        // Process top-level messages and their nested types
        for msg in &file.message_type {
            let msg_name = msg.name.clone().unwrap_or_default();
            let key = (file_name.clone(), msg_name.clone());

            if let Some(entity_opts) = options.entity_options.get(&key) {
                package
                    .entities
                    .push(build_entity(file, msg, &file_name, &msg_name, entity_opts, options));
            } else {
                package
                    .messages
                    .push(build_message(file, msg, &file_name, &msg_name, options));
            }

            // Recursively process nested messages
            process_nested_messages(file, msg, &file_name, &msg_name, options, package);
        }

        // Process top-level enums
        for enum_desc in &file.enum_type {
            package
                .enums
                .push(build_enum(file, enum_desc, &file_name, options));
        }

        // Process services
        for svc in &file.service {
            package
                .services
                .push(build_service(file, svc, &file_name, options));
        }
    }

    let mut packages: Vec<_> = package_map.into_values().collect();
    packages.sort_by(|a, b| a.name.cmp(&b.name));
    Schema { packages }
}

// ---------------------------------------------------------------------------
// Nested message processing
// ---------------------------------------------------------------------------

fn process_nested_messages<'a>(
    file: &'a FileDescriptorProto,
    parent_msg: &'a DescriptorProto,
    file_name: &str,
    parent_name: &str,
    options: &ExtractedOptions,
    package: &mut Package<'a>,
) {
    for nested in &parent_msg.nested_type {
        let nested_name = nested.name.clone().unwrap_or_default();
        let full_name = format!("{}.{}", parent_name, nested_name);
        let key = (file_name.to_string(), full_name.clone());

        if let Some(entity_opts) = options.entity_options.get(&key) {
            package
                .entities
                .push(build_entity(file, nested, file_name, &full_name, entity_opts, options));
        } else {
            package
                .messages
                .push(build_message(file, nested, file_name, &full_name, options));
        }

        // Recurse into deeper nested messages
        process_nested_messages(file, nested, file_name, &full_name, options, package);
    }
}

// ---------------------------------------------------------------------------
// Entity builder
// ---------------------------------------------------------------------------

fn build_entity<'a>(
    file: &'a FileDescriptorProto,
    msg: &'a DescriptorProto,
    file_name: &str,
    msg_name: &str,
    entity_opts: &proto_storage::EntityOptions,
    options: &ExtractedOptions,
) -> Entity<'a> {
    let fields: Vec<Field<'a>> = msg
        .field
        .iter()
        .map(|f| build_field(file_name, msg_name, f, options))
        .collect();

    let relations: Vec<Relation> = entity_opts
        .relations
        .iter()
        .map(|r| convert_relation(r))
        .collect();

    let graphql = options
        .graphql_type_options
        .get(&(file_name.to_string(), msg_name.to_string()))
        .map(|g| convert_graphql_type_options(g));

    let graphql_resolver = options
        .graphql_resolver_options
        .get(&(file_name.to_string(), msg_name.to_string()))
        .map(|r| convert_graphql_resolver_options(r));

    let validate = options
        .validate_message_options
        .get(&(file_name.to_string(), msg_name.to_string()))
        .map(|v| ValidateMessageOptions {
            skip: v.skip,
            name: v.name.clone(),
            generate_conversion: v.generate_conversion,
        });

    Entity {
        name: msg_name.to_string(),
        table_name: entity_opts.table_name.clone(),
        skip: entity_opts.skip,
        fields,
        relations,
        graphql,
        graphql_resolver,
        validate,
        raw: msg,
        raw_file: file,
    }
}

// ---------------------------------------------------------------------------
// Message builder (non-entity messages)
// ---------------------------------------------------------------------------

fn build_message<'a>(
    file: &'a FileDescriptorProto,
    msg: &'a DescriptorProto,
    file_name: &str,
    msg_name: &str,
    options: &ExtractedOptions,
) -> Message<'a> {
    let fields: Vec<Field<'a>> = msg
        .field
        .iter()
        .map(|f| build_field(file_name, msg_name, f, options))
        .collect();

    let validate = options
        .validate_message_options
        .get(&(file_name.to_string(), msg_name.to_string()))
        .map(|v| ValidateMessageOptions {
            skip: v.skip,
            name: v.name.clone(),
            generate_conversion: v.generate_conversion,
        });

    let graphql = options
        .graphql_type_options
        .get(&(file_name.to_string(), msg_name.to_string()))
        .map(|g| convert_graphql_type_options(g));

    let graphql_resolver = options
        .graphql_resolver_options
        .get(&(file_name.to_string(), msg_name.to_string()))
        .map(|r| convert_graphql_resolver_options(r));

    let grpc_response = options
        .grpc_response_options
        .get(&(file_name.to_string(), msg_name.to_string()))
        .map(|r| GrpcResponseOptions {
            rich_errors: r.rich_errors,
        });

    Message {
        name: msg_name.to_string(),
        fields,
        validate,
        graphql,
        graphql_resolver,
        grpc_response,
        raw: msg,
        raw_file: file,
    }
}

// ---------------------------------------------------------------------------
// Field builder
// ---------------------------------------------------------------------------

fn build_field<'a>(
    file_name: &str,
    msg_name: &str,
    field: &'a FieldDescriptorProto,
    options: &ExtractedOptions,
) -> Field<'a> {
    let number = field.number.unwrap_or(0);
    let name = field.name.clone().unwrap_or_default();

    let column = options
        .column_options
        .get(&(file_name.to_string(), msg_name.to_string(), number))
        .map(|c| ColumnOptions {
            primary_key: c.primary_key,
            auto_increment: c.auto_increment,
            unique: c.unique,
            column_name: c.column_name.clone(),
            default_value: c.default_value.clone(),
            embed: c.embed,
            column_type: c.column_type.clone(),
            default_expr: c.default_expr.clone(),
        });

    let validation = options
        .validate_field_options
        .get(&(file_name.to_string(), msg_name.to_string(), number))
        .map(|v| ValidationFieldOptions {
            skip: v.skip,
            rename: v.rename.clone(),
            field_type: v.r#type.clone(),
            rules: v.rules.as_ref().map(convert_validation_rules),
        });

    let graphql = options
        .graphql_field_options
        .get(&(file_name.to_string(), msg_name.to_string(), number))
        .map(|g| GraphQLFieldOptions {
            skip: g.skip,
            name: g.name.clone(),
            deprecated: g.deprecated.as_ref().map(|d| d.reason.clone()),
            from_context: g.from_context.as_ref().map(|c| ContextSource {
                path: c.path.clone(),
                required: c.required,
                error_message: c.error_message.clone(),
            }),
        });

    let graphql_resolver = options
        .graphql_field_resolver_options
        .get(&(file_name.to_string(), msg_name.to_string(), number))
        .map(|r| GraphQLFieldResolverOptions {
            deno: r.deno.as_ref().map(convert_deno_config),
        });

    Field {
        name,
        field_type: resolve_field_type(field),
        number,
        nullable: field.proto3_optional.unwrap_or(false),
        repeated: field.label == Some(3), // LABEL_REPEATED = 3
        column,
        validation,
        graphql,
        graphql_resolver,
        raw: field,
    }
}

// ---------------------------------------------------------------------------
// Enum builder
// ---------------------------------------------------------------------------

fn build_enum<'a>(
    file: &'a FileDescriptorProto,
    enum_desc: &'a EnumDescriptorProto,
    file_name: &str,
    options: &ExtractedOptions,
) -> Enum<'a> {
    let enum_name = enum_desc.name.clone().unwrap_or_default();

    let storage = options
        .enum_options
        .get(&(file_name.to_string(), enum_name.clone()))
        .map(|e| {
            let storage_type = match e.storage_type {
                1 => EnumStorageType::String,
                2 => EnumStorageType::Integer,
                _ => EnumStorageType::Unspecified,
            };
            EnumStorageOptions {
                storage_type,
                skip: e.skip,
            }
        });

    let variants: Vec<EnumVariant> = enum_desc
        .value
        .iter()
        .map(|v| {
            let value_name = v.name.clone().unwrap_or_default();
            let value_number = v.number.unwrap_or(0);

            let ev_opts = options.enum_value_options.get(&(
                file_name.to_string(),
                enum_name.clone(),
                value_number,
            ));

            EnumVariant {
                name: value_name,
                number: value_number,
                string_value: ev_opts
                    .map(|o| o.string_value.clone())
                    .unwrap_or_default(),
                int_value: ev_opts.map(|o| o.int_value).unwrap_or(0),
                is_default: ev_opts.map(|o| o.default).unwrap_or(false),
                skip: ev_opts.map(|o| o.skip).unwrap_or(false),
            }
        })
        .collect();

    Enum {
        name: enum_name,
        variants,
        storage,
        raw: enum_desc,
        raw_file: file,
    }
}

// ---------------------------------------------------------------------------
// Service builder
// ---------------------------------------------------------------------------

fn build_service<'a>(
    file: &'a FileDescriptorProto,
    svc: &'a ServiceDescriptorProto,
    file_name: &str,
    options: &ExtractedOptions,
) -> Service<'a> {
    let svc_name = svc.name.clone().unwrap_or_default();

    let storage = options
        .service_options
        .get(&(file_name.to_string(), svc_name.clone()))
        .map(|s| StorageServiceOptions {
            generate_storage: s.generate_storage,
            generate_implementation: s.generate_implementation,
            trait_name: s.trait_name.clone(),
            skip: s.skip,
        });

    let graphql = options
        .graphql_service_options
        .get(&(file_name.to_string(), svc_name.clone()))
        .map(|g| GraphQLServiceOptions { skip: g.skip });

    let grpc = options
        .grpc_service_options
        .get(&(file_name.to_string(), svc_name.clone()))
        .map(|g| GrpcServiceOptions {
            skip: g.skip,
            struct_name: g.struct_name.clone(),
            storage_trait: g.storage_trait.clone(),
        });

    let methods: Vec<Method<'a>> = svc
        .method
        .iter()
        .map(|m| build_method(file_name, &svc_name, m, options))
        .collect();

    Service {
        name: svc_name,
        methods,
        storage,
        graphql,
        grpc,
        raw: svc,
        raw_file: file,
    }
}

// ---------------------------------------------------------------------------
// Method builder
// ---------------------------------------------------------------------------

fn build_method<'a>(
    file_name: &str,
    svc_name: &str,
    method: &'a MethodDescriptorProto,
    options: &ExtractedOptions,
) -> Method<'a> {
    let method_name = method.name.clone().unwrap_or_default();
    let input_type = method.input_type.clone().unwrap_or_default();
    let output_type = method.output_type.clone().unwrap_or_default();

    let key = (
        file_name.to_string(),
        svc_name.to_string(),
        method_name.clone(),
    );

    let storage = options.method_options.get(&key).map(|m| StorageMethodOptions {
        skip: m.skip,
        method_name: m.method_name.clone(),
        entity_name: m.entity_name.clone(),
        operation: m.operation.clone(),
    });

    let grpc = options.grpc_method_options.get(&key).map(|g| GrpcMethodOptions {
        skip: g.skip,
        method_name: g.method_name.clone(),
        input_type: g.input_type.clone(),
    });

    // GraphQL method options: combine query/mutation/subscription into a single enum
    let graphql = if let Some(q) = options.graphql_query_options.get(&key) {
        Some(GraphQLMethodOptions {
            kind: GraphQLMethodKind::Query,
            skip: q.skip,
            name: q.name.clone(),
            input_type: String::new(),
            output_type: q.output_type.clone(),
            output_field: q.output_field.clone(),
        })
    } else if let Some(m) = options.graphql_mutation_options.get(&key) {
        Some(GraphQLMethodOptions {
            kind: GraphQLMethodKind::Mutation,
            skip: m.skip,
            name: m.name.clone(),
            input_type: m.input_type.clone(),
            output_type: m.output_type.clone(),
            output_field: m.output_field.clone(),
        })
    } else if let Some(s) = options.graphql_subscription_options.get(&key) {
        Some(GraphQLMethodOptions {
            kind: GraphQLMethodKind::Subscription,
            skip: s.skip,
            name: s.name.clone(),
            input_type: String::new(),
            output_type: s.output_type.clone(),
            output_field: String::new(),
        })
    } else {
        None
    };

    let graphql_resolver = options
        .graphql_method_resolver_options
        .get(&key)
        .map(|r| GraphQLMethodResolverOptions {
            deno: r.deno.as_ref().map(convert_deno_config),
        });

    Method {
        name: method_name,
        input_type,
        output_type,
        client_streaming: method.client_streaming.unwrap_or(false),
        server_streaming: method.server_streaming.unwrap_or(false),
        storage,
        graphql,
        graphql_resolver,
        grpc,
        raw: method,
    }
}

// ---------------------------------------------------------------------------
// Field type resolution
// ---------------------------------------------------------------------------

fn resolve_field_type(field: &FieldDescriptorProto) -> FieldType {
    // Check type_name first for message/enum references
    if let Some(ref type_name) = field.type_name {
        return match type_name.as_str() {
            ".google.protobuf.Timestamp" => FieldType::Timestamp,
            ".google.protobuf.Duration" => FieldType::Duration,
            ".google.protobuf.Struct" => FieldType::Struct,
            _ => {
                // type == 14 is ENUM, type == 11 is MESSAGE
                match field.r#type {
                    Some(14) => FieldType::Enum(type_name.clone()),
                    _ => FieldType::Message(type_name.clone()),
                }
            }
        };
    }

    // Scalar types
    match field.r#type {
        Some(1) => FieldType::Double,
        Some(2) => FieldType::Float,
        Some(3) => FieldType::Int64,
        Some(4) => FieldType::UInt64,
        Some(5) => FieldType::Int32,
        Some(6) => FieldType::Fixed64,
        Some(7) => FieldType::Fixed32,
        Some(8) => FieldType::Bool,
        Some(9) => FieldType::String,
        Some(12) => FieldType::Bytes,
        Some(13) => FieldType::UInt32,
        Some(15) => FieldType::SFixed32,
        Some(16) => FieldType::SFixed64,
        Some(17) => FieldType::SInt32,
        Some(18) => FieldType::SInt64,
        _ => FieldType::String, // fallback
    }
}

// ---------------------------------------------------------------------------
// Relation conversion
// ---------------------------------------------------------------------------

fn convert_relation(rel: &proto_storage::RelationDef) -> Relation {
    Relation {
        name: rel.name.clone(),
        relation_type: convert_relation_type(rel.r#type),
        related: rel.related.clone(),
        foreign_key: rel.foreign_key.clone(),
        references: rel.references.clone(),
        through: rel.through.clone(),
    }
}

fn convert_relation_type(proto_type: i32) -> RelationType {
    match proto_type {
        1 => RelationType::BelongsTo,
        2 => RelationType::HasOne,
        3 => RelationType::HasMany,
        4 => RelationType::ManyToMany,
        _ => RelationType::HasOne,
    }
}

// ---------------------------------------------------------------------------
// GraphQL options conversion
// ---------------------------------------------------------------------------

fn convert_graphql_type_options(opts: &crate::options::synapse::graphql::TypeOptions) -> GraphQLTypeOptions {
    GraphQLTypeOptions {
        skip: opts.skip,
        name: opts.name.clone(),
        input: opts.input,
        node: opts.node,
    }
}

fn convert_graphql_resolver_options(
    opts: &crate::options::synapse::graphql::MessageResolverOptions,
) -> GraphQLResolverOptions {
    GraphQLResolverOptions {
        fields: opts
            .fields
            .iter()
            .map(|f| VirtualField {
                name: f.name.clone(),
                field_type: f.r#type.clone(),
                description: f.description.clone(),
                arguments: f
                    .arguments
                    .iter()
                    .map(|a| FieldArgument {
                        name: a.name.clone(),
                        field_type: a.r#type.clone(),
                        default_value: a.default_value.clone(),
                        description: a.description.clone(),
                    })
                    .collect(),
                deno: f.deno.as_ref().map(convert_deno_config),
            })
            .collect(),
        deno: opts.deno.as_ref().map(convert_deno_config),
    }
}

// ---------------------------------------------------------------------------
// Validation rules conversion
// ---------------------------------------------------------------------------

fn convert_validation_rules(rules: &crate::options::synapse::validate::Rules) -> ValidationRules {
    ValidationRules {
        required: rules.required,
        email: rules.email,
        url: rules.url,
        uuid: rules.uuid,
        ascii: rules.ascii,
        alphanumeric: rules.alphanumeric,
        ip: rules.ip,
        ipv4: rules.ipv4,
        ipv6: rules.ipv6,
        credit_card: rules.credit_card,
        phone: rules.phone,
        pattern: rules.pattern.clone(),
        length: rules.length.as_ref().map(|l| LengthConstraint {
            min: l.min,
            max: l.max,
            equal: l.equal,
        }),
        range: rules.range.as_ref().map(|r| RangeConstraint {
            min: r.min,
            max: r.max,
            greater_than: r.greater_than,
            less_than: r.less_than,
            exclusive_min: r.exclusive_min,
            exclusive_max: r.exclusive_max,
        }),
        unique_items: rules.unique_items,
        dive: rules.dive,
        custom: rules.custom.clone(),
        message: rules.message.clone(),
        required_if: rules.required_if.clone(),
        required_unless: rules.required_unless.clone(),
    }
}

// ---------------------------------------------------------------------------
// Deno config conversion
// ---------------------------------------------------------------------------

fn convert_deno_config(config: &crate::options::synapse::graphql::DenoConfig) -> DenoConfig {
    DenoConfig {
        module: config.module.clone(),
        function: config.function.clone(),
        timeout_ms: config.timeout_ms,
        permissions: config.permissions.as_ref().map(|p| DenoPermissions {
            net: p.net.clone(),
            read: p.read.clone(),
            env: p.env.clone(),
        }),
    }
}
