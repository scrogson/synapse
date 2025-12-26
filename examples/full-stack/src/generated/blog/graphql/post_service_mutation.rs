//! GraphQL Mutation resolvers for #svc_name
//! @generated
#![allow(missing_docs)]
#![allow(unused_imports)]
use async_graphql::{Object, Context, Result};
use std::sync::Arc;
use super::super::SeaOrmPostServiceStorage;
use super::super::PostServiceStorage;
/// Mutation resolvers from #svc_name (uses storage layer)
#[derive(Default)]
pub struct PostServiceMutation;
#[Object]
impl PostServiceMutation {
    ///Create a new post
    async fn createPost(
        &self,
        ctx: &Context<'_>,
        input: super::CreatePostInput,
    ) -> Result<super::Post> {
        let storage = ctx.data_unchecked::<Arc<Storage>>();
        let request = super::super::CreatePostRequest {
            input: Some(input.into()),
        };
        let response = storage
            .create_post(request)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        Ok(
            response
                .post
                .map(super::Post::from)
                .ok_or_else(|| async_graphql::Error::new("Failed to create"))?,
        )
    }
    ///Delete a post
    async fn deletePost(&self, ctx: &Context<'_>, id: i64) -> Result<bool> {
        let storage = ctx.data_unchecked::<Arc<Storage>>();
        let request = super::super::DeletePostRequest {
            id,
        };
        let response = storage
            .delete_post(request)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        Ok(response.success)
    }
}
type Storage = SeaOrmPostServiceStorage;
