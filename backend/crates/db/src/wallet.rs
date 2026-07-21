//! [`PostgresWallet`] + dual [`AnyWallet`] matching [`MemoryWallet`] API surface.
//!
//! Gift send is transactional and idempotent on `UNIQUE (sender_id, client_request_id)`.

use anylive_common::{AppError, ErrorCode};
use anylive_domain::UserId;
use anylive_wallet::{
    GiftCatalogItem, GiftOrder, LedgerEntry, LedgerType, MemoryWallet, WalletSnapshot,
};
use chrono::{DateTime, Utc};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

/// Postgres-backed wallet, ledger, gifts, and gift orders.
#[derive(Clone)]
pub struct PostgresWallet {
    pool: PgPool,
}

impl PostgresWallet {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Seed catalog gifts when the table is empty (dev / first boot).
    pub async fn seed_default_gifts(&self) {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM gifts")
            .fetch_one(&self.pool)
            .await
            .unwrap_or(0);
        if count > 0 {
            return;
        }
        for (name, price) in [("Rose", 1_i64), ("Rocket", 100), ("Castle", 1000)] {
            let id = Uuid::new_v4();
            let _ = sqlx::query(
                r#"
                INSERT INTO gifts (id, name, price, active)
                VALUES ($1, $2, $3, true)
                ON CONFLICT DO NOTHING
                "#,
            )
            .bind(id)
            .bind(name)
            .bind(price)
            .execute(&self.pool)
            .await;
        }
    }

    pub async fn balance(&self, user_id: UserId) -> i64 {
        let bal: Option<i64> = sqlx::query_scalar(
            r#"
            SELECT balance FROM wallet_balances WHERE user_id = $1
            "#,
        )
        .bind(user_id.0)
        .fetch_optional(&self.pool)
        .await
        .ok()
        .flatten();
        bal.unwrap_or(0)
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
        let mut tx = self.pool.begin().await.map_err(map_db)?;

        ensure_balance_row(&mut tx, user_id).await?;

        let bal: i64 = sqlx::query_scalar(
            r#"
            SELECT balance FROM wallet_balances WHERE user_id = $1 FOR UPDATE
            "#,
        )
        .bind(user_id.0)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db)?;

        let balance_after = bal.checked_add(amount).ok_or_else(|| {
            AppError::new(ErrorCode::WalletConflict, "balance overflow")
        })?;

        sqlx::query(
            r#"
            UPDATE wallet_balances SET balance = $2 WHERE user_id = $1
            "#,
        )
        .bind(user_id.0)
        .bind(balance_after)
        .execute(&mut *tx)
        .await
        .map_err(map_db)?;

        sqlx::query(
            r#"
            INSERT INTO wallet_ledger (user_id, amount, balance_after, entry_type, reference)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(user_id.0)
        .bind(amount)
        .bind(balance_after)
        .bind(ledger_type_str(LedgerType::Topup))
        .bind(&reference)
        .execute(&mut *tx)
        .await
        .map_err(map_db)?;

        tx.commit().await.map_err(map_db)?;
        Ok(WalletSnapshot {
            user_id,
            balance: balance_after,
        })
    }

