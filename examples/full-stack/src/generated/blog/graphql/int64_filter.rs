//! GraphQL InputObject type for #msg_name
//! @generated
#![allow(missing_docs)]
#![allow(unused_imports)]
use async_graphql::InputObject;
/// GraphQL input object type
#[derive(InputObject, Default)]
pub struct Int64Filter {
    pub eq: Option<i64>,
    pub ne: Option<i64>,
    pub gt: Option<i64>,
    pub gte: Option<i64>,
    pub lt: Option<i64>,
    pub lte: Option<i64>,
    pub r#in: Vec<i64>,
}
impl From<Int64Filter> for super::super::Int64Filter {
    fn from(input: Int64Filter) -> Self {
        Self {
            eq: input.eq,
            ne: input.ne,
            gt: input.gt,
            gte: input.gte,
            lt: input.lt,
            lte: input.lte,
            r#in: input.r#in,
        }
    }
}
