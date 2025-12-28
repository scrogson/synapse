//! GraphQL Gateway binary
//!
//! Connects to IAM and Blog gRPC services and exposes a unified GraphQL API.
//!
//! Run with: cargo run --bin gateway --features gateway --no-default-features

use std::net::SocketAddr;

use async_graphql::{EmptySubscription, MergedObject, Schema};
use async_graphql::dataloader::DataLoader;
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::{extract::State, routing::get, Router};
use tonic::transport::Channel;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use synapse_unified_example::{
    iam::{
        user_service_client::UserServiceClient,
        organization_service_client::OrganizationServiceClient,
        team_service_client::TeamServiceClient,
        graphql::{
            UserServiceQuery, UserServiceMutation,
            OrganizationServiceQuery, OrganizationServiceMutation,
            TeamServiceQuery, TeamServiceMutation,
            UserLoader, OrganizationLoader, TeamLoader,
            TeamsByOrganizationLoader, UsersByOrganizationLoader,
        },
    },
    blog::{
        author_service_client::AuthorServiceClient,
        post_service_client::PostServiceClient,
        graphql::{
            AuthorServiceQuery, AuthorServiceMutation,
            PostServiceQuery, PostServiceMutation,
            AuthorLoader, PostLoader, PostsByAuthorLoader,
        },
    },
};

#[derive(MergedObject, Default)]
pub struct Query(
    UserServiceQuery,
    OrganizationServiceQuery,
    TeamServiceQuery,
    AuthorServiceQuery,
    PostServiceQuery,
);

#[derive(MergedObject, Default)]
pub struct Mutation(
    UserServiceMutation,
    OrganizationServiceMutation,
    TeamServiceMutation,
    AuthorServiceMutation,
    PostServiceMutation,
);

pub type AppSchema = Schema<Query, Mutation, EmptySubscription>;

async fn graphql_handler(
    State(schema): State<AppSchema>,
    req: GraphQLRequest,
) -> GraphQLResponse {
    schema.execute(req.into_inner()).await.into()
}

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
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    dotenvy::dotenv().ok();

    // gRPC service endpoints
    let iam_endpoint = std::env::var("IAM_GRPC_ENDPOINT")
        .unwrap_or_else(|_| "http://127.0.0.1:50052".into());
    let blog_endpoint = std::env::var("BLOG_GRPC_ENDPOINT")
        .unwrap_or_else(|_| "http://127.0.0.1:50053".into());

    tracing::info!("Connecting to IAM gRPC at {}", iam_endpoint);
    tracing::info!("Connecting to Blog gRPC at {}", blog_endpoint);

    // Connect lazily (won't fail if services aren't up yet)
    let iam_channel = Channel::from_shared(iam_endpoint)?.connect_lazy();
    let blog_channel = Channel::from_shared(blog_endpoint)?.connect_lazy();

    let user_client = UserServiceClient::new(iam_channel.clone());
    let org_client = OrganizationServiceClient::new(iam_channel.clone());
    let team_client = TeamServiceClient::new(iam_channel);
    let author_client = AuthorServiceClient::new(blog_channel.clone());
    let post_client = PostServiceClient::new(blog_channel);

    // Build GraphQL schema
    let schema = Schema::build(Query::default(), Mutation::default(), EmptySubscription)
        .data(user_client.clone())
        .data(org_client.clone())
        .data(team_client.clone())
        .data(DataLoader::new(UserLoader::new(user_client.clone()), tokio::spawn))
        .data(DataLoader::new(OrganizationLoader::new(org_client.clone()), tokio::spawn))
        .data(DataLoader::new(TeamLoader::new(team_client.clone()), tokio::spawn))
        .data(DataLoader::new(TeamsByOrganizationLoader::new(team_client), tokio::spawn))
        .data(DataLoader::new(UsersByOrganizationLoader::new(user_client), tokio::spawn))
        .data(author_client.clone())
        .data(post_client.clone())
        .data(DataLoader::new(AuthorLoader::new(author_client), tokio::spawn))
        .data(DataLoader::new(PostLoader::new(post_client.clone()), tokio::spawn))
        .data(DataLoader::new(PostsByAuthorLoader::new(post_client), tokio::spawn))
        .finish();

    let addr: SocketAddr = std::env::var("GATEWAY_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:4000".into())
        .parse()?;

    tracing::info!("Gateway listening on {}", addr);
    tracing::info!("Apollo Sandbox at http://{}/", addr);

    let app = Router::new()
        .route("/graphql", get(apollo_sandbox).post(graphql_handler))
        .route("/", get(apollo_sandbox))
        .with_state(schema);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
