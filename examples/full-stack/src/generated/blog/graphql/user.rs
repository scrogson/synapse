//! GraphQL Object type for #msg_name
//! @generated
#![allow(missing_docs)]
#![allow(unused_imports)]
use async_graphql::{Object, Context, Result, ID};
use async_graphql::dataloader::DataLoader;
/// GraphQL object type
pub struct User {
    pub id: i64,
    pub email: String,
    pub name: String,
    pub bio: Option<String>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}
#[Object]
impl User {
    /// Relay global ID
    async fn id(&self) -> ID {
        let raw = format!("{}:{}", "User", self.id);
        ID(base62::encode(raw.as_bytes()))
    }
    /// Internal database ID
    async fn internal_id(&self) -> ID {
        ID(self.id.to_string())
    }
    async fn email(&self) -> &str {
        &self.email
    }
    async fn name(&self) -> &str {
        &self.name
    }
    async fn bio(&self) -> Option<&str> {
        self.bio.as_deref()
    }
    async fn created_at(&self) -> Timestamp {
        self.created_at.clone()
    }
    async fn updated_at(&self) -> Timestamp {
        self.updated_at.clone()
    }
}
impl From<proto::User> for User {
    fn from(proto: proto::User) -> Self {
        Self {
            id: proto.id,
            email: proto.email,
            name: proto.name,
            bio: proto.bio,
            created_at: proto.created_at,
            updated_at: proto.updated_at,
        }
    }
}
