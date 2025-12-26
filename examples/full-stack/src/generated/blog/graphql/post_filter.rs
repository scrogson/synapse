//! GraphQL InputObject type for #msg_name
//! @generated
#![allow(missing_docs)]
#![allow(unused_imports)]
use async_graphql::InputObject;
#[allow(unused_imports)]
use super::{Int64Filter, StringFilter, BoolFilter, OrderDirection};
/// GraphQL input object type
#[derive(InputObject, Default)]
pub struct PostFilter {
    pub id: Option<Int64Filter>,
    pub title: Option<StringFilter>,
    pub content: Option<StringFilter>,
    pub published: Option<BoolFilter>,
    pub author_id: Option<Int64Filter>,
}
impl From<PostFilter> for super::super::PostFilter {
    fn from(input: PostFilter) -> Self {
        Self {
            id: input.id.map(Into::into),
            title: input.title.map(Into::into),
            content: input.content.map(Into::into),
            published: input.published.map(Into::into),
            author_id: input.author_id.map(Into::into),
        }
    }
}
