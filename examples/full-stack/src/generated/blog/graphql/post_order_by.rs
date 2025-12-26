//! GraphQL InputObject type for #msg_name
//! @generated
#![allow(missing_docs)]
#![allow(unused_imports)]
use async_graphql::InputObject;
#[allow(unused_imports)]
use super::{Int64Filter, StringFilter, BoolFilter, OrderDirection};
/// GraphQL input object type
#[derive(InputObject, Default)]
pub struct PostOrderBy {
    pub id: Option<OrderDirection>,
    pub title: Option<OrderDirection>,
    pub published: Option<OrderDirection>,
    pub created_at: Option<OrderDirection>,
    pub updated_at: Option<OrderDirection>,
}
impl From<PostOrderBy> for super::super::PostOrderBy {
    fn from(input: PostOrderBy) -> Self {
        Self {
            id: input.id.map(|e| super::super::OrderDirection::from(e) as i32),
            title: input.title.map(|e| super::super::OrderDirection::from(e) as i32),
            published: input
                .published
                .map(|e| super::super::OrderDirection::from(e) as i32),
            created_at: input
                .created_at
                .map(|e| super::super::OrderDirection::from(e) as i32),
            updated_at: input
                .updated_at
                .map(|e| super::super::OrderDirection::from(e) as i32),
        }
    }
}
