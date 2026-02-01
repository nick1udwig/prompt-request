use aws_config::{BehaviorVersion, Region};
use aws_credential_types::Credentials;
use aws_sdk_s3::{primitives::ByteStream, Client as S3Client};
use bytes::Bytes;

use crate::{config::Config, error::ApiError, storage::ObjectStore};

pub struct S3Store {
    client: S3Client,
    bucket: String,
}

impl S3Store {
    pub async fn new(cfg: &Config) -> Result<Self, Box<dyn std::error::Error>> {
        let mut loader =
            aws_config::defaults(BehaviorVersion::latest()).region(Region::new(cfg.s3_region.clone()));

        if let Some(endpoint) = &cfg.s3_endpoint {
            loader = loader.endpoint_url(endpoint);
        }

        if let (Some(access), Some(secret)) = (&cfg.s3_access_key, &cfg.s3_secret_key) {
            let creds = Credentials::new(access, secret, None, None, "env");
            loader = loader.credentials_provider(creds);
        }

        let shared = loader.load().await;
        let mut s3_config = aws_sdk_s3::config::Builder::from(&shared);
        if cfg.s3_force_path_style {
            s3_config = s3_config.force_path_style(true);
        }
        let client = S3Client::from_conf(s3_config.build());

        Ok(Self {
            client,
            bucket: cfg.s3_bucket.clone(),
        })
    }

    pub async fn ensure_bucket(&self) -> Result<(), Box<dyn std::error::Error>> {
        let head = self.client.head_bucket().bucket(&self.bucket).send().await;
        if head.is_err() {
            self.client
                .create_bucket()
                .bucket(&self.bucket)
                .send()
                .await?;
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl ObjectStore for S3Store {
    async fn put(&self, key: &str, bytes: Bytes, content_type: &str) -> Result<(), ApiError> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(ByteStream::from(bytes))
            .content_type(content_type)
            .send()
            .await
            .map_err(|e| ApiError::Storage(e.to_string()))?;
        Ok(())
    }

    async fn get(&self, key: &str) -> Result<Bytes, ApiError> {
        let resp = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| ApiError::Storage(e.to_string()))?;

        let data = resp
            .body
            .collect()
            .await
            .map_err(|e| ApiError::Storage(e.to_string()))?
            .into_bytes();

        Ok(data)
    }

    async fn delete(&self, key: &str) -> Result<(), ApiError> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| ApiError::Storage(e.to_string()))?;
        Ok(())
    }
}
