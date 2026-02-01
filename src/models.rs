use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize)]
pub struct CreateAccountResponse {
    pub api_key: String,
}

#[derive(Serialize)]
pub struct RequestCreatedResponse {
    pub uuid: Uuid,
    pub rev: i32,
    pub content_type: String,
    pub size_bytes: i32,
    pub sha256: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct RequestListItem {
    pub uuid: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub latest_rev: i32,
    pub latest_content_type: String,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct RevisionInfo {
    pub rev: i32,
    pub created_at: DateTime<Utc>,
    pub content_type: String,
    pub size_bytes: i32,
    pub sha256: String,
}
