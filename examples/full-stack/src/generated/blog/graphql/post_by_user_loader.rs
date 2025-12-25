//! DataLoader for #relation_name relation
//! @generated
#![allow(missing_docs)]
#![allow(unused_imports)]
use async_graphql::dataloader::Loader;
use std::collections::HashMap;
use tonic::transport::Channel;
/// DataLoader for fetching #related_type by #parent_type ID
pub struct PostByUserLoader {
    client: PostServiceClient<Channel>,
}
impl PostByUserLoader {
    /// Create a new loader with the given gRPC client
    pub fn new(client: PostServiceClient<Channel>) -> Self {
        Self { client }
    }
}
impl Loader<i64> for PostByUserLoader {
    type Value = Vec<Post>;
    type Error = async_graphql::Error;
    async fn load(
        &self,
        keys: &[i64],
    ) -> Result<HashMap<i64, Self::Value>, Self::Error> {
        let request = BatchRequest { ids: keys.to_vec() };
        let response = self
            .client
            .clone()
            .list_post_by_user_ids(request)
            .await
            .map_err(|e| async_graphql::Error::new(e.message()))?;
        let mut map: HashMap<i64, Self::Value> = HashMap::new();
        for item in response.into_inner().items {
            let key = item.author_id;
            map.entry(key).or_insert_with(Vec::new).push(Post::from(item));
        }
        Ok(map)
    }
}
