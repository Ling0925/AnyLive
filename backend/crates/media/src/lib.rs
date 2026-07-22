//! Media control-plane providers (no RTP/RTMP plane in-process).
//!
//! Issues publish credentials and play URLs against an external origin (SRS).
//! Publish stream keys are HMAC-signed so bare room UUIDs cannot push.
//!
//! Play URLs always use the bare room UUID so HLS paths stay stable. Auth for
//! publish lives in the OBS stream-key query string (`?exp=&sig=`).

use std::collections::HashMap;
use std::sync::Arc;

use anylive_common::{AppError, ErrorCode};
use anylive_domain::{RoomId, Timestamp, UserId};
use async_trait::async_trait;
use chrono::{Duration, Utc};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use subtle::ConstantTimeEq;
use tokio::sync::RwLock;
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

/// Default publish credential lifetime.
pub const DEFAULT_PUBLISH_TTL_SECS: i64 = 6 * 60 * 60;

/// Credentials for an owner to push a live stream.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PublishInfo {
    pub push_url: String,
    pub stream_key: String,
    pub expires_at: Timestamp,
}

/// Viewer play endpoints for a room stream.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlayUrls {
    pub hls: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flv: Option<String>,
}

/// Port for media origin control (SRS, Cloudflare Stream, etc.).
#[async_trait]
pub trait MediaProvider: Send + Sync {
    async fn issue_publish(
        &self,
        room_id: RoomId,
        owner: UserId,
    ) -> Result<PublishInfo, AppError>;

    async fn play_urls(&self, room_id: RoomId) -> Result<PlayUrls, AppError>;
}

/// SRS-backed provider using RTMP publish + HTTP-FLV/HLS play.
#[derive(Debug, Clone)]
pub struct SrsMediaProvider {
    rtmp_url: String,
    hls_base: String,
    publish_ttl_secs: i64,
    /// HMAC secret for publish stream tokens. Production must set `SRS_PUBLISH_SECRET`.
    publish_secret: String,
    /// Room → last issued signed stream key (process-local; for diagnostics /
    /// future multi-origin routing). Play URLs always use bare room id.
    active_streams: Arc<RwLock<HashMap<Uuid, String>>>,
}

