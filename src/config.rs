use std::{env, net::SocketAddr, path::PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("missing env var {0}")]
    Missing(&'static str),
    #[error("invalid env var {0}: {1}")]
    Invalid(&'static str, String),
}

#[derive(Clone, Debug)]
pub struct Config {
    pub bind_addr: SocketAddr,
    pub database_url: String,
    pub db_max_connections: u32,
    pub s3_endpoint: Option<String>,
    pub s3_region: String,
    pub s3_bucket: String,
    pub s3_access_key: Option<String>,
    pub s3_secret_key: Option<String>,
    pub s3_force_path_style: bool,
    pub s3_create_bucket: bool,
    pub api_key_pepper: Option<String>,
    pub frontend_dist: PathBuf,
    pub front_page_path: Option<PathBuf>,
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let bind_addr = env::var("BIND_ADDR")
            .unwrap_or_else(|_| "0.0.0.0:3000".to_string())
            .parse::<SocketAddr>()
            .map_err(|e| ConfigError::Invalid("BIND_ADDR", e.to_string()))?;

        let database_url =
            env::var("DATABASE_URL").map_err(|_| ConfigError::Missing("DATABASE_URL"))?;

        let db_max_connections = env::var("DB_MAX_CONNECTIONS")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(10);

        let s3_endpoint = env::var("S3_ENDPOINT").ok();
        let s3_region = env::var("S3_REGION").unwrap_or_else(|_| "us-east-1".to_string());
        let s3_bucket = env::var("S3_BUCKET").map_err(|_| ConfigError::Missing("S3_BUCKET"))?;
        let s3_access_key = env::var("S3_ACCESS_KEY_ID").ok();
        let s3_secret_key = env::var("S3_SECRET_ACCESS_KEY").ok();
        let s3_force_path_style = env_bool("S3_FORCE_PATH_STYLE", true);
        let s3_create_bucket = env_bool("S3_CREATE_BUCKET", true);

        let api_key_pepper = env::var("API_KEY_PEPPER").ok();

        let frontend_dist = env::var("FRONTEND_DIST")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("frontend/dist"));

        let front_page_path = env::var("FRONT_PAGE_PATH").map(PathBuf::from).ok();

        Ok(Self {
            bind_addr,
            database_url,
            db_max_connections,
            s3_endpoint,
            s3_region,
            s3_bucket,
            s3_access_key,
            s3_secret_key,
            s3_force_path_style,
            s3_create_bucket,
            api_key_pepper,
            frontend_dist,
            front_page_path,
        })
    }
}

fn env_bool(key: &str, default: bool) -> bool {
    env::var(key)
        .ok()
        .and_then(|v| v.parse::<bool>().ok())
        .unwrap_or(default)
}
