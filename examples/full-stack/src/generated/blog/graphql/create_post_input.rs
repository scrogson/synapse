//! GraphQL InputObject type for #msg_name
//! @generated
#![allow(missing_docs)]
#![allow(unused_imports)]
use async_graphql::InputObject;
/// GraphQL input object type
#[derive(InputObject)]
pub struct CreatePostInput {
    pub title: String,
    pub content: String,
    pub published: bool,
    pub author_id: i64,
}
