//! Blog gRPC Service binary
//!
//! Run with: cargo run --bin blog-service --features blog-service --no-default-features

use std::net::SocketAddr;

use sea_orm::Database;
use tonic::transport::Server;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use synapse_unified_example::blog::{
    author_service_server::AuthorServiceServer,
    post_service_server::PostServiceServer,
    SeaOrmAuthorServiceStorage,
    SeaOrmPostServiceStorage,
    AuthorServiceGrpcService,
    PostServiceGrpcService,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    dotenvy::dotenv().ok();

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://synapse:synapse@localhost:5432/synapse_blog".into());

    tracing::info!("Connecting to database...");
    let db = Database::connect(&database_url).await?;
    tracing::info!("Database connected!");

    let author_storage = SeaOrmAuthorServiceStorage::new(db.clone());
    let post_storage = SeaOrmPostServiceStorage::new(db);

    let author_grpc = AuthorServiceGrpcService::new(author_storage);
    let post_grpc = PostServiceGrpcService::new(post_storage);

    let addr: SocketAddr = std::env::var("BLOG_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:50053".into())
        .parse()?;

    tracing::info!("Blog gRPC service listening on {}", addr);

    Server::builder()
        .add_service(AuthorServiceServer::new(author_grpc))
        .add_service(PostServiceServer::new(post_grpc))
        .serve(addr)
        .await?;

    Ok(())
}
