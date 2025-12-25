//! GraphQL Mutation resolvers for #svc_name
//! @generated
#![allow(missing_docs)]
#![allow(unused_imports)]
use async_graphql::{Object, Context, Result};
use tonic::transport::Channel;
/// Mutation resolvers from #svc_name
pub struct UserServiceMutation;
#[Object]
impl UserServiceMutation {
    #[graphql(desc = "Create a new user")]
    async fn createUser(
        &self,
        ctx: &Context<'_>,
        input: CreateUserInput,
    ) -> Result<User> {
        let client = ctx.data_unchecked::<ServiceClient>();
        let request = CreateUserRequest { input };
        let response = client
            .clone()
            .create_user(request)
            .await
            .map_err(|e| async_graphql::Error::new(e.message()))?;
        Ok(response.into_inner().user)
    }
    #[graphql(desc = "Update an existing user")]
    async fn updateUser(
        &self,
        ctx: &Context<'_>,
        input: UpdateUserRequest,
    ) -> Result<User> {
        let client = ctx.data_unchecked::<ServiceClient>();
        let response = client
            .clone()
            .update_user(input)
            .await
            .map_err(|e| async_graphql::Error::new(e.message()))?;
        Ok(response.into_inner().user)
    }
    #[graphql(desc = "Delete a user")]
    async fn deleteUser(
        &self,
        ctx: &Context<'_>,
        input: DeleteUserRequest,
    ) -> Result<DeleteUserResponse> {
        let client = ctx.data_unchecked::<ServiceClient>();
        let response = client
            .clone()
            .delete_user(input)
            .await
            .map_err(|e| async_graphql::Error::new(e.message()))?;
        Ok(response.into_inner())
    }
}
type ServiceClient = super::proto::UserServiceClient<Channel>;
