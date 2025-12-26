//! GraphQL InputObject type for #msg_name
//! @generated
#![allow(missing_docs)]
#![allow(unused_imports)]
use async_graphql::InputObject;
/// GraphQL input object type
#[derive(InputObject)]
pub struct StringFilter {
    pub eq: Option<String>,
    pub ne: Option<String>,
    pub contains: Option<String>,
    pub starts_with: Option<String>,
    pub ends_with: Option<String>,
}
