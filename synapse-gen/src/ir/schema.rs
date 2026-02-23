use prost_types::FileDescriptorProto;
use super::{Entity, Service, Enum, Message};

#[derive(Debug, Clone)]
pub struct Schema<'a> {
    pub packages: Vec<Package<'a>>,
}

#[derive(Debug, Clone)]
pub struct Package<'a> {
    pub name: String,
    pub entities: Vec<Entity<'a>>,
    pub services: Vec<Service<'a>>,
    pub enums: Vec<Enum<'a>>,
    pub messages: Vec<Message<'a>>,
    pub raw_files: Vec<&'a FileDescriptorProto>,
}
