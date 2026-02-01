use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};

use crate::{
    auth::ClientIp,
    error::ApiError,
    models::CreateAccountResponse,
    util::{generate_api_key, hash_api_key},
    AppState,
};

pub async fn create_account(
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
) -> Result<impl IntoResponse, ApiError> {
    state.account_create_limiter.check(&ip.to_string())?;

    let api_key = generate_api_key();
    let hash = hash_api_key(&api_key, state.api_key_pepper.as_deref());

    sqlx::query("INSERT INTO accounts (api_key_hash) VALUES ($1)")
        .bind(hash)
        .execute(&state.pool)
        .await?;

    Ok((StatusCode::CREATED, Json(CreateAccountResponse { api_key })))
}
