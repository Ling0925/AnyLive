//! Virtual currency wallet with append-only double-entry ledger.
//!
//! Amounts are integer coin units (never f64). Gift debit + credit are one atomic operation.

use std::collections::HashMap;
use std::sync::Arc;

use anylive_common::{AppError, ErrorCode};
use anylive_domain::UserId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use uuid::Uuid;

/// Append-only ledger entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LedgerEntry {
    pub id: Uuid,
    pub user_id: UserId,
    pub amount: i64,
    /// Positive credit, negative debit from the user's perspective.
    pub balance_after: i64,
    pub entry_type: LedgerType,
    pub reference: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LedgerType {
    Topup,
    GiftDebit,
    GiftCredit,
    Adjustment,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WalletSnapshot {
    pub user_id: UserId,
    pub balance: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GiftCatalogItem {
    pub id: Uuid,
    pub name: String,
    pub price: i64,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GiftOrder {
    pub id: Uuid,
    pub room_id: Uuid,
    pub sender_id: UserId,
    pub receiver_id: UserId,
    pub gift_id: Uuid,
    pub count: u32,
    pub total_coins: i64,
    pub client_request_id: String,
    pub created_at: DateTime<Utc>,
}

/// In-memory wallet + gifts for P1 (replace with Postgres later).
#[derive(Clone, Default)]
pub struct MemoryWallet {
    inner: Arc<Mutex<WalletState>>,
}

#[derive(Default)]
struct WalletState {
    balances: HashMap<Uuid, i64>,
    ledger: Vec<LedgerEntry>,
    gifts: HashMap<Uuid, GiftCatalogItem>,
    /// client_request_id -> order (idempotency)
    gift_orders_by_key: HashMap<String, GiftOrder>,
}

impl MemoryWallet {
    pub fn new() -> Self {
        Self::default()
    }

    /// Seed a couple of catalog gifts for local/dev.
    pub async fn seed_default_gifts(&self) {
        let mut g = self.inner.lock().await;
        if !g.gifts.is_empty() {
            return;
        }
        for (name, price) in [("Rose", 1_i64), ("Rocket", 100), ("Castle", 1000)] {
            let id = Uuid::new_v4();
            g.gifts.insert(
                id,
                GiftCatalogItem {
                    id,
                    name: name.into(),
                    price,
                    active: true,
                },
            );
        }
    }

    pub async fn balance(&self, user_id: UserId) -> i64 {
        let g = self.inner.lock().await;
        *g.balances.get(&user_id.0).unwrap_or(&0)
    }

    pub async fn credit_topup(
        &self,
        user_id: UserId,
        amount: i64,
        reference: impl Into<String>,
    ) -> Result<WalletSnapshot, AppError> {
        if amount <= 0 {
            return Err(AppError::validation("topup amount must be positive"));
        }
        let reference = reference.into();
        let mut g = self.inner.lock().await;
        // Idempotent: same (user_id, reference) returns existing balance snapshot.
        if let Some(existing) = g.ledger.iter().find(|e| {
            e.user_id == user_id
                && e.reference == reference
                && e.entry_type == LedgerType::Topup
        }) {
            return Ok(WalletSnapshot {
                user_id,
                balance: *g.balances.get(&user_id.0).unwrap_or(&existing.balance_after),
            });
        }
        let bal = g.balances.entry(user_id.0).or_insert(0);
        *bal = bal.checked_add(amount).ok_or_else(|| {
            AppError::new(ErrorCode::WalletConflict, "balance overflow")
        })?;
        let balance_after = *bal;
        let entry = LedgerEntry {
            id: Uuid::new_v4(),
            user_id,
            amount,
            balance_after,
            entry_type: LedgerType::Topup,
            reference,
            created_at: Utc::now(),
        };
        g.ledger.push(entry);
        Ok(WalletSnapshot {
            user_id,
            balance: balance_after,
        })
    }

    pub async fn list_gifts(&self) -> Vec<GiftCatalogItem> {
        let g = self.inner.lock().await;
        let mut items: Vec<_> = g.gifts.values().cloned().collect();
        items.sort_by_key(|x| x.price);
        items
    }

    pub async fn upsert_gift(&self, item: GiftCatalogItem) {
        let mut g = self.inner.lock().await;
        g.gifts.insert(item.id, item);
    }

    /// Idempotent gift send. Same `client_request_id` returns original order without double charge.
    pub async fn send_gift(
        &self,
        room_id: Uuid,
        sender: UserId,
        receiver: UserId,
        gift_id: Uuid,
        count: u32,
        client_request_id: impl Into<String>,
    ) -> Result<(GiftOrder, bool), AppError> {
        let client_request_id = client_request_id.into();
        if client_request_id.is_empty() || client_request_id.len() > 128 {
            return Err(AppError::validation("invalid client_request_id"));
        }
        if count == 0 || count > 99 {
            return Err(AppError::validation("count must be 1..=99"));
        }
        if sender.0 == receiver.0 {
            return Err(AppError::validation("cannot gift yourself"));
        }

        // Idempotency is scoped per sender so one user's key cannot replay another's order.
        let idem_key = format!("{}:{}", sender.0, client_request_id);

        let mut g = self.inner.lock().await;
        if let Some(existing) = g.gift_orders_by_key.get(&idem_key) {
            if existing.gift_id != gift_id
                || existing.count != count
                || existing.receiver_id != receiver
                || existing.room_id != room_id
            {
                return Err(AppError::validation(
                    "idempotency key reused with different gift parameters",
                ));
            }
            return Ok((existing.clone(), true));
        }

        let gift = g
            .gifts
            .get(&gift_id)
            .cloned()
            .ok_or_else(|| AppError::not_found("gift not found"))?;
        if !gift.active {
            return Err(AppError::validation("gift inactive"));
        }
        let total = gift
            .price
            .checked_mul(count as i64)
            .ok_or_else(|| AppError::validation("total overflow"))?;

        // Read both balances first so debit+credit stay atomic under the lock
        // (no partial debit if the credit would overflow).
        let sender_before = *g.balances.get(&sender.0).unwrap_or(&0);
        if sender_before < total {
            return Err(AppError::new(
                ErrorCode::GiftInsufficientBalance,
                "insufficient balance",
            ));
        }
        let recv_before = *g.balances.get(&receiver.0).unwrap_or(&0);
        let recv_after = recv_before.checked_add(total).ok_or_else(|| {
            AppError::new(ErrorCode::WalletConflict, "receiver balance overflow")
        })?;
        let sender_after = sender_before - total;

        g.balances.insert(sender.0, sender_after);
        g.balances.insert(receiver.0, recv_after);

        let now = Utc::now();
        let order = GiftOrder {
            id: Uuid::new_v4(),
            room_id,
            sender_id: sender,
            receiver_id: receiver,
            gift_id,
            count,
            total_coins: total,
            client_request_id: client_request_id.clone(),
            created_at: now,
        };

        g.ledger.push(LedgerEntry {
            id: Uuid::new_v4(),
            user_id: sender,
            amount: -total,
            balance_after: sender_after,
            entry_type: LedgerType::GiftDebit,
            reference: order.id.to_string(),
            created_at: now,
        });
        g.ledger.push(LedgerEntry {
            id: Uuid::new_v4(),
            user_id: receiver,
            amount: total,
            balance_after: recv_after,
            entry_type: LedgerType::GiftCredit,
            reference: order.id.to_string(),
            created_at: now,
        });
        g.gift_orders_by_key.insert(idem_key, order.clone());
        Ok((order, false))
    }

    pub async fn ledger_for(&self, user_id: UserId) -> Vec<LedgerEntry> {
        let g = self.inner.lock().await;
        g.ledger
            .iter()
            .filter(|e| e.user_id == user_id)
            .cloned()
            .collect()
    }

    /// Verify each user's balance equals the sum of ledger amounts (P1 dogfood gate).
    pub async fn reconcile(&self) -> ReconcileReport {
        let g = self.inner.lock().await;
        let mut user_ids: std::collections::HashSet<Uuid> = g.balances.keys().copied().collect();
        for e in &g.ledger {
            user_ids.insert(e.user_id.0);
        }
        let mut mismatches = Vec::new();
        let mut checked = 0u64;
        for id in user_ids {
            checked += 1;
            let stored = *g.balances.get(&id).unwrap_or(&0);
            let summed: i64 = g
                .ledger
                .iter()
                .filter(|e| e.user_id.0 == id)
                .map(|e| e.amount)
                .sum();
            if stored != summed {
                mismatches.push(BalanceMismatch {
                    user_id: UserId(id),
                    stored_balance: stored,
                    ledger_sum: summed,
                });
            }
        }
        ReconcileReport {
            checked_users: checked,
            imbalance_count: mismatches.len() as u64,
            mismatches,
        }
    }
}

/// One user whose balance map disagrees with Σ ledger.amount.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BalanceMismatch {
    pub user_id: UserId,
    pub stored_balance: i64,
    pub ledger_sum: i64,
}

/// Result of a wallet ledger/balance consistency scan.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReconcileReport {
    pub checked_users: u64,
    pub imbalance_count: u64,
    pub mismatches: Vec<BalanceMismatch>,
}

impl ReconcileReport {
    pub fn is_balanced(&self) -> bool {
        self.imbalance_count == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn topup_and_gift_idempotent() {
        let w = MemoryWallet::new();
        w.seed_default_gifts().await;
        let gifts = w.list_gifts().await;
        let rose = gifts.iter().find(|g| g.name == "Rose").unwrap().clone();

        let sender = UserId::new();
        let receiver = UserId::new();
        w.credit_topup(sender, 10, "mock-pay").await.unwrap();
        assert_eq!(w.balance(sender).await, 10);

        let room = Uuid::new_v4();
        let (o1, replay1) = w
            .send_gift(room, sender, receiver, rose.id, 3, "req-1")
            .await
            .unwrap();
        assert!(!replay1);
        assert_eq!(o1.total_coins, 3);
        assert_eq!(w.balance(sender).await, 7);
        assert_eq!(w.balance(receiver).await, 3);

        let (o2, replay2) = w
            .send_gift(room, sender, receiver, rose.id, 3, "req-1")
            .await
            .unwrap();
        assert!(replay2);
        assert_eq!(o1.id, o2.id);
        assert_eq!(w.balance(sender).await, 7);
        assert_eq!(w.balance(receiver).await, 3);
    }

    #[tokio::test]
    async fn insufficient_balance() {
        let w = MemoryWallet::new();
        w.seed_default_gifts().await;
        let rocket = w
            .list_gifts()
            .await
            .into_iter()
            .find(|g| g.name == "Rocket")
            .unwrap();
        let err = w
            .send_gift(
                Uuid::new_v4(),
                UserId::new(),
                UserId::new(),
                rocket.id,
                1,
                "x",
            )
            .await
            .unwrap_err();
        assert_eq!(err.code, ErrorCode::GiftInsufficientBalance);
    }

    #[tokio::test]
    async fn ledger_entries_pair_for_gift() {
        let w = MemoryWallet::new();
        w.seed_default_gifts().await;
        let rose = w.list_gifts().await[0].clone();
        let s = UserId::new();
        let r = UserId::new();
        w.credit_topup(s, 5, "t").await.unwrap();
        w.send_gift(Uuid::new_v4(), s, r, rose.id, 2, "k")
            .await
            .unwrap();
        let s_led = w.ledger_for(s).await;
        assert!(s_led.iter().any(|e| e.entry_type == LedgerType::GiftDebit));
        let r_led = w.ledger_for(r).await;
        assert!(r_led
            .iter()
            .any(|e| e.entry_type == LedgerType::GiftCredit));
    }

    #[tokio::test]
    async fn idempotency_key_rejects_param_mismatch() {
        let w = MemoryWallet::new();
        w.seed_default_gifts().await;
        let gifts = w.list_gifts().await;
        let rose = gifts.iter().find(|g| g.name == "Rose").unwrap().clone();
        let rocket = gifts.iter().find(|g| g.name == "Rocket").unwrap().clone();
        let sender = UserId::new();
        let receiver = UserId::new();
        w.credit_topup(sender, 500, "t").await.unwrap();
        let room = Uuid::new_v4();
        w.send_gift(room, sender, receiver, rose.id, 1, "same-key")
            .await
            .unwrap();
        let err = w
            .send_gift(room, sender, receiver, rocket.id, 1, "same-key")
            .await
            .unwrap_err();
        assert_eq!(err.code, ErrorCode::Validation);
        assert_eq!(w.balance(sender).await, 499);
    }

    #[tokio::test]
    async fn idempotency_key_is_scoped_per_sender() {
        let w = MemoryWallet::new();
        w.seed_default_gifts().await;
        let rose = w.list_gifts().await[0].clone();
        let a = UserId::new();
        let b = UserId::new();
        let recv = UserId::new();
        w.credit_topup(a, 10, "t").await.unwrap();
        w.credit_topup(b, 10, "t").await.unwrap();
        let room = Uuid::new_v4();
        let (o1, _) = w
            .send_gift(room, a, recv, rose.id, 1, "shared-key")
            .await
            .unwrap();
        let (o2, replay) = w
            .send_gift(room, b, recv, rose.id, 1, "shared-key")
            .await
            .unwrap();
        assert!(!replay);
        assert_ne!(o1.id, o2.id);
        assert_eq!(w.balance(a).await, 9);
        assert_eq!(w.balance(b).await, 9);
        assert_eq!(w.balance(recv).await, 2);
    }

    #[tokio::test]
    async fn topup_is_idempotent_on_reference() {
        let w = MemoryWallet::new();
        let u = UserId::new();
        let a = w.credit_topup(u, 10, "same-ref").await.unwrap();
        let b = w.credit_topup(u, 10, "same-ref").await.unwrap();
        assert_eq!(a.balance, 10);
        assert_eq!(b.balance, 10);
        assert_eq!(w.balance(u).await, 10);
        let c = w.credit_topup(u, 5, "other-ref").await.unwrap();
        assert_eq!(c.balance, 15);
    }

    #[tokio::test]
    async fn reconcile_reports_balanced_after_gift() {
        let w = MemoryWallet::new();
        w.seed_default_gifts().await;
        let rose = w.list_gifts().await[0].clone();
        let s = UserId::new();
        let r = UserId::new();
        w.credit_topup(s, 10, "t").await.unwrap();
        w.send_gift(Uuid::new_v4(), s, r, rose.id, 3, "k")
            .await
            .unwrap();
        let report = w.reconcile().await;
        assert!(report.is_balanced(), "{report:?}");
        assert!(report.checked_users >= 2);
    }
}
