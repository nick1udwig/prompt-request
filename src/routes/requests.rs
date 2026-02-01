use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use bytes::Bytes;
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    auth::AuthContext,
    error::ApiError,
    models::{RequestCreatedResponse, RequestListItem, RevisionInfo},
    util::{object_key, parse_content_type, sha256_hex, MAX_UPLOAD_BYTES},
    AppState,
};

#[derive(Deserialize)]
pub struct ListQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Deserialize)]
pub struct RevQuery {
    pub rev: Option<i32>,
}

pub async fn create_request(
    State(state): State<AppState>,
    auth: AuthContext,
    headers: HeaderMap,
    body: Bytes,
) -> Result<(StatusCode, Json<RequestCreatedResponse>), ApiError> {
    if body.len() > MAX_UPLOAD_BYTES {
        return Err(ApiError::PayloadTooLarge);
    }

    let kind = parse_content_type(&headers)?;
    let content_type = kind.canonical_type().to_string();
    let sha256 = sha256_hex(&body);
    let size_bytes = body.len() as i32;

    let uuid = Uuid::new_v4();
    let rev = 1;
    let key = object_key(uuid, rev, kind);

    state.store.put(&key, body.clone(), &content_type).await?;

    let mut tx = state.pool.begin().await?;

    let create_res = sqlx::query(
        "INSERT INTO requests (uuid, account_id, latest_rev) VALUES ($1, $2, $3)",
    )
    .bind(uuid)
    .bind(auth.account_id)
    .bind(rev)
    .execute(&mut *tx)
    .await;

    if let Err(err) = create_res {
        let _ = state.store.delete(&key).await;
        let _ = tx.rollback().await;
        return Err(ApiError::from(err));
    }

    let rev_created_at = sqlx::query_scalar(
        "INSERT INTO request_revisions (request_uuid, rev_number, content_type, size_bytes, sha256, object_key) \
         VALUES ($1, $2, $3, $4, $5, $6) RETURNING created_at",
    )
    .bind(uuid)
    .bind(rev)
    .bind(&content_type)
    .bind(size_bytes)
    .bind(&sha256)
    .bind(&key)
    .fetch_one(&mut *tx)
    .await;

    let rev_created_at = match rev_created_at {
        Ok(value) => value,
        Err(err) => {
            let _ = state.store.delete(&key).await;
            let _ = tx.rollback().await;
            return Err(ApiError::from(err));
        }
    };

    if let Err(err) = tx.commit().await {
        let _ = state.store.delete(&key).await;
        return Err(ApiError::from(err));
    }

    Ok((
        StatusCode::CREATED,
        Json(RequestCreatedResponse {
            uuid,
            rev,
            content_type,
            size_bytes,
            sha256,
            created_at: rev_created_at,
        }),
    ))
}

