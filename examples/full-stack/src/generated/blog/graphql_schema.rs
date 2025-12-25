//! GraphQL schema using async-graphql
//!
//! This module provides Query and Mutation types that use the storage layer.

#![allow(missing_docs)]
#![allow(unused_imports)]

use async_graphql::{Object, Context, Result, ID, InputObject, SimpleObject, EmptySubscription, Schema};
use super::user_service_storage::{UserServiceStorage, StorageError as UserStorageError};
use super::post_service_storage::{PostServiceStorage, StorageError as PostStorageError};
use super::sea_orm_user_service_storage::SeaOrmUserServiceStorage;
use super::sea_orm_post_service_storage::SeaOrmPostServiceStorage;
use std::sync::Arc;

// Import proto types with an alias to avoid conflicts
use super::{
    User as ProtoUser,
    Post as ProtoPost,
    GetUserRequest, GetUserResponse,
    ListUsersRequest, UserConnection as ProtoUserConnection, UserEdge as ProtoUserEdge,
    CreateUserRequest, CreateUserInput as ProtoCreateUserInput, CreateUserResponse,
    UpdateUserRequest, UpdateUserInput as ProtoUpdateUserInput, UpdateUserResponse,
    DeleteUserRequest, DeleteUserResponse,
    GetPostRequest, GetPostResponse,
    ListPostsRequest, PostConnection as ProtoPostConnection, PostEdge as ProtoPostEdge,
    CreatePostRequest, CreatePostInput as ProtoCreatePostInput, CreatePostResponse,
    UpdatePostRequest, UpdatePostInput as ProtoUpdatePostInput, UpdatePostResponse,
    DeletePostRequest, DeletePostResponse,
    PageInfo as ProtoPageInfo,
};

// ============================================================================
// GraphQL Types
// ============================================================================

/// User type
#[derive(SimpleObject, Clone)]
pub struct User {
    pub id: ID,
    pub email: String,
    pub name: String,
    pub bio: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<ProtoUser> for User {
    fn from(u: ProtoUser) -> Self {
        Self {
            id: ID(u.id.to_string()),
            email: u.email,
            name: u.name,
            bio: u.bio,
            created_at: u.created_at.map(|t| {
                chrono::DateTime::from_timestamp(t.seconds, t.nanos as u32)
                    .map(|dt| dt.to_rfc3339())
                    .unwrap_or_default()
            }).unwrap_or_default(),
            updated_at: u.updated_at.map(|t| {
                chrono::DateTime::from_timestamp(t.seconds, t.nanos as u32)
                    .map(|dt| dt.to_rfc3339())
                    .unwrap_or_default()
            }).unwrap_or_default(),
        }
    }
}

/// Post type
#[derive(SimpleObject, Clone)]
pub struct Post {
    pub id: ID,
    pub title: String,
    pub content: String,
    pub published: bool,
    pub author_id: ID,
    pub created_at: String,
    pub updated_at: String,
}

impl From<ProtoPost> for Post {
    fn from(p: ProtoPost) -> Self {
        Self {
            id: ID(p.id.to_string()),
            title: p.title,
            content: p.content,
            published: p.published,
            author_id: ID(p.author_id.to_string()),
            created_at: p.created_at.map(|t| {
                chrono::DateTime::from_timestamp(t.seconds, t.nanos as u32)
                    .map(|dt| dt.to_rfc3339())
                    .unwrap_or_default()
            }).unwrap_or_default(),
            updated_at: p.updated_at.map(|t| {
                chrono::DateTime::from_timestamp(t.seconds, t.nanos as u32)
                    .map(|dt| dt.to_rfc3339())
                    .unwrap_or_default()
            }).unwrap_or_default(),
        }
    }
}

/// Relay PageInfo
#[derive(SimpleObject, Clone)]
pub struct PageInfo {
    pub has_next_page: bool,
    pub has_previous_page: bool,
    pub start_cursor: Option<String>,
    pub end_cursor: Option<String>,
}

/// Relay UserEdge
#[derive(SimpleObject, Clone)]
pub struct UserEdge {
    pub cursor: String,
    pub node: User,
}

/// Relay UserConnection
#[derive(SimpleObject, Clone)]
pub struct UserConnection {
    pub edges: Vec<UserEdge>,
    pub page_info: PageInfo,
}

/// Relay PostEdge
#[derive(SimpleObject, Clone)]
pub struct PostEdge {
    pub cursor: String,
    pub node: Post,
}

/// Relay PostConnection
#[derive(SimpleObject, Clone)]
pub struct PostConnection {
    pub edges: Vec<PostEdge>,
    pub page_info: PageInfo,
}

// ============================================================================
// Input Types
// ============================================================================

#[derive(InputObject)]
pub struct CreateUserInput {
    pub email: String,
    pub name: String,
    pub bio: Option<String>,
}

#[derive(InputObject)]
pub struct UpdateUserInput {
    pub email: Option<String>,
    pub name: Option<String>,
    pub bio: Option<String>,
}

#[derive(InputObject)]
pub struct CreatePostInput {
    pub title: String,
    pub content: String,
    pub published: bool,
    pub author_id: i64,
}

#[derive(InputObject)]
pub struct UpdatePostInput {
    pub title: Option<String>,
    pub content: Option<String>,
    pub published: Option<bool>,
}

// ============================================================================
// Query
// ============================================================================

pub struct Query;

#[Object]
impl Query {
    /// Get a user by ID
    async fn user(&self, ctx: &Context<'_>, id: i64) -> Result<Option<User>> {
        let storage = ctx.data_unchecked::<Arc<SeaOrmUserServiceStorage>>();
        let request = GetUserRequest { id };
        match storage.get_user(request).await {
            Ok(response) => Ok(response.user.map(User::from)),
            Err(UserStorageError::NotFound(_)) => Ok(None),
            Err(e) => Err(async_graphql::Error::new(e.to_string())),
        }
    }

