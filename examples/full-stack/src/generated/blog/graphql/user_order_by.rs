//! GraphQL InputObject type for #msg_name
//! @generated
#![allow(missing_docs)]
#![allow(unused_imports)]
use async_graphql::InputObject;
#[allow(unused_imports)]
use super::{Int64Filter, StringFilter, BoolFilter, OrderDirection};
/// GraphQL input object type
#[derive(InputObject, Default)]
pub struct UserOrderBy {
    pub id: Option<OrderDirection>,
    pub email: Option<OrderDirection>,
    pub name: Option<OrderDirection>,
    pub created_at: Option<OrderDirection>,
    pub updated_at: Option<OrderDirection>,
}
impl From<UserOrderBy> for super::super::UserOrderBy {
    fn from(input: UserOrderBy) -> Self {
        Self {
            id: input.id.map(|e| super::super::OrderDirection::from(e) as i32),
            email: input.email.map(|e| super::super::OrderDirection::from(e) as i32),
            name: input.name.map(|e| super::super::OrderDirection::from(e) as i32),
            created_at: input
                .created_at
                .map(|e| super::super::OrderDirection::from(e) as i32),
            updated_at: input
                .updated_at
                .map(|e| super::super::OrderDirection::from(e) as i32),
        }
    }
}
