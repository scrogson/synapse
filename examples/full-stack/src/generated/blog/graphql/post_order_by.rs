//! GraphQL InputObject type for #msg_name
//! @generated
#![allow(missing_docs)]
#![allow(unused_imports)]
use async_graphql::InputObject;
/// GraphQL input object type
#[derive(InputObject)]
pub struct PostOrderBy {
    pub id: Option<OrderDirection>,
    pub title: Option<OrderDirection>,
    pub published: Option<OrderDirection>,
    pub created_at: Option<OrderDirection>,
    pub updated_at: Option<OrderDirection>,
}
