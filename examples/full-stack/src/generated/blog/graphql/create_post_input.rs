//! GraphQL InputObject type for #msg_name
//! @generated
#![allow(missing_docs)]
#![allow(unused_imports)]
use async_graphql::InputObject;
#[allow(unused_imports)]
use super::{Int64Filter, StringFilter, BoolFilter, OrderDirection};
/// GraphQL input object type
#[derive(InputObject, Default)]
pub struct CreatePostInput {
    pub title: String,
    pub content: String,
    pub published: bool,
    pub author_id: i64,
}
impl From<CreatePostInput> for super::super::CreatePostInput {
    fn from(input: CreatePostInput) -> Self {
        Self {
            title: input.title,
            content: input.content,
            published: input.published,
            author_id: input.author_id,
        }
    }
}
