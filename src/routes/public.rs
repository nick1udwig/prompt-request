use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header::CONTENT_TYPE, HeaderValue},
    response::Response,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{auth::ClientIp, error::ApiError, util::ContentKind, AppState};

#[derive(Deserialize)]
pub struct RevQuery {
    pub rev: Option<i32>,
}

#[derive(sqlx::FromRow)]
struct ObjectRow {
    object_key: String,
    content_type: String,
}

pub async fn front_page(
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
) -> Result<Response, ApiError> {
    state.public_read_limiter.check(&ip.to_string())?;

    let mut resp = Response::new(Body::from(state.front_page.as_str().to_string()));
    resp.headers_mut().insert(
        CONTENT_TYPE,
        HeaderValue::from_static("text/markdown; charset=utf-8"),
    );
    Ok(resp)
}

pub async fn get_raw(
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    Path(uuid): Path<Uuid>,
    Query(q): Query<RevQuery>,
) -> Result<Response, ApiError> {
    state.public_read_limiter.check(&ip.to_string())?;

    let row = if let Some(rev) = q.rev {
        if rev < 1 {
            return Err(ApiError::BadRequest("rev must be >= 1".to_string()));
        }
        sqlx::query_as::<_, ObjectRow>(
            "SELECT object_key, content_type FROM request_revisions WHERE request_uuid = $1 AND rev_number = $2",
        )
        .bind(uuid)
        .bind(rev)
        .fetch_optional(&state.pool)
        .await?
    } else {
        sqlx::query_as::<_, ObjectRow>(
            "SELECT rr.object_key, rr.content_type \
             FROM request_revisions rr \
             JOIN requests r ON r.uuid = rr.request_uuid \
             WHERE r.uuid = $1 AND rr.rev_number = r.latest_rev",
        )
        .bind(uuid)
        .fetch_optional(&state.pool)
        .await?
    }
    .ok_or(ApiError::NotFound)?;

    let bytes = state.store.get(&row.object_key).await?;

    let content_type = match row.content_type.as_str() {
        "text/markdown" => ContentKind::Markdown.response_type(),
        _ => row.content_type.as_str(),
    };

    let mut resp = Response::new(Body::from(bytes));
    let header = HeaderValue::from_str(content_type)
        .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream"));
    resp.headers_mut().insert(CONTENT_TYPE, header);
    Ok(resp)
}
