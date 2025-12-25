//! GraphQL Object type for #msg_name
//! @generated
#![allow(missing_docs)]
#![allow(unused_imports)]
use async_graphql::{Object, Context, Result, ID};
use async_graphql::dataloader::DataLoader;
/// GraphQL object type
pub struct Post {
    pub id: i64,
    pub title: String,
    pub content: String,
    pub published: bool,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}
#[Object]
impl Post {
    /// Relay global ID
    async fn id(&self) -> ID {
        let raw = format!("{}:{}", "Post", self.id);
        ID(base62::encode(raw.as_bytes()))
    }
    /// Internal database ID
    async fn internal_id(&self) -> ID {
        ID(self.id.to_string())
    }
    async fn title(&self) -> &str {
        &self.title
    }
    async fn content(&self) -> &str {
        &self.content
    }
    async fn published(&self) -> bool {
        self.published.clone()
    }
    async fn created_at(&self) -> Timestamp {
        self.created_at.clone()
    }
    async fn updated_at(&self) -> Timestamp {
        self.updated_at.clone()
    }
}
impl From<proto::Post> for Post {
    fn from(proto: proto::Post) -> Self {
        Self {
            id: proto.id,
            title: proto.title,
            content: proto.content,
            published: proto.published,
            created_at: proto.created_at,
            updated_at: proto.updated_at,
        }
    }
}
