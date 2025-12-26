//! GraphQL InputObject type for #msg_name
//! @generated
#![allow(missing_docs)]
#![allow(unused_imports)]
use async_graphql::InputObject;
#[allow(unused_imports)]
use super::{Int64Filter, StringFilter, BoolFilter, OrderDirection};
/// GraphQL input object type
#[derive(InputObject, Default)]
pub struct UpdatePostInput {
    pub title: Option<String>,
    pub content: Option<String>,
    pub published: Option<bool>,
}
impl From<UpdatePostInput> for super::super::UpdatePostInput {
    fn from(input: UpdatePostInput) -> Self {
        Self {
            title: input.title,
            content: input.content,
            published: input.published,
        }
    }
}
