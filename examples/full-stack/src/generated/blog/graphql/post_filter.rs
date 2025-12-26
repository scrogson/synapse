//! GraphQL InputObject type for #msg_name
//! @generated
#![allow(missing_docs)]
#![allow(unused_imports)]
use async_graphql::InputObject;
/// GraphQL input object type
#[derive(InputObject)]
pub struct PostFilter {
    pub id: Option<Int64Filter>,
    pub title: Option<StringFilter>,
    pub content: Option<StringFilter>,
    pub published: Option<BoolFilter>,
    pub author_id: Option<Int64Filter>,
}
