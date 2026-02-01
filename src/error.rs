use axum::{
    http::{header::RETRY_AFTER, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("unauthorized")]
    Unauthorized,
    #[error("not found")]
    NotFound,
    #[error("payload too large")]
    PayloadTooLarge,
    #[error("rate limited")]
    RateLimited { retry_after_secs: u64 },
    #[error("storage error: {0}")]
    Storage(String),
    #[error("database error: {0}")]
    Database(String),
    #[error("internal error: {0}")]
    Internal(String),
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
    message: Option<String>,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            ApiError::BadRequest(msg) => json_error(
                StatusCode::BAD_REQUEST,
                "bad_request",
                Some(msg),
                None,
            ),
            ApiError::Unauthorized => json_error(StatusCode::UNAUTHORIZED, "unauthorized", None, None),
            ApiError::NotFound => json_error(StatusCode::NOT_FOUND, "not_found", None, None),
            ApiError::PayloadTooLarge => {
                json_error(StatusCode::PAYLOAD_TOO_LARGE, "payload_too_large", None, None)
            }
            ApiError::RateLimited { retry_after_secs } => json_error(
                StatusCode::TOO_MANY_REQUESTS,
                "rate_limited",
                None,
                Some(retry_after_secs),
            ),
            ApiError::Storage(msg) => json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "storage",
                Some(msg),
                None,
            ),
            ApiError::Database(msg) => json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "database",
                Some(msg),
                None,
            ),
            ApiError::Internal(msg) => json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal",
                Some(msg),
                None,
            ),
        }
    }
}

fn json_error(
    status: StatusCode,
    code: &str,
    message: Option<String>,
    retry_after: Option<u64>,
) -> Response {
    let body = Json(ErrorBody {
        error: code.to_string(),
        message,
    });
    let mut resp = (status, body).into_response();
    if let Some(secs) = retry_after {
        let _ = resp.headers_mut().insert(
            RETRY_AFTER,
            HeaderValue::from_str(&secs.to_string())
                .unwrap_or_else(|_| HeaderValue::from_static("1")),
        );
    }
    resp
}

impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        ApiError::Database(err.to_string())
    }
}
