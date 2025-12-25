//! Relay Node interface
//! @generated
#![allow(missing_docs)]
#![allow(unused_imports)]
use async_graphql::{Interface, Object, Context, Result, ID};
use tonic::transport::Channel;
/// Relay Node interface - allows fetching any object by global ID
#[derive(Interface)]
#[graphql(field(name = "id", ty = "ID"))]
pub enum Node {
    User(User),
    Post(Post),
}
/// Root query for fetching nodes by global ID
pub struct NodeQuery;
#[Object]
impl NodeQuery {
    /// Fetch any node by its global ID
    async fn node(&self, ctx: &Context<'_>, id: ID) -> Result<Option<Node>> {
        let (type_name, local_id) = decode_global_id(&id)
            .ok_or_else(|| async_graphql::Error::new("Invalid node ID"))?;
        match type_name.as_str() {
            "User" => {
                let loader = ctx
                    .data_unchecked::<
                        async_graphql::dataloader::DataLoader<UserLoader>,
                    >();
                let entity = loader.load_one(local_id).await?;
                Ok(entity.map(Node::User))
            }
            "Post" => {
                let loader = ctx
                    .data_unchecked::<
                        async_graphql::dataloader::DataLoader<PostLoader>,
                    >();
                let entity = loader.load_one(local_id).await?;
                Ok(entity.map(Node::Post))
            }
            _ => Ok(None),
        }
    }
    /// Fetch multiple nodes by their global IDs
    async fn nodes(&self, ctx: &Context<'_>, ids: Vec<ID>) -> Result<Vec<Option<Node>>> {
        let mut results = Vec::with_capacity(ids.len());
        for id in ids {
            let parsed = decode_global_id(&id);
            if let Some((type_name, local_id)) = parsed {
                match type_name.as_str() {
                    "User" => {
                        let loader = ctx
                            .data_unchecked::<
                                async_graphql::dataloader::DataLoader<UserLoader>,
                            >();
                        let entity = loader.load_one(local_id).await?;
                        results.push(entity.map(Node::User));
                    }
                    "Post" => {
                        let loader = ctx
                            .data_unchecked::<
                                async_graphql::dataloader::DataLoader<PostLoader>,
                            >();
                        let entity = loader.load_one(local_id).await?;
                        results.push(entity.map(Node::Post));
                    }
                    _ => results.push(None),
                }
            } else {
                results.push(None);
            }
        }
        Ok(results)
    }
}
/// Encode a local ID to a global Relay ID using base62
pub fn encode_global_id(type_name: &str, local_id: i64) -> ID {
    let raw = format!("{}:{}", type_name, local_id);
    ID(base62::encode(raw.as_bytes()))
}
/// Decode a global Relay ID to type name and local ID
pub fn decode_global_id(id: &ID) -> Option<(String, i64)> {
    let bytes = base62::decode(id.as_str()).ok()?;
    let s = String::from_utf8(bytes).ok()?;
    let (type_name, local_id) = s.split_once(':')?;
    let id = local_id.parse().ok()?;
    Some((type_name.to_string(), id))
}
/// Cursor encoding for pagination
pub fn encode_cursor(id: i64) -> String {
    base62::encode(id.to_string().as_bytes())
}
/// Cursor decoding for pagination
pub fn decode_cursor(cursor: &str) -> Option<i64> {
    base62::decode(cursor)
        .ok()
        .and_then(|b| String::from_utf8(b).ok())
        .and_then(|s| s.parse().ok())
}
