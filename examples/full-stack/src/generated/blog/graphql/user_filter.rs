//! GraphQL InputObject type for #msg_name
//! @generated
#![allow(missing_docs)]
#![allow(unused_imports)]
use async_graphql::InputObject;
/// GraphQL input object type
#[derive(InputObject)]
pub struct UserFilter {
    pub id: Option<Int64Filter>,
    pub email: Option<StringFilter>,
    pub name: Option<StringFilter>,
    pub bio: Option<StringFilter>,
}
