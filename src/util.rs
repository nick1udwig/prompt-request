use axum::http::{header::CONTENT_TYPE, HeaderMap};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use rand::{rngs::OsRng, RngCore};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::error::ApiError;

pub const MAX_UPLOAD_BYTES: usize = 1_048_576;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContentKind {
    Markdown,
    Jsonl,
}

impl ContentKind {
    pub fn canonical_type(self) -> &'static str {
        match self {
            ContentKind::Markdown => "text/markdown",
            ContentKind::Jsonl => "application/x-ndjson",
        }
    }

    pub fn response_type(self) -> &'static str {
        match self {
            ContentKind::Markdown => "text/markdown; charset=utf-8",
            ContentKind::Jsonl => "application/x-ndjson",
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            ContentKind::Markdown => "md",
            ContentKind::Jsonl => "jsonl",
        }
    }
}

pub fn parse_content_type(headers: &HeaderMap) -> Result<ContentKind, ApiError> {
    let raw = headers
        .get(CONTENT_TYPE)
        .ok_or_else(|| ApiError::BadRequest("missing content-type".to_string()))?
        .to_str()
        .map_err(|_| ApiError::BadRequest("invalid content-type".to_string()))?;

    let base = raw.split(';').next().unwrap_or("").trim().to_ascii_lowercase();

    match base.as_str() {
        "text/markdown" | "text/x-markdown" => Ok(ContentKind::Markdown),
        "application/x-ndjson" | "application/jsonl" | "application/jsonlines" => {
            Ok(ContentKind::Jsonl)
        }
        _ => Err(ApiError::BadRequest(format!(
            "unsupported content-type: {base}"
        ))),
    }
}

pub fn generate_api_key() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    let encoded = URL_SAFE_NO_PAD.encode(bytes);
    format!("prq_{encoded}")
}

pub fn hash_api_key(key: &str, pepper: Option<&str>) -> String {
    let mut hasher = Sha256::new();
    if let Some(pepper) = pepper {
        hasher.update(pepper.as_bytes());
    }
    hasher.update(key.as_bytes());
    hex::encode(hasher.finalize())
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

pub fn object_key(uuid: Uuid, rev: i32, kind: ContentKind) -> String {
    format!("requests/{uuid}/rev-{rev}.{}", kind.extension())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_markdown() {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, "text/markdown".parse().unwrap());
        let kind = parse_content_type(&headers).unwrap();
        assert_eq!(kind, ContentKind::Markdown);
    }

    #[test]
    fn parse_jsonl() {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, "application/x-ndjson".parse().unwrap());
        let kind = parse_content_type(&headers).unwrap();
        assert_eq!(kind, ContentKind::Jsonl);
    }
}
