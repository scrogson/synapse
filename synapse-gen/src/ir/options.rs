// --- Storage options ---

#[derive(Debug, Clone, Default)]
pub struct ColumnOptions {
    pub primary_key: bool,
    pub auto_increment: bool,
    pub unique: bool,
    pub column_name: String,
    pub default_value: String,
    pub embed: bool,
    pub column_type: String,
    pub default_expr: String,
}

#[derive(Debug, Clone, Default)]
pub struct StorageServiceOptions {
    pub generate_storage: bool,
    pub generate_implementation: bool,
    pub trait_name: String,
    pub skip: bool,
}

#[derive(Debug, Clone, Default)]
pub struct StorageMethodOptions {
    pub skip: bool,
    pub method_name: String,
    pub entity_name: String,
    pub operation: String,
}

#[derive(Debug, Clone, Default)]
pub struct EnumStorageOptions {
    pub storage_type: EnumStorageType,
    pub skip: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub enum EnumStorageType {
    #[default]
    Unspecified,
    String,
    Integer,
}

// --- Validation options ---

#[derive(Debug, Clone, Default)]
pub struct ValidateMessageOptions {
    pub skip: bool,
    pub name: String,
    pub generate_conversion: bool,
}

#[derive(Debug, Clone, Default)]
pub struct ValidationRules {
    pub required: bool,
    pub email: bool,
    pub url: bool,
    pub uuid: bool,
    pub ascii: bool,
    pub alphanumeric: bool,
    pub ip: bool,
    pub ipv4: bool,
    pub ipv6: bool,
    pub credit_card: bool,
    pub phone: bool,
    pub pattern: String,
    pub length: Option<LengthConstraint>,
    pub range: Option<RangeConstraint>,
    pub unique_items: bool,
    pub dive: bool,
    pub custom: String,
    pub message: String,
    pub required_if: String,
    pub required_unless: String,
}

#[derive(Debug, Clone, Default)]
pub struct LengthConstraint {
    pub min: Option<u64>,
    pub max: Option<u64>,
    pub equal: Option<u64>,
}

#[derive(Debug, Clone, Default)]
pub struct RangeConstraint {
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub greater_than: Option<f64>,
    pub less_than: Option<f64>,
    pub exclusive_min: bool,
    pub exclusive_max: bool,
}

// --- gRPC options ---

#[derive(Debug, Clone, Default)]
pub struct GrpcServiceOptions {
    pub skip: bool,
    pub struct_name: String,
    pub storage_trait: String,
}

#[derive(Debug, Clone, Default)]
pub struct GrpcMethodOptions {
    pub skip: bool,
    pub method_name: String,
    pub input_type: String,
}

#[derive(Debug, Clone, Default)]
pub struct GrpcResponseOptions {
    pub rich_errors: bool,
}

// --- GraphQL options ---

#[derive(Debug, Clone, Default)]
pub struct GraphQLTypeOptions {
    pub skip: bool,
    pub name: String,
    pub input: bool,
    pub node: bool,
}

#[derive(Debug, Clone, Default)]
pub struct GraphQLFieldOptions {
    pub skip: bool,
    pub name: String,
    pub deprecated: Option<String>,
    pub from_context: Option<ContextSource>,
}

#[derive(Debug, Clone, Default)]
pub struct ContextSource {
    pub path: String,
    pub required: bool,
    pub error_message: String,
}

#[derive(Debug, Clone, Default)]
pub struct GraphQLServiceOptions {
    pub skip: bool,
}

#[derive(Debug, Clone, Default)]
pub struct GraphQLMethodOptions {
    pub kind: GraphQLMethodKind,
    pub skip: bool,
    pub name: String,
    pub input_type: String,
    pub output_type: String,
    pub output_field: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub enum GraphQLMethodKind {
    #[default]
    Query,
    Mutation,
    Subscription,
}

// --- GraphQL resolver options ---

#[derive(Debug, Clone, Default)]
pub struct GraphQLResolverOptions {
    pub fields: Vec<VirtualField>,
    pub deno: Option<DenoConfig>,
}

#[derive(Debug, Clone, Default)]
pub struct GraphQLFieldResolverOptions {
    pub deno: Option<DenoConfig>,
}

#[derive(Debug, Clone, Default)]
pub struct GraphQLMethodResolverOptions {
    pub deno: Option<DenoConfig>,
}

#[derive(Debug, Clone, Default)]
pub struct VirtualField {
    pub name: String,
    pub field_type: String,
    pub description: Option<String>,
    pub arguments: Vec<FieldArgument>,
    pub deno: Option<DenoConfig>,
}

#[derive(Debug, Clone, Default)]
pub struct FieldArgument {
    pub name: String,
    pub field_type: String,
    pub default_value: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct DenoConfig {
    pub module: String,
    pub function: Option<String>,
    pub timeout_ms: Option<u32>,
    pub permissions: Option<DenoPermissions>,
}

#[derive(Debug, Clone, Default)]
pub struct DenoPermissions {
    pub net: Vec<String>,
    pub read: Vec<String>,
    pub env: Vec<String>,
}
