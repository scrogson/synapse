use prost_types::{FileDescriptorProto, MethodDescriptorProto, ServiceDescriptorProto};
use super::options::*;

#[derive(Debug, Clone)]
pub struct Service<'a> {
    pub name: String,
    pub methods: Vec<Method<'a>>,
    pub storage: Option<StorageServiceOptions>,
    pub graphql: Option<GraphQLServiceOptions>,
    pub grpc: Option<GrpcServiceOptions>,
    pub raw: &'a ServiceDescriptorProto,
    pub raw_file: &'a FileDescriptorProto,
}

#[derive(Debug, Clone)]
pub struct Method<'a> {
    pub name: String,
    pub input_type: String,
    pub output_type: String,
    pub client_streaming: bool,
    pub server_streaming: bool,
    pub storage: Option<StorageMethodOptions>,
    pub graphql: Option<GraphQLMethodOptions>,
    pub graphql_resolver: Option<GraphQLMethodResolverOptions>,
    pub grpc: Option<GrpcMethodOptions>,
    pub raw: &'a MethodDescriptorProto,
}
