//! GraphQL InputObject type for #msg_name
//! @generated
#![allow(missing_docs)]
#![allow(unused_imports)]
use async_graphql::InputObject;
/// GraphQL input object type
#[derive(InputObject)]
pub struct UpdateUserInput {
    pub email: Option<String>,
    pub name: Option<String>,
    pub bio: Option<String>,
}
