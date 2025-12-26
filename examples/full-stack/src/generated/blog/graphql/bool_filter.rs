//! GraphQL InputObject type for #msg_name
//! @generated
#![allow(missing_docs)]
#![allow(unused_imports)]
use async_graphql::InputObject;
/// GraphQL input object type
#[derive(InputObject, Default)]
pub struct BoolFilter {
    pub eq: Option<bool>,
}
impl From<BoolFilter> for super::super::BoolFilter {
    fn from(input: BoolFilter) -> Self {
        Self { eq: input.eq }
    }
}
