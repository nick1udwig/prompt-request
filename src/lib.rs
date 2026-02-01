pub mod auth;
pub mod config;
pub mod error;
pub mod models;
pub mod ratelimit;
pub mod routes;
pub mod storage;
pub mod util;

use std::{
    net::SocketAddr,
    path::PathBuf,
    sync::Arc,
    time::Duration,
};

use axum::{extract::DefaultBodyLimit, routing::{get, post, put}, Router};
use sqlx::{postgres::PgPoolOptions, PgPool};
use tower_http::{
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};

use crate::{
    config::Config,
    ratelimit::RateLimiter,
    routes::{accounts, public, requests},
    storage::s3::S3Store,
};

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub store: Arc<dyn storage::ObjectStore>,
    pub account_limiter: Arc<RateLimiter>,
    pub public_read_limiter: Arc<RateLimiter>,
    pub account_create_limiter: Arc<RateLimiter>,
    pub front_page: Arc<String>,
    pub api_key_pepper: Option<String>,
    pub frontend_dist: PathBuf,
}

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();
    let cfg = Config::from_env()?;
    let state = build_state(&cfg).await?;
    let app = build_router(state);

    let listener = tokio::net::TcpListener::bind(cfg.bind_addr).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;
    Ok(())
}

pub async fn build_state(cfg: &Config) -> Result<AppState, Box<dyn std::error::Error>> {
    let pool = PgPoolOptions::new()
        .max_connections(cfg.db_max_connections)
        .connect(&cfg.database_url)
        .await?;
    sqlx::migrate!("./migrations").run(&pool).await?;

    let store = S3Store::new(cfg).await?;
    if cfg.s3_create_bucket {
        store.ensure_bucket().await?;
    }

    let front_page = load_front_page(cfg)?;

    Ok(AppState {
        pool,
        store: Arc::new(store),
        account_limiter: Arc::new(RateLimiter::new(Duration::from_secs(1))),
        public_read_limiter: Arc::new(RateLimiter::new(Duration::from_secs(1))),
        account_create_limiter: Arc::new(RateLimiter::new(Duration::from_secs(3600))),
        front_page: Arc::new(front_page),
        api_key_pepper: cfg.api_key_pepper.clone(),
        frontend_dist: cfg.frontend_dist.clone(),
    })
}

pub fn build_router(state: AppState) -> Router {
    let api = Router::new()
        .route("/accounts", post(accounts::create_account))
        .route(
            "/requests",
            post(requests::create_request).get(requests::list_requests),
        )
        .route(
            "/requests/:uuid",
            put(requests::update_request).delete(requests::delete_request),
        )
        .route(
            "/requests/:uuid/revisions",
            get(requests::list_revisions),
        )
        .route(
            "/requests/:uuid/revisions/:rev",
            get(requests::get_revision_metadata),
        )
        .layer(DefaultBodyLimit::max(util::MAX_UPLOAD_BYTES));

    let frontend = frontend_router(state.frontend_dist.clone());

    Router::new()
        .merge(frontend)
        .route("/", get(public::front_page))
        .route("/:uuid", get(public::get_raw))
        .route("/healthz", get(|| async { "ok" }))
        .nest("/api", api)
        .with_state(state)
        .layer(TraceLayer::new_for_http())
}

fn frontend_router(dist: PathBuf) -> Router<AppState> {
    let index = dist.join("index.html");
    let dir = ServeDir::new(dist)
        .append_index_html_on_directories(true)
        .not_found_service(ServeFile::new(index.clone()));

    Router::new()
        .route_service("/h", ServeFile::new(index))
        .route_service("/h/*path", dir)
}

fn init_tracing() {
    let filter = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    tracing_subscriber::fmt().with_env_filter(filter).init();
}

fn load_front_page(cfg: &Config) -> Result<String, Box<dyn std::error::Error>> {
    if let Some(path) = &cfg.front_page_path {
        return Ok(std::fs::read_to_string(path)?);
    }

    if let Ok(contents) = std::fs::read_to_string("frontpage.md") {
        return Ok(contents);
    }

    Ok(include_str!("../frontpage.md").to_string())
}
