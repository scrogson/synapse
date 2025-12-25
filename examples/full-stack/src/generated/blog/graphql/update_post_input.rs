//! GraphQL InputObject type for #msg_name
//! @generated
#![allow(missing_docs)]
#![allow(unused_imports)]
use async_graphql::InputObject;
/// GraphQL input object type
#[derive(InputObject)]
pub struct UpdatePostInput {
    pub title: Option<String>,
    pub content: Option<String>,
    pub published: Option<bool>,
}
