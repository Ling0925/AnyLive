//! Password credential storage (argon2 PHC hashes).

use std::collections::HashMap;
use std::sync::Arc;

use anylive_common::AppError;
use anylive_domain::UserId;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct CredentialRecord {
    pub user_id: UserId,
    pub password_hash: String,
    pub password_updated_at: DateTime<Utc>,
    pub must_change_password: bool,
    pub failed_attempts: u32,
    pub locked_until: Option<DateTime<Utc>>,
}

#[async_trait]
pub trait CredentialStore: Send + Sync + Clone {
    async fn get(&self, user_id: UserId) -> Result<Option<CredentialRecord>, AppError>;
    async fn upsert(&self, record: CredentialRecord) -> Result<(), AppError>;
    async fn delete(&self, user_id: UserId) -> Result<(), AppError>;
}

#[derive(Clone, Default)]
pub struct InMemoryCredentialStore {
    inner: Arc<RwLock<HashMap<uuid::Uuid, CredentialRecord>>>,
}

#[async_trait]
impl CredentialStore for InMemoryCredentialStore {
    async fn get(&self, user_id: UserId) -> Result<Option<CredentialRecord>, AppError> {
        Ok(self.inner.read().await.get(&user_id.0).cloned())
    }

    async fn upsert(&self, record: CredentialRecord) -> Result<(), AppError> {
        self.inner
            .write()
            .await
            .insert(record.user_id.0, record);
        Ok(())
    }

    async fn delete(&self, user_id: UserId) -> Result<(), AppError> {
        self.inner.write().await.remove(&user_id.0);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::password::hash_password;

    #[tokio::test]
    async fn memory_credential_roundtrip() {
        let store = InMemoryCredentialStore::default();
        let uid = UserId::new();
        let hash = hash_password("secretpass").unwrap();
        store
            .upsert(CredentialRecord {
                user_id: uid,
                password_hash: hash.clone(),
                password_updated_at: Utc::now(),
                must_change_password: true,
                failed_attempts: 0,
                locked_until: None,
            })
            .await
            .unwrap();
        let got = store.get(uid).await.unwrap().unwrap();
        assert_eq!(got.password_hash, hash);
        assert!(got.must_change_password);
    }
}
