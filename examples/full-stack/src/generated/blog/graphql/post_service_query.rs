//! GraphQL Query resolvers for #svc_name
//! @generated
#![allow(missing_docs)]
#![allow(unused_imports)]
use async_graphql::{Object, Context, Result};
use std::sync::Arc;
use super::super::SeaOrmPostServiceStorage;
use super::super::PostServiceStorage;
/// Query resolvers from #svc_name (uses storage layer)
#[derive(Default)]
pub struct PostServiceQuery;
#[Object]
impl PostServiceQuery {
    ///Get a post by ID
    async fn post(&self, ctx: &Context<'_>, id: i64) -> Result<Option<super::Post>> {
        let storage = ctx.data_unchecked::<Arc<Storage>>();
        let request = super::super::GetPostRequest { id };
        match storage.get_post(request).await {
            Ok(response) => Ok(response.post.map(super::Post::from)),
            Err(e) => {
                if e.to_string().contains("not found") {
                    Ok(None)
                } else {
                    Err(async_graphql::Error::new(e.to_string()))
                }
            }
        }
    }
    ///List posts with pagination
    async fn posts(
        &self,
        ctx: &Context<'_>,
        after: Option<String>,
        before: Option<String>,
        first: Option<i32>,
        last: Option<i32>,
        filter: Option<super::PostFilter>,
        order_by: Option<super::PostOrderBy>,
    ) -> Result<super::PostConnection> {
        let storage = ctx.data_unchecked::<Arc<Storage>>();
        let request = super::super::ListPostsRequest {
            after,
            before,
            first,
            last,
            filter: filter.map(|f| f.into()),
            order_by: order_by.map(|o| o.into()),
        };
        let response = storage
            .list_posts(request)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        Ok(response.into())
    }
}
type Storage = SeaOrmPostServiceStorage;
