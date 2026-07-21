//! Media control-plane providers (no RTP/RTMP plane in-process).
//!
//! Issues publish credentials and play URLs against an external origin (SRS).

use anylive_common::{AppError, ErrorCode};
use anylive_domain::{RoomId, Timestamp, UserId};
use async_trait::async_trait;
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};

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
}

impl SrsMediaProvider {
    pub fn new(rtmp_url: impl Into<String>, hls_base: impl Into<String>) -> Self {
        Self {
            rtmp_url: trim_trailing_slash(rtmp_url.into()),
            hls_base: trim_trailing_slash(hls_base.into()),
            publish_ttl_secs: DEFAULT_PUBLISH_TTL_SECS,
        }
    }

    pub fn with_publish_ttl(mut self, secs: i64) -> Self {
        self.publish_ttl_secs = secs;
        self
    }

    /// Load from `SRS_RTMP_URL` / `SRS_HLS_BASE` with localhost defaults.
    pub fn from_env() -> Self {
        let rtmp = std::env::var("SRS_RTMP_URL")
            .unwrap_or_else(|_| "rtmp://localhost:1935/live".to_string());
        let hls = std::env::var("SRS_HLS_BASE")
            .unwrap_or_else(|_| "http://localhost:8080/live".to_string());
        Self::new(rtmp, hls)
    }

    /// Stream name aligned with play path (room id). Owner is authz-only, not part of key.
    pub fn stream_key(room_id: RoomId, _owner: UserId) -> String {
        room_id.0.to_string()
    }

    pub fn build_publish(&self, room_id: RoomId, owner: UserId) -> PublishInfo {
        let stream_key = Self::stream_key(room_id, owner);
        let expires_at = Utc::now() + Duration::seconds(self.publish_ttl_secs);
        // Full RTMP URL includes stream name for OBS convenience.
        let push_url = format!("{}/{}", self.rtmp_url, stream_key);
        PublishInfo {
            push_url,
            stream_key,
            expires_at,
        }
    }

    pub fn build_play(&self, room_id: RoomId) -> PlayUrls {
        let id = room_id.0.to_string();
        PlayUrls {
            hls: format!("{}/{}.m3u8", self.hls_base, id),
            flv: Some(format!("{}/{}.flv", self.hls_base, id)),
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
        Ok(self.build_publish(room_id, owner))
    }

    async fn play_urls(&self, room_id: RoomId) -> Result<PlayUrls, AppError> {
        Ok(self.build_play(room_id))
    }
}

fn trim_trailing_slash(s: String) -> String {
    s.trim_end_matches('/').to_string()
}

/// Map media failures to the stable API code.
#[allow(dead_code)]
pub fn media_error(message: impl Into<String>) -> AppError {
    AppError::new(ErrorCode::MediaProviderError, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn fixed_ids() -> (RoomId, UserId) {
        let room = RoomId(Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap());
        let owner = UserId(Uuid::parse_str("22222222-2222-2222-2222-222222222222").unwrap());
        (room, owner)
    }

    #[test]
    fn stream_key_format() {
        let (room, owner) = fixed_ids();
        assert_eq!(
            SrsMediaProvider::stream_key(room, owner),
            "11111111-1111-1111-1111-111111111111"
        );
    }

    #[test]
    fn publish_url_construction() {
        let provider = SrsMediaProvider::new(
            "rtmp://localhost:1935/live/",
            "http://localhost:8080/live/",
        )
        .with_publish_ttl(3600);
        let (room, owner) = fixed_ids();
        let info = provider.build_publish(room, owner);
        assert_eq!(
            info.push_url,
            "rtmp://localhost:1935/live/11111111-1111-1111-1111-111111111111"
        );
        assert_eq!(info.stream_key, "11111111-1111-1111-1111-111111111111");
        let delta = info.expires_at - Utc::now();
        assert!(delta.num_seconds() > 3500 && delta.num_seconds() <= 3600);
    }

    #[test]
    fn play_urls_construction() {
        let provider =
            SrsMediaProvider::new("rtmp://localhost:1935/live", "http://cdn.example/live");
        let (room, _) = fixed_ids();
        let urls = provider.build_play(room);
        assert_eq!(
            urls.hls,
            "http://cdn.example/live/11111111-1111-1111-1111-111111111111.m3u8"
        );
        assert_eq!(
            urls.flv.as_deref(),
            Some("http://cdn.example/live/11111111-1111-1111-1111-111111111111.flv")
        );
    }

    #[test]
    fn from_env_defaults() {
        // Ensure defaults parse without env override in test process.
        std::env::remove_var("SRS_RTMP_URL");
        std::env::remove_var("SRS_HLS_BASE");
        let p = SrsMediaProvider::from_env();
        let (room, owner) = fixed_ids();
        let info = p.build_publish(room, owner);
        assert!(info.push_url.starts_with("rtmp://localhost:1935/live/"));
        let play = p.build_play(room);
        assert!(play.hls.starts_with("http://localhost:8080/live/"));
    }

    #[tokio::test]
    async fn trait_issue_publish_and_play() {
        let p = SrsMediaProvider::new("rtmp://o/live", "http://h/live");
        let (room, owner) = fixed_ids();
        let info = p.issue_publish(room, owner).await.unwrap();
        assert_eq!(
            info.push_url,
            "rtmp://o/live/11111111-1111-1111-1111-111111111111"
        );
        let play = p.play_urls(room).await.unwrap();
        assert!(play.hls.ends_with(".m3u8"));
    }
}
