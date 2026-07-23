//! Avatar / object-storage control plane (WBS E2.3).
//!
//! When `MINIO_*` env vars are set, returns S3-compatible path-style URLs for
//! clients to PUT. Without MinIO (default / tests) returns a synthetic upload
//! URL against this API (`PUT /api/v1/me/avatar/blob`) so dogfood works offline.
//! Actual bytes are never required for confirm — clients call confirm with the
//! public URL after a successful PUT (or skip PUT in pure control-plane tests).

use std::time::{SystemTime, UNIX_EPOCH};

use anylive_common::{AppError, ErrorCode};
use anylive_domain::UserId;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use utoipa::ToSchema;
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

/// Env-backed object storage settings.
#[derive(Debug, Clone)]
pub struct ObjectStorageConfig {
    /// e.g. `http://localhost:9000`
    pub endpoint: String,
    pub access_key: String,
    pub secret_key: String,
    pub bucket: String,
    /// Public base for avatar URLs (defaults to `{endpoint}/{bucket}`).
    pub public_base: String,
    /// When false, presign points at the API blob endpoint instead of MinIO.
    pub minio_enabled: bool,
    /// API public base for synthetic upload URLs.
    pub api_public_base: String,
    /// HMAC secret for synthetic upload tokens.
    pub upload_token_secret: String,
}

impl ObjectStorageConfig {
    pub fn from_env() -> Self {
        let endpoint = std::env::var("MINIO_ENDPOINT")
            .unwrap_or_else(|_| "http://localhost:9000".into())
            .trim_end_matches('/')
            .to_string();
        let access_key =
            std::env::var("MINIO_ACCESS_KEY").unwrap_or_else(|_| "minioadmin".into());
        let secret_key =
            std::env::var("MINIO_SECRET_KEY").unwrap_or_else(|_| "minioadmin".into());
        let bucket = std::env::var("MINIO_BUCKET").unwrap_or_else(|_| "anylive".into());
        let public_base = std::env::var("MINIO_PUBLIC_BASE")
            .unwrap_or_else(|_| format!("{endpoint}/{bucket}"))
            .trim_end_matches('/')
            .to_string();
        // Explicit opt-in: MINIO_ENABLED=1 (or presence of non-default endpoint + flag).
        let minio_enabled = std::env::var("MINIO_ENABLED")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        let api_public_base = std::env::var("PAY_PUBLIC_BASE_URL")
            .or_else(|_| std::env::var("API_PUBLIC_BASE_URL"))
            .unwrap_or_else(|_| "http://localhost:8088".into())
            .trim_end_matches('/')
            .to_string();
        let upload_token_secret = std::env::var("AVATAR_UPLOAD_SECRET").unwrap_or_else(|_| {
            std::env::var("JWT_ACCESS_SECRET")
                .unwrap_or_else(|_| "anylive-dev-avatar-upload-secret".into())
        });
        Self {
            endpoint,
            access_key,
            secret_key,
            bucket,
            public_base,
            minio_enabled,
            api_public_base,
            upload_token_secret,
        }
    }

