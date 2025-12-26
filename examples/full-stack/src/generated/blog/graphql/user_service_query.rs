//! GraphQL Query resolvers for #svc_name
//! @generated
#![allow(missing_docs)]
#![allow(unused_imports)]
use async_graphql::{Object, Context, Result};
use std::sync::Arc;
use super::super::SeaOrmUserServiceStorage;
use super::super::UserServiceStorage;
/// Query resolvers from #svc_name (uses storage layer)
#[derive(Default)]
pub struct UserServiceQuery;
#[Object]
impl UserServiceQuery {
    ///Get a user by ID
    async fn user(&self, ctx: &Context<'_>, id: i64) -> Result<Option<super::User>> {
        let storage = ctx.data_unchecked::<Arc<Storage>>();
        let request = super::super::GetUserRequest { id };
        match storage.get_user(request).await {
            Ok(response) => Ok(response.user.map(super::User::from)),
            Err(e) => {
                if e.to_string().contains("not found") {
                    Ok(None)
                } else {
                    Err(async_graphql::Error::new(e.to_string()))
                }
            }
        }
    }
    ///List users with pagination
    async fn users(
        &self,
        ctx: &Context<'_>,
        after: Option<String>,
        before: Option<String>,
        first: Option<i32>,
        last: Option<i32>,
        filter: Option<super::UserFilter>,
        order_by: Option<super::UserOrderBy>,
    ) -> Result<super::UserConnection> {
        let storage = ctx.data_unchecked::<Arc<Storage>>();
        let request = super::super::ListUsersRequest {
            after,
            before,
            first,
            last,
            filter: filter.map(|f| f.into()),
            order_by: order_by.map(|o| o.into()),
        };
        let response = storage
            .list_users(request)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        Ok(response.into())
    }
}
type Storage = SeaOrmUserServiceStorage;
