use async_trait::async_trait;
use bytes::Bytes;

use crate::error::ApiError;

pub mod s3;

#[async_trait]
pub trait ObjectStore: Send + Sync {
    async fn put(&self, key: &str, bytes: Bytes, content_type: &str) -> Result<(), ApiError>;
    async fn get(&self, key: &str) -> Result<Bytes, ApiError>;
    async fn delete(&self, key: &str) -> Result<(), ApiError>;
}
