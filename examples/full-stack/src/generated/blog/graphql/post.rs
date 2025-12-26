//! GraphQL Object type for #msg_name
//! @generated
#![allow(missing_docs)]
#![allow(unused_imports)]
use async_graphql::{Object, Context, Result, ID};
use async_graphql::dataloader::DataLoader;
/// GraphQL object type
#[derive(Clone)]
pub struct Post {
    pub id: i64,
    pub title: String,
    pub content: String,
    pub published: bool,
    pub author_id: i64,
    pub created_at: String,
    pub updated_at: String,
}
#[Object]
impl Post {
    /// Relay global ID
    async fn id(&self) -> ID {
        use base64::Engine;
        let raw = format!("{}:{}", "Post", self.id);
        ID(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(raw.as_bytes()))
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
    async fn created_at(&self) -> String {
        self.created_at.clone()
    }
    async fn updated_at(&self) -> String {
        self.updated_at.clone()
    }
    /// Resolve related #relation_name
    async fn author(&self, ctx: &Context<'_>) -> Result<Option<super::User>> {
        use super::super::SeaOrmUserServiceStorage;
        use super::super::GetUserRequest;
        use super::super::UserServiceStorage;
        let storage = ctx.data_unchecked::<std::sync::Arc<SeaOrmUserServiceStorage>>();
        let request = GetUserRequest {
            id: self.author_id,
        };
        match storage.get_user(request).await {
            Ok(response) => Ok(response.user.map(super::User::from)),
            Err(e) => {
                if e.to_string().contains("not found") {
                    Ok(None)
                } else {
                    Err(async_graphql::Error::new(e.to_string()))
                }
            }
        }
    }
}
impl From<super::super::Post> for Post {
    fn from(proto: super::super::Post) -> Self {
        Self {
            id: proto.id,
            title: proto.title,
            content: proto.content,
            published: proto.published,
            author_id: proto.author_id,
            created_at: proto
                .created_at
                .map(|t| {
                    chrono::DateTime::from_timestamp(t.seconds, t.nanos as u32)
                        .map(|dt| dt.to_rfc3339())
                        .unwrap_or_default()
                })
                .unwrap_or_default(),
            updated_at: proto
                .updated_at
                .map(|t| {
                    chrono::DateTime::from_timestamp(t.seconds, t.nanos as u32)
                        .map(|dt| dt.to_rfc3339())
                        .unwrap_or_default()
                })
                .unwrap_or_default(),
        }
    }
}
