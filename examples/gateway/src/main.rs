//! Synapse Gateway Example
//!
//! This example demonstrates a GraphQL gateway that federates multiple gRPC services:
//! - Blog service (Users, Posts)
//! - IAM service (Organizations, Teams, Users)
//!
//! The gateway connects to each gRPC service and exposes a unified GraphQL API.
//! DataLoaders are used for efficient batched data fetching across services.
//!
//! Architecture:
//! ```
//!                        ┌─────────────┐
//!                        │   Gateway   │
//!                        │  (GraphQL)  │
//!                        └──────┬──────┘
//!                               │
//!               ┌───────────────┼───────────────┐
//!               ▼               ▼               ▼
//!        ┌───────────┐   ┌───────────┐   ┌───────────┐
//!        │   Blog    │   │   IAM     │   │  Other    │
//!        │  (gRPC)   │   │  (gRPC)   │   │ Services  │
//!        └───────────┘   └───────────┘   └───────────┘
//! ```

use std::net::SocketAddr;

use async_graphql::{EmptySubscription, MergedObject, Schema};
use async_graphql::dataloader::DataLoader;
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::{extract::State, routing::get, Router};
use tonic::transport::Channel;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

// Import from the blog service
use synapse_full_stack_example::blog::{
    user_service_client::UserServiceClient as BlogUserClient,
    post_service_client::PostServiceClient,
    graphql::{
        PostServiceQuery as BlogPostServiceQuery,
        PostServiceMutation as BlogPostServiceMutation,
        User as BlogUser,
        UserConnection as BlogUserConnection,
        UserFilter as BlogUserFilter,
        UserOrderBy as BlogUserOrderBy,
        UserLoader as BlogUserLoader,
        PostLoader,
        PostsByUserLoader,
        CreateUserInput as BlogCreateUserInput,
        UpdateUserInput as BlogUpdateUserInput,
    },
};

// Import from the IAM service
use synapse_iam_example::iam::{
    user_service_client::UserServiceClient as IamUserClient,
    organization_service_client::OrganizationServiceClient,
    team_service_client::TeamServiceClient,
    graphql::{
        UserServiceQuery as IamUserServiceQuery,
        UserServiceMutation as IamUserServiceMutation,
        OrganizationServiceQuery as IamOrganizationServiceQuery,
        OrganizationServiceMutation as IamOrganizationServiceMutation,
        TeamServiceQuery as IamTeamServiceQuery,
        TeamServiceMutation as IamTeamServiceMutation,
        UserLoader as IamUserLoader,
        OrganizationLoader,
        TeamLoader,
        TeamsByOrganizationLoader,
        UsersByOrganizationLoader,
    },
};

// We need to wrap the service queries/mutations to avoid field name conflicts
// Blog and IAM both have "user" and "users" fields

/// Blog domain queries (authors and posts)
#[derive(Default)]
pub struct BlogQuery;

#[async_graphql::Object]
impl BlogQuery {
    /// Get author by ID
    async fn author(&self, ctx: &async_graphql::Context<'_>, id: i64) -> async_graphql::Result<Option<BlogUser>> {
        let client = ctx.data_unchecked::<BlogUserClient<Channel>>();
        let request = synapse_full_stack_example::blog::GetUserRequest { id };
        match client.clone().get_user(request).await {
            Ok(response) => Ok(response.into_inner().user.map(BlogUser::from)),
            Err(e) => {
                if e.code() == tonic::Code::NotFound {
                    Ok(None)
                } else {
                    Err(async_graphql::Error::new(e.message()))
                }
            }
        }
    }

    /// List authors with pagination
    async fn authors(
        &self,
        ctx: &async_graphql::Context<'_>,
        after: Option<String>,
        before: Option<String>,
        first: Option<i32>,
        last: Option<i32>,
        filter: Option<BlogUserFilter>,
        order_by: Option<BlogUserOrderBy>,
    ) -> async_graphql::Result<BlogUserConnection> {
        let client = ctx.data_unchecked::<BlogUserClient<Channel>>();
        let request = synapse_full_stack_example::blog::ListUsersRequest {
            after,
            before,
            first,
            last,
            filter: filter.map(|f| f.into()),
            order_by: order_by.map(|o| o.into()),
        };
        let response = client.clone().list_users(request).await
            .map_err(|e| async_graphql::Error::new(e.message()))?;
        Ok(response.into_inner().into())
    }
}