    /// List users with Relay-style pagination
    async fn users(
        &self,
        ctx: &Context<'_>,
        after: Option<String>,
        before: Option<String>,
        first: Option<i32>,
        last: Option<i32>,
    ) -> Result<UserConnection> {
        let storage = ctx.data_unchecked::<Arc<SeaOrmUserServiceStorage>>();
        let request = ListUsersRequest { after, before, first, last };
        let response = storage.list_users(request).await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let page_info = response.page_info.map(|p| PageInfo {
            has_next_page: p.has_next_page,
            has_previous_page: p.has_previous_page,
            start_cursor: p.start_cursor,
            end_cursor: p.end_cursor,
        }).unwrap_or(PageInfo {
            has_next_page: false,
            has_previous_page: false,
            start_cursor: None,
            end_cursor: None,
        });

        let edges = response.edges.into_iter().map(|e| {
            UserEdge {
                cursor: e.cursor,
                node: e.node.map(User::from).unwrap_or_else(|| User {
                    id: ID::from(""),
                    email: String::new(),
                    name: String::new(),
                    bio: None,
                    created_at: String::new(),
                    updated_at: String::new(),
                }),
            }
        }).collect();

        Ok(UserConnection { edges, page_info })
    }

    /// Get a post by ID
    async fn post(&self, ctx: &Context<'_>, id: i64) -> Result<Option<Post>> {
        let storage = ctx.data_unchecked::<Arc<SeaOrmPostServiceStorage>>();
        let request = GetPostRequest { id };
        match storage.get_post(request).await {
            Ok(response) => Ok(response.post.map(Post::from)),
            Err(PostStorageError::NotFound(_)) => Ok(None),
            Err(e) => Err(async_graphql::Error::new(e.to_string())),
        }
    }

