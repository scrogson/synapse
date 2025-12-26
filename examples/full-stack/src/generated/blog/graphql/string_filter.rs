//! GraphQL InputObject type for #msg_name
//! @generated
#![allow(missing_docs)]
#![allow(unused_imports)]
use async_graphql::InputObject;
/// GraphQL input object type
#[derive(InputObject, Default)]
pub struct StringFilter {
    pub eq: Option<String>,
    pub ne: Option<String>,
    pub contains: Option<String>,
    pub starts_with: Option<String>,
    pub ends_with: Option<String>,
}
impl From<StringFilter> for super::super::StringFilter {
    fn from(input: StringFilter) -> Self {
        Self {
            eq: input.eq,
            ne: input.ne,
            contains: input.contains,
            starts_with: input.starts_with,
            ends_with: input.ends_with,
        }
    }
}