/// Blog domain mutations (authors)
#[derive(Default)]
pub struct BlogMutation;

#[async_graphql::Object]
impl BlogMutation {
    /// Create a new author
    async fn create_author(
        &self,
        ctx: &async_graphql::Context<'_>,
        input: BlogCreateUserInput,
    ) -> async_graphql::Result<BlogUser> {
        let client = ctx.data_unchecked::<BlogUserClient<Channel>>();
        let request: synapse_full_stack_example::blog::CreateUserRequest = input.into();
        let response = client
            .clone()
            .create_user(request)
            .await
            .map_err(|e| async_graphql::Error::new(e.message()))?;
        Ok(response
            .into_inner()
            .user
            .map(BlogUser::from)
            .ok_or_else(|| async_graphql::Error::new("Failed to create"))?)
    }

    /// Update an existing author
    async fn update_author(
        &self,
        ctx: &async_graphql::Context<'_>,
        id: i64,
        input: BlogUpdateUserInput,
    ) -> async_graphql::Result<BlogUser> {
        let client = ctx.data_unchecked::<BlogUserClient<Channel>>();
        let request = input.to_request(id);
        let response = client
            .clone()
            .update_user(request)
            .await
            .map_err(|e| async_graphql::Error::new(e.message()))?;
        Ok(response
            .into_inner()
            .user
            .map(BlogUser::from)
            .ok_or_else(|| async_graphql::Error::new("Failed to update"))?)
    }

    /// Delete an author
    async fn delete_author(
        &self,
        ctx: &async_graphql::Context<'_>,
        id: i64,
    ) -> async_graphql::Result<bool> {
        let client = ctx.data_unchecked::<BlogUserClient<Channel>>();
        let request = synapse_full_stack_example::blog::DeleteUserRequest { id };
        let response = client
            .clone()
            .delete_user(request)
            .await
            .map_err(|e| async_graphql::Error::new(e.message()))?;
        Ok(response.into_inner().success)
    }
}

/// Combined Query merging all service queries
///
/// Note: We rename blog users to "authors" to avoid conflicts with IAM users.
/// - Blog: author, authors, post, posts
/// - IAM: user, users, organization, organizations, team, teams
#[derive(MergedObject, Default)]
pub struct Query(
    // Blog queries - using custom wrapper with prefixed names
    BlogQuery,
    BlogPostServiceQuery,
    // IAM queries - these get the unprefixed names (user, users, etc.)
    IamUserServiceQuery,
    IamOrganizationServiceQuery,
    IamTeamServiceQuery,
);

/// Combined Mutation merging all service mutations
///
/// Note: We rename blog user mutations to "author" to avoid conflicts with IAM users.
/// - Blog: createAuthor, updateAuthor, deleteAuthor, createPost, updatePost, deletePost
/// - IAM: createUser, updateUser, deleteUser, createOrganization, etc.
#[derive(MergedObject, Default)]
pub struct Mutation(
    // Blog mutations - using custom wrapper with prefixed names for user mutations
    BlogMutation,
    BlogPostServiceMutation,
    // IAM mutations - these get the unprefixed names (createUser, etc.)
    IamUserServiceMutation,
    IamOrganizationServiceMutation,
    IamTeamServiceMutation,
);

/// The combined GraphQL schema
pub type AppSchema = Schema<Query, Mutation, EmptySubscription>;