    pub async fn list_gifts(&self) -> Vec<GiftCatalogItem> {
        let rows = sqlx::query_as::<_, GiftRow>(
            r#"
            SELECT id, name, price, active FROM gifts ORDER BY price ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .unwrap_or_else(|err| {
            tracing::error!(error = %err, "postgres list_gifts failed");
            Vec::new()
        });
        rows.into_iter().map(GiftRow::into_item).collect()
    }

    pub async fn upsert_gift(&self, item: GiftCatalogItem) {
        let _ = sqlx::query(
            r#"
            INSERT INTO gifts (id, name, price, active)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (id) DO UPDATE
                SET name = EXCLUDED.name,
                    price = EXCLUDED.price,
                    active = EXCLUDED.active
            "#,
        )
        .bind(item.id)
        .bind(&item.name)
        .bind(item.price)
        .bind(item.active)
        .execute(&self.pool)
        .await;
    }

    /// Idempotent gift send. Same `(sender_id, client_request_id)` returns original order.
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

        let mut tx = self.pool.begin().await.map_err(map_db)?;

        // Idempotent replay first.
        if let Some(existing) =
            fetch_order_by_idem(&mut tx, sender, &client_request_id).await?
        {
            tx.commit().await.map_err(map_db)?;
            return Ok((existing, true));
        }

        let gift = sqlx::query_as::<_, GiftRow>(
            r#"
            SELECT id, name, price, active FROM gifts WHERE id = $1
            "#,
        )
        .bind(gift_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db)?
        .ok_or_else(|| AppError::not_found("gift not found"))?;

        if !gift.active {
            return Err(AppError::validation("gift inactive"));
        }
        let total = gift
            .price
            .checked_mul(count as i64)
            .ok_or_else(|| AppError::validation("total overflow"))?;

        ensure_balance_row(&mut tx, sender).await?;
        ensure_balance_row(&mut tx, receiver).await?;

        // Lock both balance rows in stable UUID order to avoid deadlocks.
        let (first, second) = if sender.0 <= receiver.0 {
            (sender, receiver)
        } else {
            (receiver, sender)
        };
        let _ = sqlx::query_scalar::<_, i64>(
            "SELECT balance FROM wallet_balances WHERE user_id = $1 FOR UPDATE",
        )
        .bind(first.0)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db)?;
        let _ = sqlx::query_scalar::<_, i64>(
            "SELECT balance FROM wallet_balances WHERE user_id = $1 FOR UPDATE",
        )
        .bind(second.0)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db)?;

        let sender_before: i64 = sqlx::query_scalar(
            "SELECT balance FROM wallet_balances WHERE user_id = $1",
        )
        .bind(sender.0)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db)?;

        if sender_before < total {
            return Err(AppError::new(
                ErrorCode::GiftInsufficientBalance,
                "insufficient balance",
            ));
        }

        let recv_before: i64 = sqlx::query_scalar(
            "SELECT balance FROM wallet_balances WHERE user_id = $1",
        )
        .bind(receiver.0)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db)?;

        let recv_after = recv_before.checked_add(total).ok_or_else(|| {
            AppError::new(ErrorCode::WalletConflict, "receiver balance overflow")
        })?;
        let sender_after = sender_before - total;

        sqlx::query("UPDATE wallet_balances SET balance = $2 WHERE user_id = $1")
            .bind(sender.0)
            .bind(sender_after)
            .execute(&mut *tx)
            .await
            .map_err(map_db)?;
        sqlx::query("UPDATE wallet_balances SET balance = $2 WHERE user_id = $1")
            .bind(receiver.0)
            .bind(recv_after)
            .execute(&mut *tx)
            .await
            .map_err(map_db)?;

        let order_id = Uuid::new_v4();
        let now = Utc::now();

        let order_row = sqlx::query_as::<_, GiftOrderRow>(
            r#"
            INSERT INTO gift_orders (
                id, room_id, sender_id, receiver_id, gift_id,
                count, total_coins, client_request_id, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING id, room_id, sender_id, receiver_id, gift_id,
                      count, total_coins, client_request_id, created_at
            "#,
        )
        .bind(order_id)
        .bind(room_id)
        .bind(sender.0)
        .bind(receiver.0)
        .bind(gift_id)
        .bind(count as i32)
        .bind(total)
        .bind(&client_request_id)
        .bind(now)
        .fetch_one(&mut *tx)
        .await;

        let order_row = match order_row {
            Ok(r) => r,
            Err(sqlx::Error::Database(ref db))
                if db.constraint() == Some("gift_orders_sender_id_client_request_id_key") =>
            {
                // Concurrent insert won the race — return the winner as replay.
                let existing = fetch_order_by_idem(&mut tx, sender, &client_request_id)
                    .await?
                    .ok_or_else(|| {
                        AppError::new(ErrorCode::WalletConflict, "idempotent race unresolved")
                    })?;
                tx.commit().await.map_err(map_db)?;
                return Ok((existing, true));
            }
            Err(e) => return Err(map_db(e)),
        };

        sqlx::query(
            r#"
            INSERT INTO wallet_ledger (user_id, amount, balance_after, entry_type, reference)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(sender.0)
        .bind(-total)
        .bind(sender_after)
        .bind(ledger_type_str(LedgerType::GiftDebit))
        .bind(order_id.to_string())
        .execute(&mut *tx)
        .await
        .map_err(map_db)?;

        sqlx::query(
            r#"
            INSERT INTO wallet_ledger (user_id, amount, balance_after, entry_type, reference)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(receiver.0)
        .bind(total)
        .bind(recv_after)
        .bind(ledger_type_str(LedgerType::GiftCredit))
        .bind(order_id.to_string())
        .execute(&mut *tx)
        .await
        .map_err(map_db)?;

        tx.commit().await.map_err(map_db)?;
        Ok((order_row.into_order(), false))
    }

    pub async fn ledger_for(&self, user_id: UserId) -> Vec<LedgerEntry> {
        let rows = sqlx::query_as::<_, LedgerRow>(
            r#"
            SELECT id, user_id, amount, balance_after, entry_type, reference, created_at
            FROM wallet_ledger
            WHERE user_id = $1
            ORDER BY created_at ASC
            "#,
        )
        .bind(user_id.0)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();
        rows.into_iter()
            .filter_map(|r| r.into_entry().ok())
            .collect()
    }
}

/// Dual backend so the API can switch memory ↔ Postgres without generics on `AppState`.
#[derive(Clone)]
pub enum AnyWallet {
    Memory(MemoryWallet),
    Postgres(PostgresWallet),
}

impl AnyWallet {
    pub fn memory() -> Self {
        Self::Memory(MemoryWallet::new())
    }

    pub fn postgres(pool: PgPool) -> Self {
        Self::Postgres(PostgresWallet::new(pool))
    }

    pub fn is_postgres(&self) -> bool {
        matches!(self, Self::Postgres(_))
    }

    pub async fn seed_default_gifts(&self) {
        match self {
            Self::Memory(w) => w.seed_default_gifts().await,
            Self::Postgres(w) => w.seed_default_gifts().await,
        }
    }

    pub async fn balance(&self, user_id: UserId) -> i64 {
        match self {
            Self::Memory(w) => w.balance(user_id).await,
            Self::Postgres(w) => w.balance(user_id).await,
        }
    }

    pub async fn credit_topup(
        &self,
        user_id: UserId,
        amount: i64,
        reference: impl Into<String>,
    ) -> Result<WalletSnapshot, AppError> {
        match self {
            Self::Memory(w) => w.credit_topup(user_id, amount, reference).await,
            Self::Postgres(w) => w.credit_topup(user_id, amount, reference).await,
        }
    }

    pub async fn list_gifts(&self) -> Vec<GiftCatalogItem> {
        match self {
            Self::Memory(w) => w.list_gifts().await,
            Self::Postgres(w) => w.list_gifts().await,
        }
    }

    pub async fn upsert_gift(&self, item: GiftCatalogItem) {
        match self {
            Self::Memory(w) => w.upsert_gift(item).await,
            Self::Postgres(w) => w.upsert_gift(item).await,
        }
    }

    pub async fn send_gift(
        &self,
        room_id: Uuid,
        sender: UserId,
        receiver: UserId,
        gift_id: Uuid,
        count: u32,
        client_request_id: impl Into<String>,
    ) -> Result<(GiftOrder, bool), AppError> {
        match self {
            Self::Memory(w) => {
                w.send_gift(room_id, sender, receiver, gift_id, count, client_request_id)
                    .await
            }
            Self::Postgres(w) => {
                w.send_gift(room_id, sender, receiver, gift_id, count, client_request_id)
                    .await
            }
        }
    }

    pub async fn ledger_for(&self, user_id: UserId) -> Vec<LedgerEntry> {
        match self {
            Self::Memory(w) => w.ledger_for(user_id).await,
            Self::Postgres(w) => w.ledger_for(user_id).await,
        }
    }
}

async fn ensure_balance_row(
    tx: &mut Transaction<'_, Postgres>,
    user_id: UserId,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO wallet_balances (user_id, balance)
        VALUES ($1, 0)
        ON CONFLICT (user_id) DO NOTHING
        "#,
    )
    .bind(user_id.0)
    .execute(&mut **tx)
    .await
    .map_err(map_db)?;
    Ok(())
}

