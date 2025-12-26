//! GraphQL Mutation resolvers for #svc_name
//! @generated
#![allow(missing_docs)]
#![allow(unused_imports)]
use async_graphql::{Object, Context, Result};
use std::sync::Arc;
use super::super::SeaOrmUserServiceStorage;
use super::super::UserServiceStorage;
/// Mutation resolvers from #svc_name (uses storage layer)
#[derive(Default)]
pub struct UserServiceMutation;
#[Object]
impl UserServiceMutation {
    ///Create a new user
    async fn createUser(
        &self,
        ctx: &Context<'_>,
        input: super::CreateUserInput,
    ) -> Result<super::User> {
        let storage = ctx.data_unchecked::<Arc<Storage>>();
        let request = super::super::CreateUserRequest {
            input: Some(input.into()),
        };
        let response = storage
            .create_user(request)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        Ok(
            response
                .user
                .map(super::User::from)
                .ok_or_else(|| async_graphql::Error::new("Failed to create"))?,
        )
    }
    ///Delete a user
    async fn deleteUser(&self, ctx: &Context<'_>, id: i64) -> Result<bool> {
        let storage = ctx.data_unchecked::<Arc<Storage>>();
        let request = super::super::DeleteUserRequest {
            id,
        };
        let response = storage
            .delete_user(request)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        Ok(response.success)
    }
}
type Storage = SeaOrmUserServiceStorage;
