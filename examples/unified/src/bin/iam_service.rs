//! IAM gRPC Service binary
//!
//! Run with: cargo run --bin iam-service --features iam-service --no-default-features

use std::net::SocketAddr;

use sea_orm::Database;
use tonic::transport::Server;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use synapse_unified_example::iam::{
    user_service_server::UserServiceServer,
    organization_service_server::OrganizationServiceServer,
    team_service_server::TeamServiceServer,
    SeaOrmUserServiceStorage,
    SeaOrmOrganizationServiceStorage,
    SeaOrmTeamServiceStorage,
    UserServiceGrpcService,
    OrganizationServiceGrpcService,
    TeamServiceGrpcService,
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
        .unwrap_or_else(|_| "postgres://synapse:synapse@localhost:5432/synapse_iam".into());

    tracing::info!("Connecting to database...");
    let db = Database::connect(&database_url).await?;
    tracing::info!("Database connected!");

    let user_storage = SeaOrmUserServiceStorage::new(db.clone());
    let org_storage = SeaOrmOrganizationServiceStorage::new(db.clone());
    let team_storage = SeaOrmTeamServiceStorage::new(db);

    let user_grpc = UserServiceGrpcService::new(user_storage);
    let org_grpc = OrganizationServiceGrpcService::new(org_storage);
    let team_grpc = TeamServiceGrpcService::new(team_storage);

    let addr: SocketAddr = std::env::var("IAM_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:50052".into())
        .parse()?;

    tracing::info!("IAM gRPC service listening on {}", addr);

    Server::builder()
        .add_service(UserServiceServer::new(user_grpc))
        .add_service(OrganizationServiceServer::new(org_grpc))
        .add_service(TeamServiceServer::new(team_grpc))
        .serve(addr)
        .await?;

    Ok(())
}
