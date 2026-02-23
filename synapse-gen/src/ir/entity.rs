use prost_types::{DescriptorProto, FileDescriptorProto};
use super::{Field, Relation};
use super::options::{GraphQLTypeOptions, GraphQLResolverOptions};

#[derive(Debug, Clone)]
pub struct Entity<'a> {
    pub name: String,
    pub table_name: String,
    pub skip: bool,
    pub fields: Vec<Field<'a>>,
    pub relations: Vec<Relation>,
    pub graphql: Option<GraphQLTypeOptions>,
    pub graphql_resolver: Option<GraphQLResolverOptions>,
    pub raw: &'a DescriptorProto,
    pub raw_file: &'a FileDescriptorProto,
}
