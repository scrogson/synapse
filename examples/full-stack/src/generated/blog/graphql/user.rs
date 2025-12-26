//! GraphQL Object type for #msg_name
//! @generated
#![allow(missing_docs)]
#![allow(unused_imports)]
use async_graphql::{Object, Context, Result, ID};
use async_graphql::dataloader::DataLoader;
/// GraphQL object type
#[derive(Clone)]
pub struct User {
    pub id: i64,
    pub email: String,
    pub name: String,
    pub bio: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}
#[Object]
impl User {
    /// Relay global ID
    async fn id(&self) -> ID {
        use base64::Engine;
        let raw = format!("{}:{}", "User", self.id);
        ID(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(raw.as_bytes()))
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
    async fn created_at(&self) -> String {
        self.created_at.clone()
    }
    async fn updated_at(&self) -> String {
        self.updated_at.clone()
    }
    /// Resolve related #relation_name
    async fn posts(
        &self,
        ctx: &Context<'_>,
        first: Option<i32>,
        after: Option<String>,
    ) -> Result<super::PostConnection> {
        use super::super::SeaOrmPostServiceStorage;
        use super::super::{ListPostsRequest, PostFilter, Int64Filter};
        use super::super::PostServiceStorage;
        let storage = ctx.data_unchecked::<std::sync::Arc<SeaOrmPostServiceStorage>>();
        let mut filter = PostFilter::default();
        filter.author_id = Some(Int64Filter {
            eq: Some(self.id),
            ..Default::default()
        });
        let request = ListPostsRequest {
            after,
            before: None,
            first,
            last: None,
            filter: Some(filter),
            order_by: None,
        };
        let response = storage
            .list_posts(request)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        Ok(response.into())
    }
}
impl From<super::super::User> for User {
    fn from(proto: super::super::User) -> Self {
        Self {
            id: proto.id,
            email: proto.email,
            name: proto.name,
            bio: proto.bio,
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
