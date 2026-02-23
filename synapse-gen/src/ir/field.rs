use prost_types::FieldDescriptorProto;
use super::options::{ColumnOptions, ValidationRules, GraphQLFieldOptions, GraphQLFieldResolverOptions};

#[derive(Debug, Clone)]
pub struct Field<'a> {
    pub name: String,
    pub field_type: FieldType,
    pub number: i32,
    pub nullable: bool,
    pub repeated: bool,
    pub column: Option<ColumnOptions>,
    pub validation: Option<ValidationFieldOptions>,
    pub graphql: Option<GraphQLFieldOptions>,
    pub graphql_resolver: Option<GraphQLFieldResolverOptions>,
    pub raw: &'a FieldDescriptorProto,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FieldType {
    Int32,
    Int64,
    UInt32,
    UInt64,
    SInt32,
    SInt64,
    Fixed32,
    Fixed64,
    SFixed32,
    SFixed64,
    Float,
    Double,
    Bool,
    String,
    Bytes,
    Timestamp,
    Duration,
    Struct,
    Enum(std::string::String),
    Message(std::string::String),
}

#[derive(Debug, Clone)]
pub struct ValidationFieldOptions {
    pub skip: bool,
    pub rename: String,
    pub field_type: String,
    pub rules: Option<ValidationRules>,
}
