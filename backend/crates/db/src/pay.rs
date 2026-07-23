//! [`PostgresPayStore`] + dual [`AnyPayStore`] for pay products/orders.

use anylive_common::{AppError, ErrorCode};
use anylive_domain::UserId;
use anylive_pay::{
    MemoryPayStore, PayChannel, PayMode, PayOrder, PayOrderStatus, PayProduct, PayStore,
    DEFAULT_ORDER_TTL_SECS,
};
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Clone)]
pub struct PostgresPayStore {
    pool: PgPool,
}

impl PostgresPayStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn seed_default_products(&self) {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM pay_products")
            .fetch_one(&self.pool)
            .await
            .unwrap_or(0);
        if count > 0 {
            return;
        }
        let seeds = [
            ("coins_100", "100 Coins", 100_i64, 600_i64, "CNY", 10_i32),
            ("coins_500", "500 Coins", 500, 2800, "CNY", 20),
            ("coins_1000", "1000 Coins", 1000, 5000, "CNY", 30),
        ];
        for (sku, title, coins, amount_minor, currency, sort) in seeds {
            let id = Uuid::new_v4();
            let _ = sqlx::query(
                r#"
                INSERT INTO pay_products (id, sku, title, coins, amount_minor, currency, active, sort_order)
                VALUES ($1, $2, $3, $4, $5, $6, true, $7)
                ON CONFLICT (sku) DO NOTHING
                "#,
            )
            .bind(id)
            .bind(sku)
            .bind(title)
            .bind(coins)
            .bind(amount_minor)
            .bind(currency)
            .bind(sort)
            .execute(&self.pool)
            .await;
        }
    }

    pub async fn list_active_products(&self) -> Vec<PayProduct> {
        let rows = sqlx::query_as::<_, ProductRow>(
            r#"
            SELECT id, sku, title, coins, amount_minor, currency, active, sort_order
            FROM pay_products
            WHERE active = true
            ORDER BY sort_order ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();
        rows.into_iter().map(ProductRow::into_product).collect()
    }

    pub async fn get_product(&self, id: Uuid) -> Option<PayProduct> {
        let row = sqlx::query_as::<_, ProductRow>(
            r#"
            SELECT id, sku, title, coins, amount_minor, currency, active, sort_order
            FROM pay_products WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .ok()
        .flatten()?;
        Some(row.into_product())
    }

    pub async fn create_order(
        &self,
        user_id: UserId,
        product: &PayProduct,
        channel: PayChannel,
        client_request_id: Option<String>,
    ) -> Result<PayOrder, AppError> {
        if !product.active {
            return Err(AppError::validation("product inactive"));
        }
        if product.coins <= 0 || product.amount_minor <= 0 {
            return Err(AppError::validation("invalid product pricing"));
        }
        if let Some(ref crid) = client_request_id {
            if crid.is_empty() || crid.len() > 128 {
                return Err(AppError::validation("invalid client_request_id"));
            }
            if let Some(existing) = self.find_by_client_request(user_id, crid).await {
                return Ok(existing);
            }
        }
        let now = Utc::now();
        let id = Uuid::new_v4();
        let expires = now + Duration::seconds(DEFAULT_ORDER_TTL_SECS);
        let row = sqlx::query_as::<_, OrderRow>(
            r#"
            INSERT INTO pay_orders (
                id, user_id, product_id, channel, status, coins, amount_minor, currency,
                client_request_id, pay_payload, expires_at, created_at, updated_at
            )
            VALUES ($1,$2,$3,$4,'pending',$5,$6,$7,$8,'{}'::jsonb,$9,$10,$10)
            RETURNING id, user_id, product_id, channel, status, coins, amount_minor, currency,
                      client_request_id, provider_trade_no, provider_event_id, pay_mode,
                      pay_payload, expires_at, paid_at, credited_at, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(user_id.0)
        .bind(product.id)
        .bind(channel.as_str())
        .bind(product.coins)
        .bind(product.amount_minor)
        .bind(&product.currency)
        .bind(&client_request_id)
        .bind(expires)
        .bind(now)
        .fetch_one(&self.pool)
        .await;

        match row {
            Ok(r) => r.into_order(),
            Err(sqlx::Error::Database(ref db))
                if db.constraint() == Some("pay_orders_user_client_request_uidx") =>
            {
                let crid = client_request_id.as_deref().unwrap_or("");
                self.find_by_client_request(user_id, crid)
                    .await
                    .ok_or_else(|| {
                        AppError::new(ErrorCode::Conflict, "pay order idempotent race")
                    })
            }
            Err(e) => Err(map_db(e)),
        }
    }

    pub async fn get_order(&self, id: Uuid) -> Option<PayOrder> {
        let row = sqlx::query_as::<_, OrderRow>(
            r#"
            SELECT id, user_id, product_id, channel, status, coins, amount_minor, currency,
                   client_request_id, provider_trade_no, provider_event_id, pay_mode,
                   pay_payload, expires_at, paid_at, credited_at, created_at, updated_at
            FROM pay_orders WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .ok()
        .flatten()?;
        row.into_order().ok()
    }

    pub async fn find_by_client_request(
        &self,
        user_id: UserId,
        client_request_id: &str,
    ) -> Option<PayOrder> {
        let row = sqlx::query_as::<_, OrderRow>(
            r#"
            SELECT id, user_id, product_id, channel, status, coins, amount_minor, currency,
                   client_request_id, provider_trade_no, provider_event_id, pay_mode,
                   pay_payload, expires_at, paid_at, credited_at, created_at, updated_at
            FROM pay_orders
            WHERE user_id = $1 AND client_request_id = $2
            "#,
        )
        .bind(user_id.0)
        .bind(client_request_id)
        .fetch_optional(&self.pool)
        .await
        .ok()
        .flatten()?;
        row.into_order().ok()
    }

    pub async fn mark_paying(
        &self,
        order_id: Uuid,
        pay_mode: &PayMode,
        provider_trade_no: Option<String>,
        pay_payload: serde_json::Value,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<PayOrder, AppError> {
        let existing = self
            .get_order(order_id)
            .await
            .ok_or_else(|| AppError::not_found("pay order not found"))?;
        if existing.status != PayOrderStatus::Pending
            && existing.status != PayOrderStatus::Paying
        {
            return Err(AppError::new(
                ErrorCode::Conflict,
                format!(
                    "order not markable paying (status={})",
                    existing.status.as_str()
                ),
            ));
        }
        let exp = expires_at.or(existing.expires_at);
        let now = Utc::now();
        let row = sqlx::query_as::<_, OrderRow>(
            r#"
            UPDATE pay_orders SET
                status = 'paying',
                pay_mode = $2,
                provider_trade_no = COALESCE($3, provider_trade_no),
                pay_payload = $4,
                expires_at = COALESCE($5, expires_at),
                updated_at = $6
            WHERE id = $1
            RETURNING id, user_id, product_id, channel, status, coins, amount_minor, currency,
                      client_request_id, provider_trade_no, provider_event_id, pay_mode,
                      pay_payload, expires_at, paid_at, credited_at, created_at, updated_at
            "#,
        )
        .bind(order_id)
        .bind(pay_mode.kind_name())
        .bind(provider_trade_no)
        .bind(pay_payload)
        .bind(exp)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(map_db)?;
        row.into_order()
    }

    pub async fn mark_paid(
        &self,
        order_id: Uuid,
        provider_trade_no: String,
        provider_event_id: Option<String>,
        paid_amount_minor: Option<i64>,
    ) -> Result<(PayOrder, bool), AppError> {
        let existing = self
            .get_order(order_id)
            .await
            .ok_or_else(|| AppError::not_found("pay order not found"))?;
        if existing.status == PayOrderStatus::Credited || existing.status == PayOrderStatus::Paid {
            // Amount check still applies on late notifies with amount present.
            if let Some(paid) = paid_amount_minor {
                if paid != existing.amount_minor {
                    return Err(AppError::new(
                        ErrorCode::Validation,
                        format!(
                            "paid amount mismatch: expected {} got {}",
                            existing.amount_minor, paid
                        ),
                    ));
                }
            }
            return Ok((existing, true));
        }
        if existing.status == PayOrderStatus::Expired || existing.status == PayOrderStatus::Failed {
            return Err(AppError::new(
                ErrorCode::Conflict,
                format!("order closed (status={})", existing.status.as_str()),
            ));
        }
        if existing.status != PayOrderStatus::Paying && existing.status != PayOrderStatus::Pending {
            return Err(AppError::new(
                ErrorCode::Conflict,
                format!("order not payable (status={})", existing.status.as_str()),
            ));
        }
        if let Some(paid) = paid_amount_minor {
            if paid != existing.amount_minor {
                return Err(AppError::new(
                    ErrorCode::Validation,
                    format!(
                        "paid amount mismatch: expected {} got {}",
                        existing.amount_minor, paid
                    ),
                ));
            }
        } else {
            // Production channels must prove amount; memory/tests may pass Some from API.
            return Err(AppError::validation("paid amount required"));
        }
        if let Some(exp) = existing.expires_at {
            if Utc::now() > exp {
                return Err(AppError::new(ErrorCode::Conflict, "order expired"));
            }
        }
        let now = Utc::now();
        // CAS: only transition from pending/paying to avoid TOCTOU double-wins.
        let row = sqlx::query_as::<_, OrderRow>(
            r#"
            UPDATE pay_orders SET
                status = 'paid',
                provider_trade_no = $2,
                provider_event_id = COALESCE($3, provider_event_id),
                paid_at = $4,
                updated_at = $4
            WHERE id = $1 AND status IN ('pending', 'paying')
            RETURNING id, user_id, product_id, channel, status, coins, amount_minor, currency,
                      client_request_id, provider_trade_no, provider_event_id, pay_mode,
                      pay_payload, expires_at, paid_at, credited_at, created_at, updated_at
            "#,
        )
        .bind(order_id)
        .bind(provider_trade_no)
        .bind(provider_event_id)
        .bind(now)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db)?;

        match row {
            Some(r) => Ok((r.into_order()?, false)),
            None => {
                // Lost race or concurrent credit — re-fetch.
                let again = self
                    .get_order(order_id)
                    .await
                    .ok_or_else(|| AppError::not_found("pay order not found"))?;
                if again.status == PayOrderStatus::Paid || again.status == PayOrderStatus::Credited {
                    Ok((again, true))
                } else {
                    Err(AppError::new(
                        ErrorCode::Conflict,
                        format!(
                            "order not payable after CAS (status={})",
                            again.status.as_str()
                        ),
                    ))
                }
            }
        }
    }

    pub async fn mark_credited(&self, order_id: Uuid) -> Result<PayOrder, AppError> {
        let existing = self
            .get_order(order_id)
            .await
            .ok_or_else(|| AppError::not_found("pay order not found"))?;
        if existing.status == PayOrderStatus::Credited {
            return Ok(existing);
        }
        if existing.status != PayOrderStatus::Paid {
            return Err(AppError::new(
                ErrorCode::Conflict,
                format!(
                    "order not credit-able (status={})",
                    existing.status.as_str()
                ),
            ));
        }
        let now = Utc::now();
        let row = sqlx::query_as::<_, OrderRow>(
            r#"
            UPDATE pay_orders SET
                status = 'credited',
                credited_at = $2,
                updated_at = $2
            WHERE id = $1
            RETURNING id, user_id, product_id, channel, status, coins, amount_minor, currency,
                      client_request_id, provider_trade_no, provider_event_id, pay_mode,
                      pay_payload, expires_at, paid_at, credited_at, created_at, updated_at
            "#,
        )
        .bind(order_id)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(map_db)?;
        row.into_order()
    }

    pub async fn record_webhook_event(
        &self,
        channel: PayChannel,
        provider_event_id: &str,
        order_id: Uuid,
    ) -> bool {
        let res = sqlx::query(
            r#"
            INSERT INTO pay_webhook_events (channel, provider_event_id, order_id, verified, process_status)
            VALUES ($1, $2, $3, true, 'processed')
            ON CONFLICT (channel, provider_event_id) DO NOTHING
            "#,
        )
        .bind(channel.as_str())
        .bind(provider_event_id)
        .bind(order_id)
        .execute(&self.pool)
        .await;
        match res {
            Ok(r) => r.rows_affected() > 0,
            Err(_) => false,
        }
    }

    /// Mark pending/paying orders past `expires_at` as `expired`.
    pub async fn expire_stale_orders(&self, now: chrono::DateTime<Utc>) -> u64 {
        let res = sqlx::query(
            r#"
            UPDATE pay_orders
            SET status = 'expired', updated_at = $1
            WHERE status IN ('pending', 'paying')
              AND expires_at IS NOT NULL
              AND expires_at < $1
            "#,
        )
        .bind(now)
        .execute(&self.pool)
        .await;
        match res {
            Ok(r) => r.rows_affected(),
            Err(err) => {
                tracing::error!(error = %err, "expire_stale_orders failed");
                0
            }
        }
    }
}

#[derive(sqlx::FromRow)]
struct ProductRow {
    id: Uuid,
    sku: String,
    title: String,
    coins: i64,
    amount_minor: i64,
    currency: String,
    active: bool,
    sort_order: i32,
}

impl ProductRow {
    fn into_product(self) -> PayProduct {
        PayProduct {
            id: self.id,
            sku: self.sku,
            title: self.title,
            coins: self.coins,
            amount_minor: self.amount_minor,
            currency: self.currency,
            active: self.active,
            sort_order: self.sort_order,
        }
    }
}

#[derive(sqlx::FromRow)]
struct OrderRow {
    id: Uuid,
    user_id: Uuid,
    product_id: Option<Uuid>,
    channel: String,
    status: String,
    coins: i64,
    amount_minor: i64,
    currency: String,
    client_request_id: Option<String>,
    provider_trade_no: Option<String>,
    provider_event_id: Option<String>,
    pay_mode: Option<String>,
    pay_payload: serde_json::Value,
    expires_at: Option<DateTime<Utc>>,
    paid_at: Option<DateTime<Utc>>,
    credited_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl OrderRow {
    fn into_order(self) -> Result<PayOrder, AppError> {
        let channel = PayChannel::parse(&self.channel).ok_or_else(|| {
            AppError::new(ErrorCode::Internal, format!("unknown channel {}", self.channel))
        })?;
        let status = PayOrderStatus::parse(&self.status).ok_or_else(|| {
            AppError::new(ErrorCode::Internal, format!("unknown status {}", self.status))
        })?;
        Ok(PayOrder {
            id: self.id,
            user_id: UserId(self.user_id),
            product_id: self.product_id,
            channel,
            status,
            coins: self.coins,
            amount_minor: self.amount_minor,
            currency: self.currency,
            client_request_id: self.client_request_id,
            provider_trade_no: self.provider_trade_no,
            provider_event_id: self.provider_event_id,
            pay_mode: self.pay_mode,
            pay_payload: self.pay_payload,
            expires_at: self.expires_at,
            paid_at: self.paid_at,
            credited_at: self.credited_at,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}

fn map_db(err: sqlx::Error) -> AppError {
    AppError::new(ErrorCode::Internal, format!("database error: {err}"))
}

/// Dual backend for API state (memory default / Postgres when enabled).
#[derive(Clone)]
pub enum AnyPayStore {
    Memory(MemoryPayStore),
    Postgres(PostgresPayStore),
}

impl AnyPayStore {
    pub fn memory() -> Self {
        Self::Memory(MemoryPayStore::new())
    }

    pub fn postgres(pool: PgPool) -> Self {
        Self::Postgres(PostgresPayStore::new(pool))
    }
}

#[async_trait]
impl PayStore for AnyPayStore {
    async fn seed_default_products(&self) {
        match self {
            Self::Memory(s) => s.seed_default_products().await,
            Self::Postgres(s) => s.seed_default_products().await,
        }
    }

    async fn list_active_products(&self) -> Vec<PayProduct> {
        match self {
            Self::Memory(s) => s.list_active_products().await,
            Self::Postgres(s) => s.list_active_products().await,
        }
    }

    async fn get_product(&self, id: Uuid) -> Option<PayProduct> {
        match self {
            Self::Memory(s) => s.get_product(id).await,
            Self::Postgres(s) => s.get_product(id).await,
        }
    }

    async fn create_order(
        &self,
        user_id: UserId,
        product: &PayProduct,
        channel: PayChannel,
        client_request_id: Option<String>,
    ) -> Result<PayOrder, AppError> {
        match self {
            Self::Memory(s) => s.create_order(user_id, product, channel, client_request_id).await,
            Self::Postgres(s) => s.create_order(user_id, product, channel, client_request_id).await,
        }
    }

    async fn get_order(&self, id: Uuid) -> Option<PayOrder> {
        match self {
            Self::Memory(s) => s.get_order(id).await,
            Self::Postgres(s) => s.get_order(id).await,
        }
    }

    async fn find_by_client_request(
        &self,
        user_id: UserId,
        client_request_id: &str,
    ) -> Option<PayOrder> {
        match self {
            Self::Memory(s) => s.find_by_client_request(user_id, client_request_id).await,
            Self::Postgres(s) => s.find_by_client_request(user_id, client_request_id).await,
        }
    }

    async fn mark_paying(
        &self,
        order_id: Uuid,
        pay_mode: &PayMode,
        provider_trade_no: Option<String>,
        pay_payload: serde_json::Value,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<PayOrder, AppError> {
        match self {
            Self::Memory(s) => {
                s.mark_paying(order_id, pay_mode, provider_trade_no, pay_payload, expires_at)
                    .await
            }
            Self::Postgres(s) => {
                s.mark_paying(order_id, pay_mode, provider_trade_no, pay_payload, expires_at)
                    .await
            }
        }
    }

    async fn mark_paid(
        &self,
        order_id: Uuid,
        provider_trade_no: String,
        provider_event_id: Option<String>,
        paid_amount_minor: Option<i64>,
    ) -> Result<(PayOrder, bool), AppError> {
        match self {
            Self::Memory(s) => {
                s.mark_paid(order_id, provider_trade_no, provider_event_id, paid_amount_minor)
                    .await
            }
            Self::Postgres(s) => {
                s.mark_paid(order_id, provider_trade_no, provider_event_id, paid_amount_minor)
                    .await
            }
        }
    }

    async fn mark_credited(&self, order_id: Uuid) -> Result<PayOrder, AppError> {
        match self {
            Self::Memory(s) => s.mark_credited(order_id).await,
            Self::Postgres(s) => s.mark_credited(order_id).await,
        }
    }

    async fn record_webhook_event(
        &self,
        channel: PayChannel,
        provider_event_id: &str,
        order_id: Uuid,
    ) -> bool {
        match self {
            Self::Memory(s) => {
                s.record_webhook_event(channel, provider_event_id, order_id)
                    .await
            }
            Self::Postgres(s) => {
                s.record_webhook_event(channel, provider_event_id, order_id)
                    .await
            }
        }
    }

    async fn expire_stale_orders(&self, now: chrono::DateTime<Utc>) -> u64 {
        match self {
            Self::Memory(s) => s.expire_stale_orders(now).await,
            Self::Postgres(s) => s.expire_stale_orders(now).await,
        }
    }
}