    /// List posts with Relay-style pagination
    async fn posts(
        &self,
        ctx: &Context<'_>,
        after: Option<String>,
        before: Option<String>,
        first: Option<i32>,
        last: Option<i32>,
    ) -> Result<PostConnection> {
        let storage = ctx.data_unchecked::<Arc<SeaOrmPostServiceStorage>>();
        let request = ListPostsRequest { after, before, first, last };
        let response = storage.list_posts(request).await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let page_info = response.page_info.map(|p| PageInfo {
            has_next_page: p.has_next_page,
            has_previous_page: p.has_previous_page,
            start_cursor: p.start_cursor,
            end_cursor: p.end_cursor,
        }).unwrap_or(PageInfo {
            has_next_page: false,
            has_previous_page: false,
            start_cursor: None,
            end_cursor: None,
        });

        let edges = response.edges.into_iter().map(|e| {
            PostEdge {
                cursor: e.cursor,
                node: e.node.map(Post::from).unwrap_or_else(|| Post {
                    id: ID::from(""),
                    title: String::new(),
                    content: String::new(),
                    published: false,
                    author_id: ID::from(""),
                    created_at: String::new(),
                    updated_at: String::new(),
                }),
            }
        }).collect();

        Ok(PostConnection { edges, page_info })
    }
}

// ============================================================================
// Mutation
// ============================================================================

pub struct Mutation;

#[Object]
impl Mutation {
    /// Create a new user
    async fn create_user(&self, ctx: &Context<'_>, input: CreateUserInput) -> Result<User> {
        let storage = ctx.data_unchecked::<Arc<SeaOrmUserServiceStorage>>();
        let request = CreateUserRequest {
            input: Some(ProtoCreateUserInput {
                email: input.email,
                name: input.name,
                bio: input.bio,
            }),
        };
        let response = storage.create_user(request).await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        Ok(response.user.map(User::from).ok_or("Failed to create user")?)
    }

    /// Update an existing user
    async fn update_user(&self, ctx: &Context<'_>, id: i64, input: UpdateUserInput) -> Result<User> {
        let storage = ctx.data_unchecked::<Arc<SeaOrmUserServiceStorage>>();
        let request = UpdateUserRequest {
            id,
            input: Some(ProtoUpdateUserInput {
                email: input.email,
                name: input.name,
                bio: input.bio,
            }),
        };
        let response = storage.update_user(request).await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        Ok(response.user.map(User::from).ok_or("Failed to update user")?)
    }

    /// Delete a user
    async fn delete_user(&self, ctx: &Context<'_>, id: i64) -> Result<bool> {
        let storage = ctx.data_unchecked::<Arc<SeaOrmUserServiceStorage>>();
        let request = DeleteUserRequest { id };
        let response = storage.delete_user(request).await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        Ok(response.success)
    }

    /// Create a new post
    async fn create_post(&self, ctx: &Context<'_>, input: CreatePostInput) -> Result<Post> {
        let storage = ctx.data_unchecked::<Arc<SeaOrmPostServiceStorage>>();
        let request = CreatePostRequest {
            input: Some(ProtoCreatePostInput {
                title: input.title,
                content: input.content,
                published: input.published,
                author_id: input.author_id,
            }),
        };
        let response = storage.create_post(request).await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        Ok(response.post.map(Post::from).ok_or("Failed to create post")?)
    }

    /// Update an existing post
    async fn update_post(&self, ctx: &Context<'_>, id: i64, input: UpdatePostInput) -> Result<Post> {
        let storage = ctx.data_unchecked::<Arc<SeaOrmPostServiceStorage>>();
        let request = UpdatePostRequest {
            id,
            input: Some(ProtoUpdatePostInput {
                title: input.title,
                content: input.content,
                published: input.published,
            }),
        };
        let response = storage.update_post(request).await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        Ok(response.post.map(Post::from).ok_or("Failed to update post")?)
    }

    /// Delete a post
    async fn delete_post(&self, ctx: &Context<'_>, id: i64) -> Result<bool> {
        let storage = ctx.data_unchecked::<Arc<SeaOrmPostServiceStorage>>();
        let request = DeletePostRequest { id };
        let response = storage.delete_post(request).await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        Ok(response.success)
    }
}

// ============================================================================
// Schema Builder
// ============================================================================

pub type BlogSchema = Schema<Query, Mutation, EmptySubscription>;

pub fn build_schema(
    user_storage: Arc<SeaOrmUserServiceStorage>,
    post_storage: Arc<SeaOrmPostServiceStorage>,
) -> BlogSchema {
    Schema::build(Query, Mutation, EmptySubscription)
        .data(user_storage)
        .data(post_storage)
        .finish()
}