async fn fetch_order_by_idem(
    tx: &mut Transaction<'_, Postgres>,
    sender: UserId,
    client_request_id: &str,
) -> Result<Option<GiftOrder>, AppError> {
    let row = sqlx::query_as::<_, GiftOrderRow>(
        r#"
        SELECT id, room_id, sender_id, receiver_id, gift_id,
               count, total_coins, client_request_id, created_at
        FROM gift_orders
        WHERE sender_id = $1 AND client_request_id = $2
        "#,
    )
    .bind(sender.0)
    .bind(client_request_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db)?;
    Ok(row.map(GiftOrderRow::into_order))
}

fn map_db(err: sqlx::Error) -> AppError {
    tracing::error!(error = %err, "postgres wallet error");
    if let sqlx::Error::Database(db) = &err {
        // FK on users for balances / orders.
        if db.constraint().is_some_and(|c| c.ends_with("_fkey")) {
            return AppError::validation("referenced user or gift does not exist");
        }
    }
    AppError::new(ErrorCode::Internal, "database error")
}

fn ledger_type_str(t: LedgerType) -> &'static str {
    match t {
        LedgerType::Topup => "topup",
        LedgerType::GiftDebit => "gift_debit",
        LedgerType::GiftCredit => "gift_credit",
        LedgerType::Adjustment => "adjustment",
    }
}

