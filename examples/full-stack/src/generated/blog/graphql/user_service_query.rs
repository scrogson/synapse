//! GraphQL Query resolvers for #svc_name
//! @generated
#![allow(missing_docs)]
#![allow(unused_imports)]
use async_graphql::{Object, Context, Result};
use tonic::transport::Channel;
/// Query resolvers from #svc_name
pub struct UserServiceQuery;
#[Object]
impl UserServiceQuery {
    #[graphql(desc = "Get a user by ID")]
    async fn user(&self, ctx: &Context<'_>, input: GetUserRequest) -> Result<User> {
        let client = ctx.data_unchecked::<ServiceClient>();
        let response = client
            .clone()
            .get_user(input)
            .await
            .map_err(|e| async_graphql::Error::new(e.message()))?;
        Ok(response.into_inner().user)
    }
    #[graphql(desc = "List users with pagination")]
    async fn users(
        &self,
        ctx: &Context<'_>,
        input: ListUsersRequest,
    ) -> Result<UserConnection> {
        let client = ctx.data_unchecked::<ServiceClient>();
        let response = client
            .clone()
            .list_users(input)
            .await
            .map_err(|e| async_graphql::Error::new(e.message()))?;
        Ok(response.into_inner())
    }
}
type ServiceClient = super::proto::UserServiceClient<Channel>;
