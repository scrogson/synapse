//! GraphQL Mutation resolvers for #svc_name
//! @generated
#![allow(missing_docs)]
#![allow(unused_imports)]
use async_graphql::{Object, Context, Result};
use tonic::transport::Channel;
/// Mutation resolvers from #svc_name
pub struct PostServiceMutation;
#[Object]
impl PostServiceMutation {
    #[graphql(desc = "Create a new post")]
    async fn createPost(
        &self,
        ctx: &Context<'_>,
        input: CreatePostInput,
    ) -> Result<Post> {
        let client = ctx.data_unchecked::<ServiceClient>();
        let request = CreatePostRequest { input };
        let response = client
            .clone()
            .create_post(request)
            .await
            .map_err(|e| async_graphql::Error::new(e.message()))?;
        Ok(response.into_inner().post)
    }
    #[graphql(desc = "Update an existing post")]
    async fn updatePost(
        &self,
        ctx: &Context<'_>,
        input: UpdatePostRequest,
    ) -> Result<Post> {
        let client = ctx.data_unchecked::<ServiceClient>();
        let response = client
            .clone()
            .update_post(input)
            .await
            .map_err(|e| async_graphql::Error::new(e.message()))?;
        Ok(response.into_inner().post)
    }
    #[graphql(desc = "Delete a post")]
    async fn deletePost(
        &self,
        ctx: &Context<'_>,
        input: DeletePostRequest,
    ) -> Result<DeletePostResponse> {
        let client = ctx.data_unchecked::<ServiceClient>();
        let response = client
            .clone()
            .delete_post(input)
            .await
            .map_err(|e| async_graphql::Error::new(e.message()))?;
        Ok(response.into_inner())
    }
}
type ServiceClient = super::proto::PostServiceClient<Channel>;
