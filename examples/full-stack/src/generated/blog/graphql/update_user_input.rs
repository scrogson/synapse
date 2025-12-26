//! GraphQL InputObject type for #msg_name
//! @generated
#![allow(missing_docs)]
#![allow(unused_imports)]
use async_graphql::InputObject;
#[allow(unused_imports)]
use super::{Int64Filter, StringFilter, BoolFilter, OrderDirection};
/// GraphQL input object type
#[derive(InputObject, Default)]
pub struct UpdateUserInput {
    pub email: Option<String>,
    pub name: Option<String>,
    pub bio: Option<String>,
}
impl From<UpdateUserInput> for super::super::UpdateUserInput {
    fn from(input: UpdateUserInput) -> Self {
        Self {
            email: input.email,
            name: input.name,
            bio: input.bio,
        }
    }
}
