//! GraphQL Query resolvers for #svc_name
//! @generated
#![allow(missing_docs)]
#![allow(unused_imports)]
use async_graphql::{Object, Context, Result};
use tonic::transport::Channel;
/// Query resolvers from #svc_name
pub struct PostServiceQuery;
#[Object]
impl PostServiceQuery {
    #[graphql(desc = "Get a post by ID")]
    async fn post(&self, ctx: &Context<'_>, input: GetPostRequest) -> Result<Post> {
        let client = ctx.data_unchecked::<ServiceClient>();
        let response = client
            .clone()
            .get_post(input)
            .await
            .map_err(|e| async_graphql::Error::new(e.message()))?;
        Ok(response.into_inner().post)
    }
    #[graphql(desc = "List posts with pagination")]
    async fn posts(
        &self,
        ctx: &Context<'_>,
        input: ListPostsRequest,
    ) -> Result<PostConnection> {
        let client = ctx.data_unchecked::<ServiceClient>();
        let response = client
            .clone()
            .list_posts(input)
            .await
            .map_err(|e| async_graphql::Error::new(e.message()))?;
        Ok(response.into_inner())
    }
}
type ServiceClient = super::proto::PostServiceClient<Channel>;
