use prost_types::{DescriptorProto, FileDescriptorProto};
use super::Field;
use super::options::*;

#[derive(Debug, Clone)]
pub struct Message<'a> {
    pub name: String,
    pub fields: Vec<Field<'a>>,
    pub validate: Option<ValidateMessageOptions>,
    pub graphql: Option<GraphQLTypeOptions>,
    pub graphql_resolver: Option<GraphQLResolverOptions>,
    pub grpc_response: Option<GrpcResponseOptions>,
    pub raw: &'a DescriptorProto,
    pub raw_file: &'a FileDescriptorProto,
}
