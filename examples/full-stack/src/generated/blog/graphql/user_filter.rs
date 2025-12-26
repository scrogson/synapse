//! GraphQL InputObject type for #msg_name
//! @generated
#![allow(missing_docs)]
#![allow(unused_imports)]
use async_graphql::InputObject;
#[allow(unused_imports)]
use super::{Int64Filter, StringFilter, BoolFilter, OrderDirection};
/// GraphQL input object type
#[derive(InputObject, Default)]
pub struct UserFilter {
    pub id: Option<Int64Filter>,
    pub email: Option<StringFilter>,
    pub name: Option<StringFilter>,
    pub bio: Option<StringFilter>,
}
impl From<UserFilter> for super::super::UserFilter {
    fn from(input: UserFilter) -> Self {
        Self {
            id: input.id.map(Into::into),
            email: input.email.map(Into::into),
            name: input.name.map(Into::into),
            bio: input.bio.map(Into::into),
        }
    }
}