impl SrsMediaProvider {
    pub fn new(rtmp_url: impl Into<String>, hls_base: impl Into<String>) -> Self {
        Self {
            rtmp_url: trim_trailing_slash(rtmp_url.into()),
            hls_base: trim_trailing_slash(hls_base.into()),
            publish_ttl_secs: DEFAULT_PUBLISH_TTL_SECS,
            publish_secret: std::env::var("SRS_PUBLISH_SECRET")
                .unwrap_or_else(|_| default_publish_secret()),
            active_streams: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Explicit secret (tests / controlled wiring).
    pub fn with_publish_secret(mut self, secret: impl Into<String>) -> Self {
        self.publish_secret = secret.into();
        self
    }

    pub fn with_publish_ttl(mut self, secs: i64) -> Self {
        self.publish_ttl_secs = secs;
        self
    }

    /// Load from `SRS_RTMP_URL` / `SRS_HLS_BASE` / `SRS_PUBLISH_SECRET` with localhost defaults.
    pub fn from_env() -> Self {
        let rtmp = std::env::var("SRS_RTMP_URL")
            .unwrap_or_else(|_| "rtmp://localhost:1935/live".to_string());
        let hls = std::env::var("SRS_HLS_BASE")
            .unwrap_or_else(|_| "http://localhost:8080/live".to_string());
        Self::new(rtmp, hls)
    }

    /// Default public play stream name (bare room id) when no active publish key.
    pub fn play_stream_name(room_id: RoomId) -> String {
        room_id.0.to_string()
    }

    /// OBS stream key: `{room_id}?exp={exp}&sig={sig}`.
    ///
    /// RTMP **stream name** is the bare room UUID (stable HLS path). The HMAC is
    /// carried in the query string so SRS `on_publish` can validate without
    /// putting underscores into the stream name (which breaks default HLS templates).
    pub fn stream_key(&self, room_id: RoomId, _owner: UserId, exp: i64) -> String {
        let room = room_id.0.to_string();
        let sig = self.sign_publish(&room, exp);
        format!("{room}?exp={exp}&sig={sig}")
    }

    fn sign_publish(&self, room: &str, exp: i64) -> String {
        let mut mac = HmacSha256::new_from_slice(self.publish_secret.as_bytes())
            .expect("HMAC key length");
        mac.update(room.as_bytes());
        mac.update(b"|");
        mac.update(exp.to_string().as_bytes());
        let bytes = mac.finalize().into_bytes();
        // Truncate to 16 bytes (32 hex chars) — enough for stream auth.
        hex::encode(&bytes[..16])
    }

    /// Validate an SRS publish callback. Returns the room id on success.
    ///
    /// `stream` is the RTMP stream name (bare room UUID). Auth is in `param`
    /// (SRS query string, e.g. `?exp=…&sig=…`) and/or embedded in a combined
    /// OBS key `uuid?exp=&sig=` when only `stream` is provided.
    pub fn validate_publish_stream(&self, stream: &str) -> Result<RoomId, AppError> {
        self.validate_publish_stream_with_param(stream, "")
    }

    /// Like [`Self::validate_publish_stream`] but with explicit SRS `param`.
    pub fn validate_publish_stream_with_param(
        &self,
        stream: &str,
        param: &str,
    ) -> Result<RoomId, AppError> {
        let raw = stream.trim().trim_start_matches('/');
        // Combined OBS key form: uuid?exp=&sig=
        let (stream_part, embedded_q) = if let Some(i) = raw.find('?') {
            (&raw[..i], &raw[i..])
        } else {
            (raw, "")
        };
        let base = stream_part.split('.').next().unwrap_or(stream_part);
        let room_uuid = Uuid::parse_str(base).map_err(|_| {
            AppError::new(ErrorCode::Forbidden, "invalid publish stream room id")
        })?;
        let room_s = room_uuid.to_string();

        let query = if !param.trim().is_empty() {
            param.trim()
        } else {
            embedded_q
        };
        let q = query.trim().trim_start_matches('?');
        if q.is_empty() {
            return Err(AppError::new(
                ErrorCode::Forbidden,
                "publish requires exp+sig query (bare room id rejected)",
            ));
        }
        let mut exp: Option<i64> = None;
        let mut sig: Option<&str> = None;
        for pair in q.split('&') {
            let mut it = pair.splitn(2, '=');
            let k = it.next().unwrap_or("");
            let v = it.next().unwrap_or("");
            match k {
                "exp" => exp = v.parse().ok(),
                "sig" => sig = Some(v),
                _ => {}
            }
        }
        let exp = exp.ok_or_else(|| {
            AppError::new(ErrorCode::Forbidden, "missing publish exp")
        })?;
        let sig = sig.ok_or_else(|| {
            AppError::new(ErrorCode::Forbidden, "missing publish sig")
        })?;
        if exp < Utc::now().timestamp() {
            return Err(AppError::new(
                ErrorCode::Forbidden,
                "publish stream key expired",
            ));
        }
        let expected = self.sign_publish(&room_s, exp);
        if !constant_time_eq(sig, &expected) {
            return Err(AppError::new(
                ErrorCode::Forbidden,
                "invalid publish stream signature",
            ));
        }
        Ok(RoomId(room_uuid))
    }

    pub fn build_publish(&self, room_id: RoomId, owner: UserId) -> PublishInfo {
        let expires_at = Utc::now() + Duration::seconds(self.publish_ttl_secs);
        let exp = expires_at.timestamp();
        let stream_key = self.stream_key(room_id, owner, exp);
        // Full RTMP URL includes stream name for OBS convenience.
        let push_url = format!("{}/{}", self.rtmp_url, stream_key);
        PublishInfo {
            push_url,
            stream_key,
            expires_at,
        }
    }

    /// Remember the signed stream name so play URLs match OBS/SRS HLS path.
    pub async fn remember_active_stream(&self, room_id: RoomId, stream_key: impl Into<String>) {
        self.active_streams
            .write()
            .await
            .insert(room_id.0, stream_key.into());
    }

    /// Drop active stream mapping (room stop / unpublish / force-close).
    pub async fn clear_active_stream(&self, room_id: RoomId) {
        self.active_streams.write().await.remove(&room_id.0);
    }

    /// Current active stream name for a room, if any.
    pub async fn active_stream_key(&self, room_id: RoomId) -> Option<String> {
        self.active_streams.read().await.get(&room_id.0).cloned()
    }

    /// Build play URLs. HLS/FLV always use the bare room id stream name so
    /// paths stay stable and match SRS default HLS templates.
    pub async fn build_play(&self, room_id: RoomId) -> PlayUrls {
        let stream = Self::play_stream_name(room_id);
        let _ = self.active_stream_key(room_id).await; // keep API hot path simple
        PlayUrls {
            hls: format!("{}/{}.m3u8", self.hls_base, stream),
            flv: Some(format!("{}/{}.flv", self.hls_base, stream)),
        }
    }
}

#[async_trait]
impl MediaProvider for SrsMediaProvider {
    async fn issue_publish(
        &self,
        room_id: RoomId,
        owner: UserId,
    ) -> Result<PublishInfo, AppError> {
        let info = self.build_publish(room_id, owner);
        self.remember_active_stream(room_id, info.stream_key.clone())
            .await;
        Ok(info)
    }

    async fn play_urls(&self, room_id: RoomId) -> Result<PlayUrls, AppError> {
        Ok(self.build_play(room_id).await)
    }
}

fn trim_trailing_slash(s: String) -> String {
    s.trim_end_matches('/').to_string()
}

fn default_publish_secret() -> String {
    // Distinct from JWT defaults; still only for local. Production should set env.
    "anylive-srs-publish-secret-change-me!!".into()
}

fn constant_time_eq(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    bool::from(a.as_bytes().ct_eq(b.as_bytes()))
}

/// Map media failures to the stable API code.
#[allow(dead_code)]
pub fn media_error(message: impl Into<String>) -> AppError {
    AppError::new(ErrorCode::MediaProviderError, message)
}

/// Parse room id from a stream name that may be bare UUID or signed key.
/// Used only for unpublish / play-side helpers — publish validation is stricter.
pub fn parse_room_from_stream(stream: &str) -> Option<RoomId> {
    let name = stream.trim().trim_start_matches('/');
    // Strip query (`uuid?exp=&sig=`) and extension (`.flv` / `.m3u8`).
    let no_query = name.split('?').next().unwrap_or(name);
    let base = no_query.split('.').next().unwrap_or(no_query);
    // Legacy signed form: uuid_exp_sig
    if let Some(room_part) = base.split('_').next() {
        if let Ok(u) = Uuid::parse_str(room_part) {
            return Some(RoomId(u));
        }
    }
    Uuid::parse_str(base).ok().map(RoomId)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixed_ids() -> (RoomId, UserId) {
        let room = RoomId(Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap());
        let owner = UserId(Uuid::parse_str("22222222-2222-2222-2222-222222222222").unwrap());
        (room, owner)
    }

    fn provider() -> SrsMediaProvider {
        SrsMediaProvider::new("rtmp://localhost:1935/live", "http://localhost:8080/live")
            .with_publish_secret("test-publish-secret-32-bytes-long!!")
            .with_publish_ttl(3600)
    }

    #[test]
    fn stream_key_embeds_query_sig() {
        let (room, owner) = fixed_ids();
        let p = provider();
        let exp = Utc::now().timestamp() + 3600;
        let key = p.stream_key(room, owner, exp);
        assert!(key.starts_with(&format!("{}?", room.0)));
        assert!(key.contains("exp="));
        assert!(key.contains("sig="));
        assert_ne!(key, room.0.to_string());
    }

    #[test]
    fn publish_url_construction() {
        let p = provider();
        let (room, owner) = fixed_ids();
        let info = p.build_publish(room, owner);
        assert!(info.push_url.starts_with("rtmp://localhost:1935/live/"));
        assert!(info.stream_key.starts_with("11111111-1111-1111-1111-111111111111?"));
        assert!(info.stream_key.contains("sig="));
        assert_ne!(info.stream_key, "11111111-1111-1111-1111-111111111111");
        let delta = info.expires_at - Utc::now();
        assert!(delta.num_seconds() > 3500 && delta.num_seconds() <= 3600);
    }

    #[test]
    fn validate_accepts_issued_key() {
        let p = provider();
        let (room, owner) = fixed_ids();
        let info = p.build_publish(room, owner);
        // Full OBS key with query
        let parsed = p.validate_publish_stream(&info.stream_key).unwrap();
        assert_eq!(parsed, room);
        // SRS style: stream=uuid, param=?exp&sig
        let q = info.stream_key.split_once('?').unwrap().1;
        let parsed2 = p
            .validate_publish_stream_with_param(&room.0.to_string(), &format!("?{q}"))
            .unwrap();
        assert_eq!(parsed2, room);
    }

    #[test]
    fn validate_rejects_bare_uuid() {
        let p = provider();
        let (room, _) = fixed_ids();
        let err = p
            .validate_publish_stream(&room.0.to_string())
            .unwrap_err();
        assert_eq!(err.code, ErrorCode::Forbidden);
    }

    #[test]
    fn validate_rejects_tampered_sig() {
        let p = provider();
        let (room, owner) = fixed_ids();
        let info = p.build_publish(room, owner);
        let bad = info.stream_key.replace("sig=", "sig=00");
        assert!(p.validate_publish_stream(&bad).is_err());
    }

    #[tokio::test]
    async fn play_urls_without_publish_use_bare_room() {
        let p = provider();
        let (room, _) = fixed_ids();
        let urls = p.build_play(room).await;
        assert_eq!(
            urls.hls,
            "http://localhost:8080/live/11111111-1111-1111-1111-111111111111.m3u8"
        );
        assert_eq!(
            urls.flv.as_deref(),
            Some("http://localhost:8080/live/11111111-1111-1111-1111-111111111111.flv")
        );
    }

    #[tokio::test]
    async fn play_urls_always_bare_room_id() {
        let p = provider();
        let (room, owner) = fixed_ids();
        let _info = p.issue_publish(room, owner).await.unwrap();
        let urls = p.play_urls(room).await.unwrap();
        assert_eq!(
            urls.hls,
            format!("http://localhost:8080/live/{}.m3u8", room.0)
        );
        assert!(urls.hls.ends_with(".m3u8"));
    }

    #[test]
    fn parse_room_from_signed_or_bare() {
        let id = Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap();
        assert_eq!(parse_room_from_stream(&id.to_string()).unwrap().0, id);
        assert_eq!(
            parse_room_from_stream(&format!("{id}_999_abc")).unwrap().0,
            id
        );
        assert_eq!(
            parse_room_from_stream(&format!("{id}?exp=1&sig=abc")).unwrap().0,
            id
        );
        assert_eq!(
            parse_room_from_stream(&format!("{id}.flv")).unwrap().0,
            id
        );
        assert!(parse_room_from_stream("not-a-uuid").is_none());
    }

    #[tokio::test]
    async fn trait_issue_publish_and_play() {
        let p = provider();
        let (room, owner) = fixed_ids();
        let info = p.issue_publish(room, owner).await.unwrap();
        assert!(info.push_url.contains(&room.0.to_string()));
        assert!(p.validate_publish_stream(&info.stream_key).is_ok());
        let play = p.play_urls(room).await.unwrap();
        assert!(play.hls.ends_with(".m3u8"));
        assert!(play.hls.contains(&room.0.to_string()));
        assert!(!play.hls.contains('?'));
    }
}
