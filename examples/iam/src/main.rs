//! IAM Service - Identity and Access Management
//!
//! This service provides gRPC APIs for managing:
//! - Organizations
//! - Teams
//! - Users
//!
//! It exposes a gRPC server that can be called by other services or a gateway.

use synapse_iam_example::iam::{
    organization_service_server::OrganizationServiceServer,
    team_service_server::TeamServiceServer,
    user_service_server::UserServiceServer,
    OrganizationServiceGrpcService, SeaOrmOrganizationServiceStorage,
    TeamServiceGrpcService, SeaOrmTeamServiceStorage,
    UserServiceGrpcService, SeaOrmUserServiceStorage,
};
use sea_orm::Database;
use tonic::transport::Server;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info,sqlx=info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Connect to database
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/iam".into());

    tracing::info!("Connecting to database...");
    let db = Database::connect(&database_url).await?;
    tracing::info!("Database connected!");

    // Create storage implementations
    let user_storage = SeaOrmUserServiceStorage::new(db.clone());
    let org_storage = SeaOrmOrganizationServiceStorage::new(db.clone());
    let team_storage = SeaOrmTeamServiceStorage::new(db.clone());

    // Create gRPC services
    let user_service = UserServiceGrpcService::new(user_storage);
    let org_service = OrganizationServiceGrpcService::new(org_storage);
    let team_service = TeamServiceGrpcService::new(team_storage);

    // Start gRPC server
    let addr = "0.0.0.0:50061".parse()?;
    tracing::info!("IAM gRPC server listening on {}", addr);

    Server::builder()
        .add_service(UserServiceServer::new(user_service))
        .add_service(OrganizationServiceServer::new(org_service))
        .add_service(TeamServiceServer::new(team_service))
        .serve(addr)
        .await?;

    Ok(())
}
