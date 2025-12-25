//! GraphQL InputObject type for #msg_name
//! @generated
#![allow(missing_docs)]
#![allow(unused_imports)]
use async_graphql::InputObject;
/// GraphQL input object type
#[derive(InputObject)]
pub struct CreateUserInput {
    pub email: String,
    pub name: String,
    pub bio: Option<String>,
}