/// Build the combined GraphQL schema with all gRPC clients
pub fn build_schema(
    // Blog clients
    blog_user_client: BlogUserClient<Channel>,
    post_client: PostServiceClient<Channel>,
    // IAM clients
    iam_user_client: IamUserClient<Channel>,
    org_client: OrganizationServiceClient<Channel>,
    team_client: TeamServiceClient<Channel>,
) -> AppSchema {
    Schema::build(Query::default(), Mutation::default(), EmptySubscription)
        // Blog service clients and loaders
        .data(blog_user_client.clone())
        .data(post_client.clone())
        .data(DataLoader::new(
            BlogUserLoader::new(blog_user_client.clone()),
            tokio::spawn,
        ))
        .data(DataLoader::new(
            PostLoader::new(post_client.clone()),
            tokio::spawn,
        ))
        .data(DataLoader::new(
            PostsByUserLoader::new(post_client),
            tokio::spawn,
        ))
        // IAM service clients and loaders
        .data(iam_user_client.clone())
        .data(org_client.clone())
        .data(team_client.clone())
        .data(DataLoader::new(
            IamUserLoader::new(iam_user_client.clone()),
            tokio::spawn,
        ))
        .data(DataLoader::new(
            OrganizationLoader::new(org_client.clone()),
            tokio::spawn,
        ))
        .data(DataLoader::new(
            TeamLoader::new(team_client.clone()),
            tokio::spawn,
        ))
        .data(DataLoader::new(
            TeamsByOrganizationLoader::new(team_client),
            tokio::spawn,
        ))
        .data(DataLoader::new(
            UsersByOrganizationLoader::new(iam_user_client.clone()),
            tokio::spawn,
        ))
        .finish()
}

/// GraphQL handler
async fn graphql_handler(
    State(schema): State<AppSchema>,
    req: GraphQLRequest,
) -> GraphQLResponse {
    schema.execute(req.into_inner()).await.into()
}

/// Apollo Sandbox handler
async fn apollo_sandbox() -> impl axum::response::IntoResponse {
    axum::response::Html(r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>Synapse Gateway - Apollo Sandbox</title>
    <style>body { margin: 0; overflow: hidden; }</style>
</head>
<body>
    <div id="sandbox" style="width: 100vw; height: 100vh;"></div>
    <script src="https://embeddable-sandbox.cdn.apollographql.com/_latest/embeddable-sandbox.umd.production.min.js"></script>
    <script>
        new window.EmbeddedSandbox({
            target: '#sandbox',
            initialEndpoint: window.location.origin + '/graphql',
        });
    </script>
</body>
</html>"#)
}

/// Export SDL schema for debugging
fn export_sdl() {
    // Create dummy clients just to build the schema for SDL export
    // In reality, we'd need connected clients, but for SDL export we can
    // create the schema without actual connections
    println!("Note: Full SDL export requires connected gRPC services.");
    println!("The schema merges Blog and IAM services together.");
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Check if running in SDL export mode
    if std::env::args().any(|a| a == "--export-sdl") {
        export_sdl();
        return Ok(());
    }
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load environment variables
    dotenvy::dotenv().ok();

    // gRPC service endpoints
    let blog_endpoint = std::env::var("BLOG_GRPC_ENDPOINT")
        .unwrap_or_else(|_| "http://127.0.0.1:50060".into());
    let iam_endpoint = std::env::var("IAM_GRPC_ENDPOINT")
        .unwrap_or_else(|_| "http://127.0.0.1:50051".into());

    tracing::info!("Connecting to Blog gRPC service at {}", blog_endpoint);
    tracing::info!("Connecting to IAM gRPC service at {}", iam_endpoint);

    // Connect to Blog gRPC services
    let blog_channel = Channel::from_shared(blog_endpoint)?
        .connect()
        .await?;
    let blog_user_client = BlogUserClient::new(blog_channel.clone());
    let post_client = PostServiceClient::new(blog_channel);

    // Connect to IAM gRPC services
    let iam_channel = Channel::from_shared(iam_endpoint)?
        .connect()
        .await?;
    let iam_user_client = IamUserClient::new(iam_channel.clone());
    let org_client = OrganizationServiceClient::new(iam_channel.clone());
    let team_client = TeamServiceClient::new(iam_channel);

    // Build the combined GraphQL schema
    let schema = build_schema(
        blog_user_client,
        post_client,
        iam_user_client,
        org_client,
        team_client,
    );

    // GraphQL server address
    let addr: SocketAddr = std::env::var("GATEWAY_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:4000".into())
        .parse()?;

    tracing::info!("Gateway GraphQL server listening on {}", addr);
    tracing::info!("Apollo Sandbox available at http://{}/", addr);

    // Build axum router
    let app = Router::new()
        .route("/graphql", get(apollo_sandbox).post(graphql_handler))
        .route("/", get(apollo_sandbox))
        .with_state(schema);

    // Start server
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