    /// Offline / test default (synthetic API upload, no MinIO).
    pub fn dev() -> Self {
        Self {
            endpoint: "http://localhost:9000".into(),
            access_key: "minioadmin".into(),
            secret_key: "minioadmin".into(),
            bucket: "anylive".into(),
            public_base: "http://localhost:9000/anylive".into(),
            minio_enabled: false,
            api_public_base: "http://localhost:8088".into(),
            upload_token_secret: "anylive-dev-avatar-upload-secret".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AvatarPresignResponse {
    pub object_key: String,
    pub upload_url: String,
    pub public_url: String,
    pub method: String,
    pub expires_in: i64,
    /// Extra headers the client should send on the PUT (e.g. Content-Type).
    pub headers: Vec<AvatarPresignHeader>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AvatarPresignHeader {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct AvatarConfirmBody {
    pub object_key: String,
    /// Optional override; defaults to derived public_url for object_key.
    #[serde(default)]
    pub public_url: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct AvatarPresignBody {
    /// Optional content type; defaults to image/jpeg.
    #[serde(default)]
    pub content_type: Option<String>,
}

/// Build object key `avatars/{user_id}/{uuid}.jpg`.
pub fn avatar_object_key(user_id: UserId) -> String {
    format!("avatars/{}/{}.jpg", user_id.0, Uuid::new_v4())
}

pub fn public_url_for(cfg: &ObjectStorageConfig, object_key: &str) -> String {
    format!("{}/{}", cfg.public_base.trim_end_matches('/'), object_key)
}

/// Create a time-limited presign for avatar upload.
pub fn presign_avatar_put(
    cfg: &ObjectStorageConfig,
    user_id: UserId,
    content_type: &str,
) -> Result<AvatarPresignResponse, AppError> {
    let object_key = avatar_object_key(user_id);
    let public_url = public_url_for(cfg, &object_key);
    let expires_in: i64 = 900;

    if cfg.minio_enabled {
        // Path-style PUT URL. Full SigV4 query signing is optional; clients that
        // hit MinIO with static credentials in dogfood can also use the URL as a
        // hint. We attach a simple HMAC query token derived from secret_key so
        // middleware/proxy can validate if desired.
        let exp = now_epoch() + expires_in as u64;
        let sig = sign_token(
            &cfg.secret_key,
            &format!("PUT\n{object_key}\n{exp}\n{}", cfg.access_key),
        );
        let upload_url = format!(
            "{}/{}/{}?X-AnyLive-Expires={exp}&X-AnyLive-Signature={sig}",
            cfg.endpoint.trim_end_matches('/'),
            cfg.bucket,
            object_key
        );
        return Ok(AvatarPresignResponse {
            object_key,
            upload_url,
            public_url,
            method: "PUT".into(),
            expires_in,
            headers: vec![AvatarPresignHeader {
                name: "Content-Type".into(),
                value: content_type.to_string(),
            }],
        });
    }

    // Synthetic: PUT against API with HMAC token.
    let exp = now_epoch() + expires_in as u64;
    let token = sign_token(
        &cfg.upload_token_secret,
        &format!("avatar-put\n{}\n{object_key}\n{exp}", user_id.0),
    );
    let upload_url = format!(
        "{}/api/v1/me/avatar/blob?object_key={}&expires={exp}&token={token}",
        cfg.api_public_base.trim_end_matches('/'),
        urlencoding_encode(&object_key),
    );
    Ok(AvatarPresignResponse {
        object_key,
        upload_url,
        public_url,
        method: "PUT".into(),
        expires_in,
        headers: vec![AvatarPresignHeader {
            name: "Content-Type".into(),
            value: content_type.to_string(),
        }],
    })
}

/// Validate synthetic blob upload token (API path).
pub fn validate_blob_token(
    cfg: &ObjectStorageConfig,
    user_id: UserId,
    object_key: &str,
    expires: u64,
    token: &str,
) -> Result<(), AppError> {
    if now_epoch() > expires {
        return Err(AppError::new(ErrorCode::Forbidden, "upload token expired"));
    }
    if !object_key.starts_with(&format!("avatars/{}/", user_id.0)) {
        return Err(AppError::validation("object_key does not belong to user"));
    }
    let expected = sign_token(
        &cfg.upload_token_secret,
        &format!("avatar-put\n{}\n{object_key}\n{expires}", user_id.0),
    );
    if !constant_time_eq(expected.as_bytes(), token.as_bytes()) {
        return Err(AppError::new(ErrorCode::Forbidden, "invalid upload token"));
    }
    Ok(())
}

/// Confirm that object_key is owned by user and return the public URL to store.
pub fn resolve_confirm_url(
    cfg: &ObjectStorageConfig,
    user_id: UserId,
    object_key: &str,
    public_url: Option<String>,
) -> Result<String, AppError> {
    if object_key.is_empty() || object_key.len() > 256 {
        return Err(AppError::validation("invalid object_key"));
    }
    if !object_key.starts_with(&format!("avatars/{}/", user_id.0)) {
        return Err(AppError::validation("object_key does not belong to user"));
    }
    if object_key.contains("..") || object_key.contains(' ') {
        return Err(AppError::validation("invalid object_key"));
    }
    if let Some(url) = public_url {
        let url = url.trim().to_string();
        if url.is_empty() || url.len() > 1024 {
            return Err(AppError::validation("invalid public_url"));
        }
        if !(url.starts_with("http://") || url.starts_with("https://")) {
            return Err(AppError::validation("public_url must be http(s)"));
        }
        return Ok(url);
    }
    Ok(public_url_for(cfg, object_key))
}

fn sign_token(secret: &str, payload: &str) -> String {
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC accepts any key length");
    mac.update(payload.as_bytes());
    let bytes = mac.finalize().into_bytes();
    hex::encode(bytes)
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

fn now_epoch() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Minimal path encoding for object keys in query strings.
fn urlencoding_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => {
                out.push(b as char);
            }
            _ => {
                out.push('%');
                out.push_str(&hex::encode([b]).to_uppercase());
            }
        }
    }
    out
}

/// Content hash helper (optional dogfood assertion).
#[allow(dead_code)]
pub fn content_sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presign_synthetic_contains_token() {
        let cfg = ObjectStorageConfig::dev();
        let uid = UserId::new();
        let p = presign_avatar_put(&cfg, uid, "image/jpeg").unwrap();
        assert!(p.object_key.starts_with(&format!("avatars/{}/", uid.0)));
        assert!(p.upload_url.contains("/api/v1/me/avatar/blob"));
        assert!(p.upload_url.contains("token="));
        assert_eq!(p.method, "PUT");
        assert!(p.public_url.contains(&p.object_key));
    }

    #[test]
    fn validate_blob_token_roundtrip() {
        let cfg = ObjectStorageConfig::dev();
        let uid = UserId::new();
        let p = presign_avatar_put(&cfg, uid, "image/png").unwrap();
        // parse query
        let url = p.upload_url;
        let q = url.split('?').nth(1).unwrap();
        let mut object_key = String::new();
        let mut expires = 0u64;
        let mut token = String::new();
        for part in q.split('&') {
            let mut kv = part.splitn(2, '=');
            let k = kv.next().unwrap_or("");
            let v = kv.next().unwrap_or("");
            match k {
                "object_key" => object_key = v.to_string(),
                "expires" => expires = v.parse().unwrap_or(0),
                "token" => token = v.to_string(),
                _ => {}
            }
        }
        validate_blob_token(&cfg, uid, &object_key, expires, &token).unwrap();
        assert!(validate_blob_token(&cfg, uid, &object_key, expires, "bad").is_err());
    }

    #[test]
    fn confirm_rejects_other_user_key() {
        let cfg = ObjectStorageConfig::dev();
        let uid = UserId::new();
        let other = UserId::new();
        let key = format!("avatars/{}/x.jpg", other.0);
        assert!(resolve_confirm_url(&cfg, uid, &key, None).is_err());
    }

    #[test]
    fn minio_presign_uses_endpoint() {
        let mut cfg = ObjectStorageConfig::dev();
        cfg.minio_enabled = true;
        cfg.endpoint = "http://minio:9000".into();
        cfg.bucket = "anylive".into();
        let uid = UserId::new();
        let p = presign_avatar_put(&cfg, uid, "image/jpeg").unwrap();
        assert!(p.upload_url.starts_with("http://minio:9000/anylive/avatars/"));
        assert!(p.upload_url.contains("X-AnyLive-Signature="));
    }
}
