use std::time::Duration;

use aws_config::{BehaviorVersion, Region};
use aws_credential_types::Credentials;
use aws_sdk_s3::{primitives::ByteStream, Client as S3Client};
use bytes::Bytes;
use tokio::time::sleep;

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
        let mut delay = Duration::from_millis(200);
        let mut last_err: Option<String> = None;

        for attempt in 1..=6 {
            match self.client.head_bucket().bucket(&self.bucket).send().await {
                Ok(_) => return Ok(()),
                Err(head_err) => {
                    let create = self
                        .client
                        .create_bucket()
                        .bucket(&self.bucket)
                        .send()
                        .await;
                    match create {
                        Ok(_) => return Ok(()),
                        Err(create_err) => {
                            // If the bucket appeared between calls, treat it as success.
                            if self
                                .client
                                .head_bucket()
                                .bucket(&self.bucket)
                                .send()
                                .await
                                .is_ok()
                            {
                                return Ok(());
                            }
                            last_err = Some(format!(
                                "head error: {head_err}; create error: {create_err}"
                            ));
                        }
                    }
                }
            }

            if attempt < 6 {
                tracing::warn!(
                    attempt,
                    delay_ms = delay.as_millis(),
                    "s3 not ready, retrying"
                );
                sleep(delay).await;
                delay = (delay * 2).min(Duration::from_secs(5));
            }
        }

        Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            last_err.unwrap_or_else(|| "s3 bucket check failed".to_string()),
        )))
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