fn parse_ledger_type(s: &str) -> Option<LedgerType> {
    match s {
        "topup" => Some(LedgerType::Topup),
        "gift_debit" => Some(LedgerType::GiftDebit),
        "gift_credit" => Some(LedgerType::GiftCredit),
        "adjustment" => Some(LedgerType::Adjustment),
        _ => None,
    }
}

#[derive(Debug, sqlx::FromRow)]
struct GiftRow {
    id: Uuid,
    name: String,
    price: i64,
    active: bool,
}

impl GiftRow {
    fn into_item(self) -> GiftCatalogItem {
        GiftCatalogItem {
            id: self.id,
            name: self.name,
            price: self.price,
            active: self.active,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct GiftOrderRow {
    id: Uuid,
    room_id: Uuid,
    sender_id: Uuid,
    receiver_id: Uuid,
    gift_id: Uuid,
    count: i32,
    total_coins: i64,
    client_request_id: String,
    created_at: DateTime<Utc>,
}

impl GiftOrderRow {
    fn into_order(self) -> GiftOrder {
        GiftOrder {
            id: self.id,
            room_id: self.room_id,
            sender_id: UserId(self.sender_id),
            receiver_id: UserId(self.receiver_id),
            gift_id: self.gift_id,
            count: self.count as u32,
            total_coins: self.total_coins,
            client_request_id: self.client_request_id,
            created_at: self.created_at,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct LedgerRow {
    id: Uuid,
    user_id: Uuid,
    amount: i64,
    balance_after: i64,
    entry_type: String,
    reference: String,
    created_at: DateTime<Utc>,
}

impl LedgerRow {
    fn into_entry(self) -> Result<LedgerEntry, AppError> {
        let entry_type = parse_ledger_type(&self.entry_type).ok_or_else(|| {
            AppError::new(
                ErrorCode::Internal,
                format!("invalid ledger type: {}", self.entry_type),
            )
        })?;
        Ok(LedgerEntry {
            id: self.id,
            user_id: UserId(self.user_id),
            amount: self.amount,
            balance_after: self.balance_after,
            entry_type,
            reference: self.reference,
            created_at: self.created_at,
        })
    }
}

/// Pure SQL fragments for offline unit tests / documentation.
#[allow(dead_code)]
pub mod sql {
    pub const UPSERT_BALANCE_ZERO: &str = r#"
        INSERT INTO wallet_balances (user_id, balance)
        VALUES ($1, 0)
        ON CONFLICT (user_id) DO NOTHING
        "#;

    pub const LOCK_BALANCE: &str =
        "SELECT balance FROM wallet_balances WHERE user_id = $1 FOR UPDATE";

    pub const UPDATE_BALANCE: &str =
        "UPDATE wallet_balances SET balance = $2 WHERE user_id = $1";

    pub const INSERT_LEDGER: &str = r#"
            INSERT INTO wallet_ledger (user_id, amount, balance_after, entry_type, reference)
            VALUES ($1, $2, $3, $4, $5)
            "#;

    pub const SELECT_ORDER_BY_IDEMPOTENCY: &str = r#"
        SELECT id, room_id, sender_id, receiver_id, gift_id,
               count, total_coins, client_request_id, created_at
        FROM gift_orders
        WHERE sender_id = $1 AND client_request_id = $2
        "#;

    pub const INSERT_GIFT_ORDER: &str = r#"
            INSERT INTO gift_orders (
                id, room_id, sender_id, receiver_id, gift_id,
                count, total_coins, client_request_id, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING id, room_id, sender_id, receiver_id, gift_id,
                      count, total_coins, client_request_id, created_at
            "#;

    pub const UPSERT_GIFT: &str = r#"
            INSERT INTO gifts (id, name, price, active)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (id) DO UPDATE
                SET name = EXCLUDED.name,
                    price = EXCLUDED.price,
                    active = EXCLUDED.active
            "#;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::postgres_enabled;
    use anylive_auth::UserStore;

    #[test]
    fn sql_fragments_cover_idempotent_gift_path() {
        assert!(sql::SELECT_ORDER_BY_IDEMPOTENCY.contains("sender_id = $1"));
        assert!(sql::SELECT_ORDER_BY_IDEMPOTENCY.contains("client_request_id = $2"));
        assert!(sql::INSERT_GIFT_ORDER.contains("INSERT INTO gift_orders"));
        assert!(sql::LOCK_BALANCE.contains("FOR UPDATE"));
        assert!(sql::INSERT_LEDGER.contains("wallet_ledger"));
    }

    #[test]
    fn ledger_type_roundtrip() {
        for t in [
            LedgerType::Topup,
            LedgerType::GiftDebit,
            LedgerType::GiftCredit,
            LedgerType::Adjustment,
        ] {
            assert_eq!(parse_ledger_type(ledger_type_str(t)), Some(t));
        }
        assert_eq!(parse_ledger_type("nope"), None);
    }

    #[tokio::test]
    async fn memory_backend_topup() {
        let w = AnyWallet::memory();
        let u = UserId::new();
        let snap = w.credit_topup(u, 10, "t").await.unwrap();
        assert_eq!(snap.balance, 10);
        assert_eq!(w.balance(u).await, 10);
        assert!(!w.is_postgres());
    }

    /// Optional integration — skipped unless `USE_POSTGRES=1` + `DATABASE_URL`.
    #[tokio::test]
    async fn postgres_wallet_topup_and_gift_idempotent() {
        if !postgres_enabled() {
            return;
        }
        let url = std::env::var("DATABASE_URL").unwrap();
        let pool = crate::connect(&url).await.expect("connect");
        crate::migrate(&pool).await.expect("migrate");

        let users = crate::PostgresUserStore::new(pool.clone());
        let sender = users
            .upsert_by_email(&format!("w-send-{}@example.com", Uuid::new_v4()))
            .await
            .unwrap();
        let receiver = users
            .upsert_by_email(&format!("w-recv-{}@example.com", Uuid::new_v4()))
            .await
            .unwrap();

        let w = PostgresWallet::new(pool);
        w.seed_default_gifts().await;
        let gifts = w.list_gifts().await;
        assert!(!gifts.is_empty());
        let rose = gifts.iter().find(|g| g.name == "Rose").unwrap().clone();

        w.credit_topup(sender.id, 10, "mock-pay").await.unwrap();
        assert_eq!(w.balance(sender.id).await, 10);

        let room = Uuid::new_v4();
        let req = format!("req-{}", Uuid::new_v4());
        let (o1, replay1) = w
            .send_gift(room, sender.id, receiver.id, rose.id, 3, &req)
            .await
            .unwrap();
        assert!(!replay1);
        assert_eq!(o1.total_coins, 3);
        assert_eq!(w.balance(sender.id).await, 7);
        assert_eq!(w.balance(receiver.id).await, 3);

        let (o2, replay2) = w
            .send_gift(room, sender.id, receiver.id, rose.id, 3, &req)
            .await
            .unwrap();
        assert!(replay2);
        assert_eq!(o1.id, o2.id);
        assert_eq!(w.balance(sender.id).await, 7);
    }
}
