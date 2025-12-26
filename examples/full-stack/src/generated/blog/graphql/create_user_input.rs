//! GraphQL InputObject type for #msg_name
//! @generated
#![allow(missing_docs)]
#![allow(unused_imports)]
use async_graphql::InputObject;
#[allow(unused_imports)]
use super::{Int64Filter, StringFilter, BoolFilter, OrderDirection};
/// GraphQL input object type
#[derive(InputObject, Default)]
pub struct CreateUserInput {
    pub email: String,
    pub name: String,
    pub bio: Option<String>,
}
impl From<CreateUserInput> for super::super::CreateUserInput {
    fn from(input: CreateUserInput) -> Self {
        Self {
            email: input.email,
            name: input.name,
            bio: input.bio,
        }
    }
}