pub async fn update_request(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(uuid): Path<Uuid>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<(StatusCode, Json<RequestCreatedResponse>), ApiError> {
    if body.len() > MAX_UPLOAD_BYTES {
        return Err(ApiError::PayloadTooLarge);
    }

    let kind = parse_content_type(&headers)?;
    let content_type = kind.canonical_type().to_string();
    let sha256 = sha256_hex(&body);
    let size_bytes = body.len() as i32;

    let mut tx = state.pool.begin().await?;

    let latest_rev: i32 = sqlx::query_scalar(
        "SELECT latest_rev FROM requests WHERE uuid = $1 AND account_id = $2 FOR UPDATE",
    )
    .bind(uuid)
    .bind(auth.account_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(ApiError::NotFound)?;

    let next_rev = latest_rev + 1;
    let key = object_key(uuid, next_rev, kind);

    if let Err(err) = state.store.put(&key, body.clone(), &content_type).await {
        let _ = tx.rollback().await;
        return Err(err);
    }

    let rev_created_at = sqlx::query_scalar(
        "INSERT INTO request_revisions (request_uuid, rev_number, content_type, size_bytes, sha256, object_key) \
         VALUES ($1, $2, $3, $4, $5, $6) RETURNING created_at",
    )
    .bind(uuid)
    .bind(next_rev)
    .bind(&content_type)
    .bind(size_bytes)
    .bind(&sha256)
    .bind(&key)
    .fetch_one(&mut *tx)
    .await;

    let rev_created_at = match rev_created_at {
        Ok(value) => value,
        Err(err) => {
            let _ = state.store.delete(&key).await;
            let _ = tx.rollback().await;
            return Err(ApiError::from(err));
        }
    };

    sqlx::query("UPDATE requests SET latest_rev = $1, updated_at = now() WHERE uuid = $2")
        .bind(next_rev)
        .bind(uuid)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok((
        StatusCode::CREATED,
        Json(RequestCreatedResponse {
            uuid,
            rev: next_rev,
            content_type,
            size_bytes,
            sha256,
            created_at: rev_created_at,
        }),
    ))
}

pub async fn list_requests(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<RequestListItem>>, ApiError> {
    let limit = q.limit.unwrap_or(50).clamp(1, 100);
    let offset = q.offset.unwrap_or(0).max(0);

    let rows = sqlx::query_as::<_, RequestListItem>(
        "SELECT r.uuid, r.created_at, r.updated_at, r.latest_rev, \
                rr.content_type as latest_content_type \
         FROM requests r \
         JOIN request_revisions rr \
           ON rr.request_uuid = r.uuid AND rr.rev_number = r.latest_rev \
         WHERE r.account_id = $1 \
         ORDER BY r.created_at DESC \
         LIMIT $2 OFFSET $3",
    )
    .bind(auth.account_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(rows))
}

pub async fn list_revisions(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(uuid): Path<Uuid>,
) -> Result<Json<Vec<RevisionInfo>>, ApiError> {
    ensure_request_owner(&state, uuid, auth.account_id).await?;

    let rows = sqlx::query_as::<_, RevisionInfo>(
        "SELECT rev_number as rev, created_at, content_type, size_bytes, sha256 \
         FROM request_revisions \
         WHERE request_uuid = $1 \
         ORDER BY rev_number DESC",
    )
    .bind(uuid)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(rows))
}

pub async fn get_revision_metadata(
    State(state): State<AppState>,
    auth: AuthContext,
    Path((uuid, rev)): Path<(Uuid, i32)>,
) -> Result<Json<RevisionInfo>, ApiError> {
    if rev < 1 {
        return Err(ApiError::BadRequest("rev must be >= 1".to_string()));
    }
    ensure_request_owner(&state, uuid, auth.account_id).await?;

    let row = sqlx::query_as::<_, RevisionInfo>(
        "SELECT rev_number as rev, created_at, content_type, size_bytes, sha256 \
         FROM request_revisions \
         WHERE request_uuid = $1 AND rev_number = $2",
    )
    .bind(uuid)
    .bind(rev)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(ApiError::NotFound)?;

    Ok(Json(row))
}

pub async fn delete_request(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(uuid): Path<Uuid>,
    Query(q): Query<RevQuery>,
) -> Result<StatusCode, ApiError> {
    if let Some(rev) = q.rev {
        delete_revision(&state, uuid, auth.account_id, rev).await?;
    } else {
        delete_all(&state, uuid, auth.account_id).await?;
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_revision(
    state: &AppState,
    uuid: Uuid,
    account_id: i64,
    rev: i32,
) -> Result<(), ApiError> {
    if rev < 1 {
        return Err(ApiError::BadRequest("rev must be >= 1".to_string()));
    }

    let mut tx = state.pool.begin().await?;

    let owner: i64 = sqlx::query_scalar("SELECT account_id FROM requests WHERE uuid = $1")
        .bind(uuid)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(ApiError::NotFound)?;

    if owner != account_id {
        return Err(ApiError::NotFound);
    }

    #[derive(sqlx::FromRow)]
    struct ObjectRow {
        object_key: String,
    }

    let row = sqlx::query_as::<_, ObjectRow>(
        "SELECT object_key FROM request_revisions WHERE request_uuid = $1 AND rev_number = $2",
    )
    .bind(uuid)
    .bind(rev)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(ApiError::NotFound)?;

    sqlx::query(
        "DELETE FROM request_revisions WHERE request_uuid = $1 AND rev_number = $2",
    )
    .bind(uuid)
    .bind(rev)
    .execute(&mut *tx)
    .await?;

    let max_rev: Option<i32> = sqlx::query_scalar(
        "SELECT MAX(rev_number) FROM request_revisions WHERE request_uuid = $1",
    )
    .bind(uuid)
    .fetch_one(&mut *tx)
    .await?;

    if let Some(max_rev) = max_rev {
        sqlx::query("UPDATE requests SET latest_rev = $1, updated_at = now() WHERE uuid = $2")
            .bind(max_rev)
            .bind(uuid)
            .execute(&mut *tx)
            .await?;
    } else {
        sqlx::query("DELETE FROM requests WHERE uuid = $1")
            .bind(uuid)
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await?;

    if let Err(err) = state.store.delete(&row.object_key).await {
        tracing::warn!("failed to delete object {}: {}", row.object_key, err);
    }

    Ok(())
}

async fn delete_all(state: &AppState, uuid: Uuid, account_id: i64) -> Result<(), ApiError> {
    let mut tx = state.pool.begin().await?;

    let owner: i64 = sqlx::query_scalar("SELECT account_id FROM requests WHERE uuid = $1")
        .bind(uuid)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(ApiError::NotFound)?;

    if owner != account_id {
        return Err(ApiError::NotFound);
    }

    let keys: Vec<String> = sqlx::query_scalar(
        "SELECT object_key FROM request_revisions WHERE request_uuid = $1",
    )
    .bind(uuid)
    .fetch_all(&mut *tx)
    .await?;

    sqlx::query("DELETE FROM request_revisions WHERE request_uuid = $1")
        .bind(uuid)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM requests WHERE uuid = $1")
        .bind(uuid)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    for key in keys {
        if let Err(err) = state.store.delete(&key).await {
            tracing::warn!("failed to delete object {}: {}", key, err);
        }
    }

    Ok(())
}

async fn ensure_request_owner(
    state: &AppState,
    uuid: Uuid,
    account_id: i64,
) -> Result<(), ApiError> {
    let exists = sqlx::query_scalar::<_, i64>(
        "SELECT 1 FROM requests WHERE uuid = $1 AND account_id = $2",
    )
    .bind(uuid)
    .bind(account_id)
    .fetch_optional(&state.pool)
    .await?;

    if exists.is_none() {
        return Err(ApiError::NotFound);
    }
    Ok(())
}
