//! Client analytics event ingest (P4 scaffold). In-memory ring buffer for dogfood.

use std::collections::VecDeque;
use std::sync::Arc;

use anylive_domain::UserId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use uuid::Uuid;

/// Cap retained events in process memory (oldest dropped).
pub const MAX_RETAINED_EVENTS: usize = 10_000;
/// Max events accepted per request.
pub const MAX_BATCH_SIZE: usize = 50;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredClientEvent {
    pub id: Uuid,
    pub user_id: UserId,
    pub name: String,
    pub occurred_at: DateTime<Utc>,
    pub received_at: DateTime<Utc>,
    pub props: serde_json::Value,
    pub client_event_id: Option<String>,
}

#[derive(Clone, Default)]
pub struct AnalyticsStore {
    inner: Arc<RwLock<VecDeque<StoredClientEvent>>>,
    /// client_event_id → server id for simple request-level dedupe in-process.
    seen: Arc<RwLock<std::collections::HashSet<String>>>,
}

impl AnalyticsStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn ingest(
        &self,
        user_id: UserId,
        events: Vec<ClientEventInput>,
    ) -> (u64, u64) {
        let mut accepted = 0u64;
        let mut dropped = 0u64;
        let mut guard = self.inner.write().await;
        let mut seen = self.seen.write().await;
        let now = Utc::now();

        for ev in events.into_iter().take(MAX_BATCH_SIZE) {
            let name = ev.name.trim().to_string();
            if name.is_empty() || name.len() > 128 {
                dropped += 1;
                continue;
            }
            if let Some(cid) = ev.client_event_id.as_ref() {
                let key = format!("{}:{}", user_id.0, cid);
                if !seen.insert(key) {
                    dropped += 1;
                    continue;
                }
            }
            let stored = StoredClientEvent {
                id: Uuid::new_v4(),
                user_id,
                name,
                occurred_at: ev.occurred_at.unwrap_or(now),
                received_at: now,
                props: ev.props.unwrap_or(serde_json::json!({})),
                client_event_id: ev.client_event_id,
            };
            guard.push_back(stored);
            accepted += 1;
            while guard.len() > MAX_RETAINED_EVENTS {
                guard.pop_front();
            }
        }
        // Bound seen set roughly with retained events.
        if seen.len() > MAX_RETAINED_EVENTS {
            seen.clear();
        }
        (accepted, dropped)
    }

    pub async fn count(&self) -> usize {
        self.inner.read().await.len()
    }

    pub async fn recent(&self, limit: usize) -> Vec<StoredClientEvent> {
        let guard = self.inner.read().await;
        guard.iter().rev().take(limit).cloned().collect()
    }

    /// Aggregate retained events by name (dogfood / admin dashboard).
    pub async fn counts_by_name(&self) -> Vec<(String, u64)> {
        use std::collections::HashMap;
        let guard = self.inner.read().await;
        let mut map: HashMap<String, u64> = HashMap::new();
        for ev in guard.iter() {
            *map.entry(ev.name.clone()).or_default() += 1;
        }
        let mut rows: Vec<(String, u64)> = map.into_iter().collect();
        rows.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        rows
    }

    /// Distinct user_ids among retained events (rough dogfood "active" estimate).
    pub async fn distinct_users(&self) -> u64 {
        use std::collections::HashSet;
        let guard = self.inner.read().await;
        let set: HashSet<_> = guard.iter().map(|e| e.user_id).collect();
        set.len() as u64
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ClientEventInput {
    pub name: String,
    pub occurred_at: Option<DateTime<Utc>>,
    pub props: Option<serde_json::Value>,
    pub client_event_id: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn ingest_and_dedupe() {
        let store = AnalyticsStore::new();
        let uid = UserId::new();
        let (a, d) = store
            .ingest(
                uid,
                vec![
                    ClientEventInput {
                        name: "room.view".into(),
                        occurred_at: None,
                        props: Some(serde_json::json!({"room_id": "x"})),
                        client_event_id: Some("e1".into()),
                    },
                    ClientEventInput {
                        name: "room.view".into(),
                        occurred_at: None,
                        props: None,
                        client_event_id: Some("e1".into()),
                    },
                    ClientEventInput {
                        name: "".into(),
                        occurred_at: None,
                        props: None,
                        client_event_id: None,
                    },
                ],
            )
            .await;
        assert_eq!(a, 1);
        assert_eq!(d, 2);
        assert_eq!(store.count().await, 1);
    }

    #[tokio::test]
    async fn counts_by_name_and_distinct_users() {
        let store = AnalyticsStore::new();
        let u1 = UserId::new();
        let u2 = UserId::new();
        let _ = store
            .ingest(
                u1,
                vec![
                    ClientEventInput {
                        name: "room.view".into(),
                        occurred_at: None,
                        props: None,
                        client_event_id: None,
                    },
                    ClientEventInput {
                        name: "gift.tap".into(),
                        occurred_at: None,
                        props: None,
                        client_event_id: None,
                    },
                ],
            )
            .await;
        let _ = store
            .ingest(
                u2,
                vec![ClientEventInput {
                    name: "room.view".into(),
                    occurred_at: None,
                    props: None,
                    client_event_id: None,
                }],
            )
            .await;
        let by = store.counts_by_name().await;
        assert_eq!(by.iter().find(|(n, _)| n == "room.view").map(|(_, c)| *c), Some(2));
        assert_eq!(by.iter().find(|(n, _)| n == "gift.tap").map(|(_, c)| *c), Some(1));
        assert_eq!(store.distinct_users().await, 2);
    }
}
