use async_trait::async_trait;
use axum::{
    extract::{FromRequestParts, State},
    http::{header::AUTHORIZATION, request::Parts},
};
use std::net::{IpAddr, SocketAddr};

use crate::{error::ApiError, util::hash_api_key, AppState};

#[derive(Clone, Debug)]
pub struct AuthContext {
    pub account_id: i64,
}

#[async_trait]
impl FromRequestParts<AppState> for AuthContext {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let State(app) = State::<AppState>::from_request_parts(parts, state)
            .await
            .map_err(|_| ApiError::Internal("state unavailable".to_string()))?;

        let header = parts
            .headers
            .get(AUTHORIZATION)
            .ok_or(ApiError::Unauthorized)?;
        let header = header.to_str().map_err(|_| ApiError::Unauthorized)?;
        let key = header
            .strip_prefix("Bearer ")
            .ok_or(ApiError::Unauthorized)?;
        if key.trim().is_empty() {
            return Err(ApiError::Unauthorized);
        }

        let hash = hash_api_key(key, app.api_key_pepper.as_deref());
        #[derive(sqlx::FromRow)]
        struct AccountRow {
            id: i64,
        }

        let row = sqlx::query_as::<_, AccountRow>(
            r#"SELECT id FROM accounts WHERE api_key_hash = $1"#,
        )
        .bind(hash)
        .fetch_optional(&app.pool)
        .await
        .map_err(ApiError::from)?
        .ok_or(ApiError::Unauthorized)?;

        app.account_limiter.check(&row.id.to_string())?;

        let _ = sqlx::query(r#"UPDATE accounts SET last_used_at = now() WHERE id = $1"#)
            .bind(row.id)
            .execute(&app.pool)
            .await;

        Ok(AuthContext { account_id: row.id })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ClientIp(pub IpAddr);

#[async_trait]
impl<S> FromRequestParts<S> for ClientIp
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        if let Some(value) = parts.headers.get("x-forwarded-for") {
            if let Ok(s) = value.to_str() {
                if let Some(first) = s.split(',').next() {
                    if let Ok(ip) = first.trim().parse::<IpAddr>() {
                        return Ok(ClientIp(ip));
                    }
                }
            }
        }

        if let Some(axum::extract::ConnectInfo(addr)) =
            parts.extensions.get::<axum::extract::ConnectInfo<SocketAddr>>()
        {
            return Ok(ClientIp(addr.ip()));
        }

        Err(ApiError::BadRequest("missing client ip".to_string()))
    }
}
