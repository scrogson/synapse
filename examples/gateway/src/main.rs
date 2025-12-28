//! Synapse Gateway Example
//!
//! This example demonstrates a GraphQL gateway that federates multiple gRPC services:
//! - Blog service (Authors, Posts)
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
    author_service_client::AuthorServiceClient,
    post_service_client::PostServiceClient,
    graphql::{
        AuthorServiceQuery,
        AuthorServiceMutation,
        PostServiceQuery,
        PostServiceMutation,
        AuthorLoader,
        PostLoader,
        PostsByAuthorLoader,
    },
};

// Import from the IAM service
use synapse_iam_example::iam::{
    user_service_client::UserServiceClient,
    organization_service_client::OrganizationServiceClient,
    team_service_client::TeamServiceClient,
    graphql::{
        UserServiceQuery,
        UserServiceMutation,
        OrganizationServiceQuery,
        OrganizationServiceMutation,
        TeamServiceQuery,
        TeamServiceMutation,
        UserLoader,
        OrganizationLoader,
        TeamLoader,
        TeamsByOrganizationLoader,
        UsersByOrganizationLoader,
    },
};

/// Combined Query merging all service queries
///
/// - Blog: author, authors, post, posts
/// - IAM: user, users, organization, organizations, team, teams
#[derive(MergedObject, Default)]
pub struct Query(
    // Blog queries
    AuthorServiceQuery,
    PostServiceQuery,
    // IAM queries
    UserServiceQuery,
    OrganizationServiceQuery,
    TeamServiceQuery,
);

/// Combined Mutation merging all service mutations
///
/// - Blog: createAuthor, updateAuthor, deleteAuthor, createPost, updatePost, deletePost
/// - IAM: createUser, updateUser, deleteUser, createOrganization, etc.
#[derive(MergedObject, Default)]
pub struct Mutation(
    // Blog mutations
    AuthorServiceMutation,
    PostServiceMutation,
    // IAM mutations
    UserServiceMutation,
    OrganizationServiceMutation,
    TeamServiceMutation,
);

/// The combined GraphQL schema
pub type AppSchema = Schema<Query, Mutation, EmptySubscription>;

/// Build the combined GraphQL schema with all gRPC clients
pub fn build_schema(
    // Blog clients
    author_client: AuthorServiceClient<Channel>,
    post_client: PostServiceClient<Channel>,
    // IAM clients
    user_client: UserServiceClient<Channel>,
    org_client: OrganizationServiceClient<Channel>,
    team_client: TeamServiceClient<Channel>,
) -> AppSchema {
    Schema::build(Query::default(), Mutation::default(), EmptySubscription)
        // Blog service clients and loaders
        .data(author_client.clone())
        .data(post_client.clone())
        .data(DataLoader::new(
            AuthorLoader::new(author_client.clone()),
            tokio::spawn,
        ))
        .data(DataLoader::new(
            PostLoader::new(post_client.clone()),
            tokio::spawn,
        ))
        .data(DataLoader::new(
            PostsByAuthorLoader::new(post_client),
            tokio::spawn,
        ))
        // IAM service clients and loaders
        .data(user_client.clone())
        .data(org_client.clone())
        .data(team_client.clone())
        .data(DataLoader::new(
            UserLoader::new(user_client.clone()),
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
            UsersByOrganizationLoader::new(user_client.clone()),
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
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
        .unwrap_or_else(|_| "http://127.0.0.1:50061".into());

    tracing::info!("Connecting to Blog gRPC service at {}", blog_endpoint);
    tracing::info!("Connecting to IAM gRPC service at {}", iam_endpoint);

    // Connect to Blog gRPC services (lazy - connects on first request)
    let blog_channel = Channel::from_shared(blog_endpoint)?
        .connect_lazy();
    let author_client = AuthorServiceClient::new(blog_channel.clone());
    let post_client = PostServiceClient::new(blog_channel);

    // Connect to IAM gRPC services (lazy - connects on first request)
    let iam_channel = Channel::from_shared(iam_endpoint)?
        .connect_lazy();
    let user_client = UserServiceClient::new(iam_channel.clone());
    let org_client = OrganizationServiceClient::new(iam_channel.clone());
    let team_client = TeamServiceClient::new(iam_channel);

    // Build the combined GraphQL schema
    let schema = build_schema(
        author_client,
        post_client,
        user_client,
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
