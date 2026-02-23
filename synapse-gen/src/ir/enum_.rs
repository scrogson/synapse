use prost_types::{EnumDescriptorProto, FileDescriptorProto};
use super::options::EnumStorageOptions;

#[derive(Debug, Clone)]
pub struct Enum<'a> {
    pub name: String,
    pub variants: Vec<EnumVariant>,
    pub storage: Option<EnumStorageOptions>,
    pub raw: &'a EnumDescriptorProto,
    pub raw_file: &'a FileDescriptorProto,
}

#[derive(Debug, Clone)]
pub struct EnumVariant {
    pub name: String,
    pub number: i32,
    pub string_value: String,
    pub int_value: i32,
    pub is_default: bool,
    pub skip: bool,
}
