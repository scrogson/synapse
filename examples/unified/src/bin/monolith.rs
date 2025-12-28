//! Monolith binary - runs everything in one process
//!
//! - IAM gRPC service
//! - Blog gRPC service
//! - GraphQL gateway
//!
//! Run with: cargo run --bin monolith

use std::net::SocketAddr;
use std::time::Duration;

use async_graphql::{EmptySubscription, MergedObject, Schema};
use async_graphql::dataloader::DataLoader;
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::{extract::State, routing::get, Router};
use sea_orm::Database;
use tonic::transport::{Channel, Server as TonicServer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use synapse_unified_example::{
    iam::{
        user_service_server::UserServiceServer,
        organization_service_server::OrganizationServiceServer,
        team_service_server::TeamServiceServer,
        user_service_client::UserServiceClient,
        organization_service_client::OrganizationServiceClient,
        team_service_client::TeamServiceClient,
        SeaOrmUserServiceStorage,
        SeaOrmOrganizationServiceStorage,
        SeaOrmTeamServiceStorage,
        UserServiceGrpcService,
        OrganizationServiceGrpcService,
        TeamServiceGrpcService,
        graphql::{
            UserServiceQuery, UserServiceMutation,
            OrganizationServiceQuery, OrganizationServiceMutation,
            TeamServiceQuery, TeamServiceMutation,
            UserLoader, OrganizationLoader, TeamLoader,
            TeamsByOrganizationLoader, UsersByOrganizationLoader,
        },
    },
    blog::{
        author_service_server::AuthorServiceServer,
        post_service_server::PostServiceServer,
        author_service_client::AuthorServiceClient,
        post_service_client::PostServiceClient,
        SeaOrmAuthorServiceStorage,
        SeaOrmPostServiceStorage,
        AuthorServiceGrpcService,
        PostServiceGrpcService,
        graphql::{
            AuthorServiceQuery, AuthorServiceMutation,
            PostServiceQuery, PostServiceMutation,
            AuthorLoader, PostLoader, PostsByAuthorLoader,
        },
    },
};

/// Combined Query type
#[derive(MergedObject, Default)]
pub struct Query(
    UserServiceQuery,
    OrganizationServiceQuery,
    TeamServiceQuery,
    AuthorServiceQuery,
    PostServiceQuery,
);

/// Combined Mutation type
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
    <title>Synapse Unified - Apollo Sandbox</title>
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

    // Connect to database
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://synapse:synapse@localhost:5432/synapse".into());

    tracing::info!("Connecting to database...");
    let db = Database::connect(&database_url).await?;
    tracing::info!("Database connected!");

    // Create storage implementations
    let user_storage = SeaOrmUserServiceStorage::new(db.clone());
    let org_storage = SeaOrmOrganizationServiceStorage::new(db.clone());
    let team_storage = SeaOrmTeamServiceStorage::new(db.clone());
    let author_storage = SeaOrmAuthorServiceStorage::new(db.clone());
    let post_storage = SeaOrmPostServiceStorage::new(db.clone());

    // Wrap in gRPC services
    let user_grpc = UserServiceGrpcService::new(user_storage);
    let org_grpc = OrganizationServiceGrpcService::new(org_storage);
    let team_grpc = TeamServiceGrpcService::new(team_storage);
    let author_grpc = AuthorServiceGrpcService::new(author_storage);
    let post_grpc = PostServiceGrpcService::new(post_storage);

    // gRPC server address
    let grpc_addr: SocketAddr = std::env::var("GRPC_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:50051".into())
        .parse()?;

    // GraphQL server address
    let graphql_addr: SocketAddr = std::env::var("GRAPHQL_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:4000".into())
        .parse()?;

    tracing::info!("Starting gRPC server on {}", grpc_addr);

    // Start gRPC server
    let grpc_server = TonicServer::builder()
        .add_service(UserServiceServer::new(user_grpc))
        .add_service(OrganizationServiceServer::new(org_grpc))
        .add_service(TeamServiceServer::new(team_grpc))
        .add_service(AuthorServiceServer::new(author_grpc))
        .add_service(PostServiceServer::new(post_grpc))
        .serve(grpc_addr);

    tokio::spawn(async move {
        if let Err(e) = grpc_server.await {
            tracing::error!("gRPC server error: {}", e);
        }
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Create gRPC clients for GraphQL
    let grpc_endpoint = format!("http://127.0.0.1:{}", grpc_addr.port());
    tracing::info!("Connecting GraphQL to gRPC at {}", grpc_endpoint);

    let channel = Channel::from_shared(grpc_endpoint)?
        .connect()
        .await?;

    let user_client = UserServiceClient::new(channel.clone());
    let org_client = OrganizationServiceClient::new(channel.clone());
    let team_client = TeamServiceClient::new(channel.clone());
    let author_client = AuthorServiceClient::new(channel.clone());
    let post_client = PostServiceClient::new(channel);

    // Build GraphQL schema
    let schema = Schema::build(Query::default(), Mutation::default(), EmptySubscription)
        // IAM clients and loaders
        .data(user_client.clone())
        .data(org_client.clone())
        .data(team_client.clone())
        .data(DataLoader::new(UserLoader::new(user_client.clone()), tokio::spawn))
        .data(DataLoader::new(OrganizationLoader::new(org_client.clone()), tokio::spawn))
        .data(DataLoader::new(TeamLoader::new(team_client.clone()), tokio::spawn))
        .data(DataLoader::new(TeamsByOrganizationLoader::new(team_client), tokio::spawn))
        .data(DataLoader::new(UsersByOrganizationLoader::new(user_client), tokio::spawn))
        // Blog clients and loaders
        .data(author_client.clone())
        .data(post_client.clone())
        .data(DataLoader::new(AuthorLoader::new(author_client), tokio::spawn))
        .data(DataLoader::new(PostLoader::new(post_client.clone()), tokio::spawn))
        .data(DataLoader::new(PostsByAuthorLoader::new(post_client), tokio::spawn))
        .finish();

    tracing::info!("GraphQL server listening on {}", graphql_addr);
    tracing::info!("Apollo Sandbox available at http://{}/", graphql_addr);

    let app = Router::new()
        .route("/graphql", get(apollo_sandbox).post(graphql_handler))
        .route("/", get(apollo_sandbox))
        .with_state(schema);

    let listener = tokio::net::TcpListener::bind(graphql_addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
