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
        let mut g = self.inner.lock().await;
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
            reference: reference.into(),
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

        let mut g = self.inner.lock().await;
        if let Some(existing) = g.gift_orders_by_key.get(&client_request_id) {
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

        let sender_bal = g.balances.entry(sender.0).or_insert(0);
        if *sender_bal < total {
            return Err(AppError::new(
                ErrorCode::GiftInsufficientBalance,
                "insufficient balance",
            ));
        }
        *sender_bal -= total;
        let sender_after = *sender_bal;

        let recv_bal = g.balances.entry(receiver.0).or_insert(0);
        *recv_bal = recv_bal.checked_add(total).ok_or_else(|| {
            AppError::new(ErrorCode::WalletConflict, "receiver balance overflow")
        })?;
        let recv_after = *recv_bal;

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
        g.gift_orders_by_key
            .insert(client_request_id, order.clone());
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
}
