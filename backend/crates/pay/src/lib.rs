//! Payment control-plane: PayProvider port, catalog, orders, mock channel.
//!
//! PayProvider **never** credits the wallet. Application layer calls
//! `credit_topup(user, coins, "pay:{order_id}")` after a verified notify.

use std::collections::HashMap;
use std::sync::Arc;

use anylive_common::{AppError, ErrorCode};
use anylive_domain::UserId;
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use subtle::ConstantTimeEq;
use tokio::sync::Mutex;
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

/// Default order lifetime for mock / unpaid channel sessions.
pub const DEFAULT_ORDER_TTL_SECS: i64 = 30 * 60;

/// Dev-only default mock HMAC secret (forbidden in production).
pub const DEFAULT_PAY_MOCK_SECRET: &str = "anylive-dev-pay-mock-secret-change-me";

/// Wallet ledger reference prefix — keep in sync with credit orchestration.
pub fn wallet_reference_for_order(order_id: Uuid) -> String {
    format!("pay:{order_id}")
}

// ── Channel ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PayChannel {
    Mock,
    Jeepay,
    Epay,
    Tokenpay,
}

impl PayChannel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Mock => "mock",
            Self::Jeepay => "jeepay",
            Self::Epay => "epay",
            Self::Tokenpay => "tokenpay",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "mock" => Some(Self::Mock),
            "jeepay" => Some(Self::Jeepay),
            "epay" | "epayment" => Some(Self::Epay),
            "tokenpay" | "token_pay" => Some(Self::Tokenpay),
            _ => None,
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::Mock => "Mock (sandbox)",
            Self::Jeepay => "Jeepay",
            Self::Epay => "EPay",
            Self::Tokenpay => "TokenPay",
        }
    }
}

impl std::fmt::Display for PayChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ── Status / modes ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PayOrderStatus {
    Pending,
    Paying,
    Paid,
    Credited,
    Failed,
    Expired,
    Refunded,
}

impl PayOrderStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Paying => "paying",
            Self::Paid => "paid",
            Self::Credited => "credited",
            Self::Failed => "failed",
            Self::Expired => "expired",
            Self::Refunded => "refunded",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(Self::Pending),
            "paying" => Some(Self::Paying),
            "paid" => Some(Self::Paid),
            "credited" => Some(Self::Credited),
            "failed" => Some(Self::Failed),
            "expired" => Some(Self::Expired),
            "refunded" => Some(Self::Refunded),
            _ => None,
        }
    }

    /// Terminal success for the user (coins already in wallet).
    pub fn is_success(self) -> bool {
        matches!(self, Self::Credited)
    }

    /// Already past payment confirmation (including credited).
    pub fn is_paid_or_later(self) -> bool {
        matches!(self, Self::Paid | Self::Credited | Self::Refunded)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PayMode {
    Redirect { url: String },
    QrCode { content: String },
    Jsapi { params: serde_json::Value },
    /// Mock sandbox: client POSTs to webhook with order_id + HMAC.
    MockComplete { hint: String },
    None,
}

impl PayMode {
    pub fn kind_name(&self) -> &'static str {
        match self {
            Self::Redirect { .. } => "redirect",
            Self::QrCode { .. } => "qrcode",
            Self::Jsapi { .. } => "jsapi",
            Self::MockComplete { .. } => "mock_complete",
            Self::None => "none",
        }
    }
}

// ── Domain models ────────────────────────────────────────────────────────────

/// Coin SKU. `amount_minor` is the smallest currency unit (e.g. cents).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PayProduct {
    pub id: Uuid,
    pub sku: String,
    pub title: String,
    pub coins: i64,
    pub amount_minor: i64,
    pub currency: String,
    pub active: bool,
    pub sort_order: i32,
}

impl PayProduct {
    /// Format amount_minor as a fixed 2-decimal string (e.g. 600 → "6.00").
    pub fn amount_display(&self) -> String {
        format_amount_minor(self.amount_minor)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PayOrder {
    pub id: Uuid,
    pub user_id: UserId,
    pub product_id: Option<Uuid>,
    pub channel: PayChannel,
    pub status: PayOrderStatus,
    pub coins: i64,
    pub amount_minor: i64,
    pub currency: String,
    pub client_request_id: Option<String>,
    pub provider_trade_no: Option<String>,
    pub provider_event_id: Option<String>,
    pub pay_mode: Option<String>,
    pub pay_payload: serde_json::Value,
    pub expires_at: Option<DateTime<Utc>>,
    pub paid_at: Option<DateTime<Utc>>,
    pub credited_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl PayOrder {
    pub fn amount_display(&self) -> String {
        format_amount_minor(self.amount_minor)
    }

    pub fn wallet_reference(&self) -> String {
        wallet_reference_for_order(self.id)
    }
}

pub fn format_amount_minor(amount_minor: i64) -> String {
    let sign = if amount_minor < 0 { "-" } else { "" };
    let abs = amount_minor.unsigned_abs();
    let whole = abs / 100;
    let frac = abs % 100;
    format!("{sign}{whole}.{frac:02}")
}

/// Parse "6.00" / "6" into minor units (cents). Rejects more than 2 decimal places.
pub fn parse_amount_to_minor(s: &str) -> Result<i64, AppError> {
    let s = s.trim();
    if s.is_empty() {
        return Err(AppError::validation("empty amount"));
    }
    let negative = s.starts_with('-');
    let s = s.trim_start_matches('-').trim_start_matches('+');
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() > 2 {
        return Err(AppError::validation("invalid amount"));
    }
    let whole: i64 = parts[0]
        .parse()
        .map_err(|_| AppError::validation("invalid amount whole"))?;
    let frac: i64 = if parts.len() == 2 {
        let f = parts[1];
        if f.is_empty() || f.len() > 2 || !f.chars().all(|c| c.is_ascii_digit()) {
            return Err(AppError::validation("invalid amount fraction"));
        }
        let padded = if f.len() == 1 {
            format!("{f}0")
        } else {
            f.to_string()
        };
        padded
            .parse()
            .map_err(|_| AppError::validation("invalid amount fraction"))?
    } else {
        0
    };
    let minor = whole
        .checked_mul(100)
        .and_then(|w| w.checked_add(frac))
        .ok_or_else(|| AppError::validation("amount overflow"))?;
    Ok(if negative { -minor } else { minor })
}

// ── Provider types ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct CreatePaymentRequest {
    pub order_id: Uuid,
    pub user_id: UserId,
    pub amount_minor: i64,
    pub currency: String,
    pub coins: i64,
    pub subject: String,
    pub notify_url: String,
    pub return_url: Option<String>,
    pub client_ip: Option<String>,
    pub extra: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct CreatePaymentResult {
    pub pay_mode: PayMode,
    pub provider_trade_no: Option<String>,
    pub raw: serde_json::Value,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub enum PaymentStatus {
    Pending,
    Success {
        provider_trade_no: String,
        provider_event_id: Option<String>,
        paid_amount_minor: Option<i64>,
    },
    Failed {
        reason: String,
    },
    Closed,
}

#[derive(Debug, Clone)]
pub struct NotifyEvent {
    pub order_id: Uuid,
    pub status: PaymentStatus,
    pub provider_trade_no: String,
    pub provider_event_id: Option<String>,
    pub paid_amount_minor: Option<i64>,
    pub paid_currency: Option<String>,
    pub raw: serde_json::Value,
}

/// Port for external payment channels (Jeepay / EPay / TokenPay / Mock).
#[async_trait]
pub trait PayProvider: Send + Sync {
    fn channel(&self) -> PayChannel;

    async fn create_payment(
        &self,
        req: CreatePaymentRequest,
    ) -> Result<CreatePaymentResult, AppError>;

    async fn query_payment(&self, order_id: Uuid) -> Result<PaymentStatus, AppError>;

    /// Verify signature and parse notify body. Do not credit wallet here.
    async fn parse_and_verify_notify(
        &self,
        headers: &http::HeaderMap,
        body: &[u8],
    ) -> Result<NotifyEvent, AppError>;
}

// ── Mock provider (sandbox) ──────────────────────────────────────────────────

/// Sandbox channel: creates a "mock complete" pay mode; webhook HMAC-signed with
/// `PAY_MOCK_SECRET` (or a process-local default in non-production tests).
#[derive(Debug, Clone)]
pub struct MockPayProvider {
    secret: String,
    public_base: String,
}

impl MockPayProvider {
    pub fn new(secret: impl Into<String>, public_base: impl Into<String>) -> Self {
        Self {
            secret: secret.into(),
            public_base: public_base.into().trim_end_matches('/').to_string(),
        }
    }

    pub fn from_env() -> Self {
        let secret = std::env::var("PAY_MOCK_SECRET")
            .unwrap_or_else(|_| DEFAULT_PAY_MOCK_SECRET.into());
        let public_base = std::env::var("PAY_PUBLIC_BASE_URL")
            .or_else(|_| std::env::var("API_PUBLIC_BASE_URL"))
            .unwrap_or_else(|_| "http://localhost:8088".into());
        Self::new(secret, public_base)
    }

    pub fn secret(&self) -> &str {
        &self.secret
    }

    /// Sign `order_id` for sandbox webhook completion.
    pub fn sign_order(&self, order_id: Uuid) -> String {
        let mut mac = HmacSha256::new_from_slice(self.secret.as_bytes())
            .expect("HMAC key length");
        mac.update(order_id.as_bytes());
        hex::encode(mac.finalize().into_bytes())
    }

    pub fn verify_signature(&self, order_id: Uuid, sig_hex: &str) -> bool {
        let expected = self.sign_order(order_id);
        expected.as_bytes().ct_eq(sig_hex.as_bytes()).into()
    }
}

#[async_trait]
impl PayProvider for MockPayProvider {
    fn channel(&self) -> PayChannel {
        PayChannel::Mock
    }

    async fn create_payment(
        &self,
        req: CreatePaymentRequest,
    ) -> Result<CreatePaymentResult, AppError> {
        // Intentionally do NOT return the HMAC signature to clients. Sandbox
        // tools that know PAY_MOCK_SECRET can sign locally (see dogfood scripts).
        let expires_at = Utc::now() + Duration::seconds(DEFAULT_ORDER_TTL_SECS);
        let raw = serde_json::json!({
            "order_id": req.order_id.to_string(),
            "hint": "sandbox: POST /api/v1/webhooks/pay/mock with {order_id,sig} signed by PAY_MOCK_SECRET",
        });
        Ok(CreatePaymentResult {
            pay_mode: PayMode::MockComplete {
                hint: format!(
                    "sandbox: complete via POST {}/api/v1/webhooks/pay/mock (HMAC order_id with PAY_MOCK_SECRET)",
                    self.public_base
                ),
            },
            provider_trade_no: Some(format!("mock-{}", req.order_id)),
            raw,
            expires_at: Some(expires_at),
        })
    }

    async fn query_payment(&self, _order_id: Uuid) -> Result<PaymentStatus, AppError> {
        // Mock has no remote state; status lives in our store.
        Ok(PaymentStatus::Pending)
    }

    async fn parse_and_verify_notify(
        &self,
        _headers: &http::HeaderMap,
        body: &[u8],
    ) -> Result<NotifyEvent, AppError> {
        #[derive(Deserialize)]
        struct MockBody {
            order_id: String,
            sig: String,
            #[serde(default)]
            trade_no: Option<String>,
            #[serde(default)]
            event_id: Option<String>,
            #[serde(default)]
            amount: Option<String>,
        }
        let parsed: MockBody = serde_json::from_slice(body).map_err(|e| {
            AppError::validation(format!("invalid mock notify body: {e}"))
        })?;
        let order_id = Uuid::parse_str(&parsed.order_id)
            .map_err(|_| AppError::validation("invalid order_id"))?;
        if !self.verify_signature(order_id, &parsed.sig) {
            return Err(AppError::new(
                ErrorCode::Forbidden,
                "invalid mock pay signature",
            ));
        }
        let paid_amount_minor = match parsed.amount.as_deref() {
            Some(a) => Some(parse_amount_to_minor(a)?),
            None => None,
        };
        let trade_no = parsed
            .trade_no
            .unwrap_or_else(|| format!("mock-{order_id}"));
        let event_id = parsed
            .event_id
            .unwrap_or_else(|| format!("mock-evt-{order_id}-{trade_no}"));
        Ok(NotifyEvent {
            order_id,
            status: PaymentStatus::Success {
                provider_trade_no: trade_no.clone(),
                provider_event_id: Some(event_id.clone()),
                paid_amount_minor,
            },
            provider_trade_no: trade_no,
            provider_event_id: Some(event_id),
            paid_amount_minor,
            paid_currency: None,
            raw: serde_json::from_slice(body).unwrap_or(serde_json::Value::Null),
        })
    }
}

// ── In-memory product + order store ──────────────────────────────────────────

#[derive(Clone, Default)]
pub struct MemoryPayStore {
    inner: Arc<Mutex<PayStoreState>>,
}

#[derive(Default)]
struct PayStoreState {
    products: HashMap<Uuid, PayProduct>,
    /// sku → id for uniqueness
    products_by_sku: HashMap<String, Uuid>,
    orders: HashMap<Uuid, PayOrder>,
    /// (user_id, client_request_id) → order_id
    by_client_request: HashMap<(Uuid, String), Uuid>,
    /// processed provider event keys for webhook dedupe: "channel:event_id"
    processed_events: HashMap<String, Uuid>,
}

impl MemoryPayStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn seed_default_products(&self) {
        let mut g = self.inner.lock().await;
        if !g.products.is_empty() {
            return;
        }
        let seeds = [
            ("coins_100", "100 Coins", 100_i64, 600_i64, "CNY", 10),
            ("coins_500", "500 Coins", 500, 2800, "CNY", 20),
            ("coins_1000", "1000 Coins", 1000, 5000, "CNY", 30),
        ];
        for (sku, title, coins, amount_minor, currency, sort) in seeds {
            let id = Uuid::new_v4();
            let p = PayProduct {
                id,
                sku: sku.into(),
                title: title.into(),
                coins,
                amount_minor,
                currency: currency.into(),
                active: true,
                sort_order: sort,
            };
            g.products_by_sku.insert(sku.into(), id);
            g.products.insert(id, p);
        }
    }

    pub async fn list_active_products(&self) -> Vec<PayProduct> {
        let g = self.inner.lock().await;
        let mut items: Vec<_> = g
            .products
            .values()
            .filter(|p| p.active)
            .cloned()
            .collect();
        items.sort_by_key(|p| p.sort_order);
        items
    }

    pub async fn get_product(&self, id: Uuid) -> Option<PayProduct> {
        let g = self.inner.lock().await;
        g.products.get(&id).cloned()
    }

    pub async fn upsert_product(&self, product: PayProduct) {
        let mut g = self.inner.lock().await;
        g.products_by_sku
            .insert(product.sku.clone(), product.id);
        g.products.insert(product.id, product);
    }

    pub async fn get_order(&self, id: Uuid) -> Option<PayOrder> {
        let g = self.inner.lock().await;
        g.orders.get(&id).cloned()
    }

    pub async fn find_by_client_request(
        &self,
        user_id: UserId,
        client_request_id: &str,
    ) -> Option<PayOrder> {
        let g = self.inner.lock().await;
        g.by_client_request
            .get(&(user_id.0, client_request_id.to_string()))
            .and_then(|oid| g.orders.get(oid).cloned())
    }

    /// Insert a new pending order. Returns existing order if client_request_id matches.
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
        let mut g = self.inner.lock().await;
        if let Some(ref crid) = client_request_id {
            if crid.is_empty() || crid.len() > 128 {
                return Err(AppError::validation("invalid client_request_id"));
            }
            if let Some(oid) = g.by_client_request.get(&(user_id.0, crid.clone())) {
                if let Some(existing) = g.orders.get(oid) {
                    return Ok(existing.clone());
                }
            }
        }
        let now = Utc::now();
        let order = PayOrder {
            id: Uuid::new_v4(),
            user_id,
            product_id: Some(product.id),
            channel,
            status: PayOrderStatus::Pending,
            coins: product.coins,
            amount_minor: product.amount_minor,
            currency: product.currency.clone(),
            client_request_id: client_request_id.clone(),
            provider_trade_no: None,
            provider_event_id: None,
            pay_mode: None,
            pay_payload: serde_json::Value::Null,
            expires_at: Some(now + Duration::seconds(DEFAULT_ORDER_TTL_SECS)),
            paid_at: None,
            credited_at: None,
            created_at: now,
            updated_at: now,
        };
        if let Some(crid) = client_request_id {
            g.by_client_request
                .insert((user_id.0, crid), order.id);
        }
        g.orders.insert(order.id, order.clone());
        Ok(order)
    }

    pub async fn mark_paying(
        &self,
        order_id: Uuid,
        pay_mode: &PayMode,
        provider_trade_no: Option<String>,
        pay_payload: serde_json::Value,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<PayOrder, AppError> {
        let mut g = self.inner.lock().await;
        let order = g
            .orders
            .get_mut(&order_id)
            .ok_or_else(|| AppError::not_found("pay order not found"))?;
        if order.status != PayOrderStatus::Pending && order.status != PayOrderStatus::Paying {
            return Err(AppError::new(
                ErrorCode::Conflict,
                format!("order not markable paying (status={})", order.status.as_str()),
            ));
        }
        order.status = PayOrderStatus::Paying;
        order.pay_mode = Some(pay_mode.kind_name().into());
        order.provider_trade_no = provider_trade_no;
        order.pay_payload = pay_payload;
        if expires_at.is_some() {
            order.expires_at = expires_at;
        }
        order.updated_at = Utc::now();
        Ok(order.clone())
    }

    /// Transition paying → paid (idempotent if already paid/credited).
    /// Returns `(order, already_finalized)`.
    pub async fn mark_paid(
        &self,
        order_id: Uuid,
        provider_trade_no: String,
        provider_event_id: Option<String>,
        paid_amount_minor: Option<i64>,
    ) -> Result<(PayOrder, bool), AppError> {
        let mut g = self.inner.lock().await;
        let order = g
            .orders
            .get_mut(&order_id)
            .ok_or_else(|| AppError::not_found("pay order not found"))?;

        if order.status == PayOrderStatus::Credited || order.status == PayOrderStatus::Paid {
            return Ok((order.clone(), true));
        }
        if order.status == PayOrderStatus::Expired || order.status == PayOrderStatus::Failed {
            return Err(AppError::new(
                ErrorCode::Conflict,
                format!("order closed (status={})", order.status.as_str()),
            ));
        }
        if order.status != PayOrderStatus::Paying && order.status != PayOrderStatus::Pending {
            return Err(AppError::new(
                ErrorCode::Conflict,
                format!("order not payable (status={})", order.status.as_str()),
            ));
        }
        let paid = paid_amount_minor.ok_or_else(|| {
            AppError::validation("paid amount required")
        })?;
        if paid != order.amount_minor {
            return Err(AppError::new(
                ErrorCode::Validation,
                format!(
                    "paid amount mismatch: expected {} got {}",
                    order.amount_minor, paid
                ),
            ));
        }
        if let Some(ref exp) = order.expires_at {
            if Utc::now() > *exp && order.status == PayOrderStatus::Paying {
                // Soft-expire on late notify still allowed if within grace? Fail closed.
                // Allow late notify within same day for dogfood: still accept if not expired long.
                // Strict: reject after expires_at.
                return Err(AppError::new(ErrorCode::Conflict, "order expired"));
            }
        }
        let now = Utc::now();
        order.status = PayOrderStatus::Paid;
        order.provider_trade_no = Some(provider_trade_no);
        order.provider_event_id = provider_event_id;
        order.paid_at = Some(now);
        order.updated_at = now;
        Ok((order.clone(), false))
    }

    pub async fn mark_credited(&self, order_id: Uuid) -> Result<PayOrder, AppError> {
        let mut g = self.inner.lock().await;
        let order = g
            .orders
            .get_mut(&order_id)
            .ok_or_else(|| AppError::not_found("pay order not found"))?;
        if order.status == PayOrderStatus::Credited {
            return Ok(order.clone());
        }
        if order.status != PayOrderStatus::Paid {
            return Err(AppError::new(
                ErrorCode::Conflict,
                format!("order not credit-able (status={})", order.status.as_str()),
            ));
        }
        let now = Utc::now();
        order.status = PayOrderStatus::Credited;
        order.credited_at = Some(now);
        order.updated_at = now;
        Ok(order.clone())
    }

    /// Record webhook event for dedupe. Returns false if already seen.
    pub async fn record_webhook_event(
        &self,
        channel: PayChannel,
        provider_event_id: &str,
        order_id: Uuid,
    ) -> bool {
        let key = format!("{}:{}", channel.as_str(), provider_event_id);
        let mut g = self.inner.lock().await;
        if g.processed_events.contains_key(&key) {
            return false;
        }
        g.processed_events.insert(key, order_id);
        true
    }
}

// ── Registry ─────────────────────────────────────────────────────────────────

#[derive(Clone, Default)]
pub struct PayChannelRegistry {
    providers: HashMap<PayChannel, Arc<dyn PayProvider>>,
}

impl PayChannelRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, provider: Arc<dyn PayProvider>) {
        self.providers.insert(provider.channel(), provider);
    }

    pub fn get(&self, channel: PayChannel) -> Option<Arc<dyn PayProvider>> {
        self.providers.get(&channel).cloned()
    }

    pub fn enabled_channels(&self) -> Vec<PayChannel> {
        let mut keys: Vec<_> = self.providers.keys().copied().collect();
        keys.sort_by_key(|c| c.as_str());
        keys
    }

    /// Build registry from env.
    ///
    /// - `PAY_CHANNELS` — comma list, e.g. `mock` or `mock,jeepay`
    /// - When unset: enable `mock` if `ALLOW_MOCK_TOPUP=1` or `PAY_ENABLE_MOCK=1`, else empty.
    pub fn from_env() -> Self {
        let mut reg = Self::new();
        let list = std::env::var("PAY_CHANNELS").unwrap_or_default();
        let channels: Vec<PayChannel> = if list.trim().is_empty() {
            let mock_flag = matches!(
                std::env::var("PAY_ENABLE_MOCK").as_deref(),
                Ok("1") | Ok("true")
            ) || matches!(
                std::env::var("ALLOW_MOCK_TOPUP").as_deref(),
                Ok("1") | Ok("true")
            );
            if mock_flag {
                vec![PayChannel::Mock]
            } else {
                vec![]
            }
        } else {
            list.split(',')
                .filter_map(|s| PayChannel::parse(s))
                .collect()
        };
        for ch in channels {
            match ch {
                PayChannel::Mock => {
                    reg.register(Arc::new(MockPayProvider::from_env()));
                }
                // Real PSP adapters land in follow-up commits; skip with warning.
                PayChannel::Jeepay | PayChannel::Epay | PayChannel::Tokenpay => {
                    tracing::warn!(
                        channel = ch.as_str(),
                        "pay channel listed in PAY_CHANNELS but adapter not implemented yet"
                    );
                }
            }
        }
        reg
    }

    /// Test helper: mock-only registry with fixed secret.
    pub fn mock_only(secret: &str) -> Self {
        let mut reg = Self::new();
        reg.register(Arc::new(MockPayProvider::new(
            secret,
            "http://localhost:8088",
        )));
        reg
    }
}

/// True when env would enable the mock pay channel.
pub fn mock_pay_enabled_from_env() -> bool {
    if matches!(
        std::env::var("PAY_ENABLE_MOCK").as_deref(),
        Ok("1") | Ok("true")
    ) {
        return true;
    }
    if let Ok(list) = std::env::var("PAY_CHANNELS") {
        if !list.trim().is_empty() {
            return list
                .split(',')
                .any(|s| PayChannel::parse(s) == Some(PayChannel::Mock));
        }
    }
    // Empty PAY_CHANNELS falls back to ALLOW_MOCK_TOPUP / PAY_ENABLE_MOCK (already checked).
    matches!(
        std::env::var("ALLOW_MOCK_TOPUP").as_deref(),
        Ok("1") | Ok("true")
    )
}

// ── Dual store enum (memory; Postgres via anylive-db) ────────────────────────

/// Trait object surface used by API for products/orders (memory implementation here).
#[async_trait]
pub trait PayStore: Send + Sync {
    async fn seed_default_products(&self);
    async fn list_active_products(&self) -> Vec<PayProduct>;
    async fn get_product(&self, id: Uuid) -> Option<PayProduct>;
    async fn create_order(
        &self,
        user_id: UserId,
        product: &PayProduct,
        channel: PayChannel,
        client_request_id: Option<String>,
    ) -> Result<PayOrder, AppError>;
    async fn get_order(&self, id: Uuid) -> Option<PayOrder>;
    async fn find_by_client_request(
        &self,
        user_id: UserId,
        client_request_id: &str,
    ) -> Option<PayOrder>;
    async fn mark_paying(
        &self,
        order_id: Uuid,
        pay_mode: &PayMode,
        provider_trade_no: Option<String>,
        pay_payload: serde_json::Value,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<PayOrder, AppError>;
    async fn mark_paid(
        &self,
        order_id: Uuid,
        provider_trade_no: String,
        provider_event_id: Option<String>,
        paid_amount_minor: Option<i64>,
    ) -> Result<(PayOrder, bool), AppError>;
    async fn mark_credited(&self, order_id: Uuid) -> Result<PayOrder, AppError>;
    async fn record_webhook_event(
        &self,
        channel: PayChannel,
        provider_event_id: &str,
        order_id: Uuid,
    ) -> bool;
}

#[async_trait]
impl PayStore for MemoryPayStore {
    async fn seed_default_products(&self) {
        MemoryPayStore::seed_default_products(self).await
    }
    async fn list_active_products(&self) -> Vec<PayProduct> {
        MemoryPayStore::list_active_products(self).await
    }
    async fn get_product(&self, id: Uuid) -> Option<PayProduct> {
        MemoryPayStore::get_product(self, id).await
    }
    async fn create_order(
        &self,
        user_id: UserId,
        product: &PayProduct,
        channel: PayChannel,
        client_request_id: Option<String>,
    ) -> Result<PayOrder, AppError> {
        MemoryPayStore::create_order(self, user_id, product, channel, client_request_id).await
    }
    async fn get_order(&self, id: Uuid) -> Option<PayOrder> {
        MemoryPayStore::get_order(self, id).await
    }
    async fn find_by_client_request(
        &self,
        user_id: UserId,
        client_request_id: &str,
    ) -> Option<PayOrder> {
        MemoryPayStore::find_by_client_request(self, user_id, client_request_id).await
    }
    async fn mark_paying(
        &self,
        order_id: Uuid,
        pay_mode: &PayMode,
        provider_trade_no: Option<String>,
        pay_payload: serde_json::Value,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<PayOrder, AppError> {
        MemoryPayStore::mark_paying(
            self,
            order_id,
            pay_mode,
            provider_trade_no,
            pay_payload,
            expires_at,
        )
        .await
    }
    async fn mark_paid(
        &self,
        order_id: Uuid,
        provider_trade_no: String,
        provider_event_id: Option<String>,
        paid_amount_minor: Option<i64>,
    ) -> Result<(PayOrder, bool), AppError> {
        MemoryPayStore::mark_paid(
            self,
            order_id,
            provider_trade_no,
            provider_event_id,
            paid_amount_minor,
        )
        .await
    }
    async fn mark_credited(&self, order_id: Uuid) -> Result<PayOrder, AppError> {
        MemoryPayStore::mark_credited(self, order_id).await
    }
    async fn record_webhook_event(
        &self,
        channel: PayChannel,
        provider_event_id: &str,
        order_id: Uuid,
    ) -> bool {
        MemoryPayStore::record_webhook_event(self, channel, provider_event_id, order_id).await
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn amount_roundtrip() {
        assert_eq!(format_amount_minor(600), "6.00");
        assert_eq!(format_amount_minor(5), "0.05");
        assert_eq!(parse_amount_to_minor("6.00").unwrap(), 600);
        assert_eq!(parse_amount_to_minor("6").unwrap(), 600);
        assert_eq!(parse_amount_to_minor("0.05").unwrap(), 5);
    }

    #[test]
    fn channel_parse() {
        assert_eq!(PayChannel::parse("Mock"), Some(PayChannel::Mock));
        assert_eq!(PayChannel::parse("epay"), Some(PayChannel::Epay));
        assert!(PayChannel::parse("stripe").is_none());
    }

    #[tokio::test]
    async fn mock_create_and_verify_notify() {
        let mock = MockPayProvider::new("test-secret", "http://localhost:8088");
        let order_id = Uuid::new_v4();
        let result = mock
            .create_payment(CreatePaymentRequest {
                order_id,
                user_id: UserId(Uuid::new_v4()),
                amount_minor: 600,
                currency: "CNY".into(),
                coins: 100,
                subject: "100 Coins".into(),
                notify_url: "http://localhost/hook".into(),
                return_url: None,
                client_ip: None,
                extra: serde_json::Value::Null,
            })
            .await
            .unwrap();
        assert!(matches!(result.pay_mode, PayMode::MockComplete { .. }));
        let sig = mock.sign_order(order_id);
        let body = serde_json::json!({
            "order_id": order_id.to_string(),
            "sig": sig,
        });
        let ev = mock
            .parse_and_verify_notify(&http::HeaderMap::new(), &serde_json::to_vec(&body).unwrap())
            .await
            .unwrap();
        assert_eq!(ev.order_id, order_id);
        assert!(matches!(ev.status, PaymentStatus::Success { .. }));
    }

    #[tokio::test]
    async fn mock_rejects_bad_sig() {
        let mock = MockPayProvider::new("test-secret", "http://localhost:8088");
        let order_id = Uuid::new_v4();
        let body = serde_json::json!({
            "order_id": order_id.to_string(),
            "sig": "deadbeef",
        });
        let err = mock
            .parse_and_verify_notify(&http::HeaderMap::new(), &serde_json::to_vec(&body).unwrap())
            .await
            .unwrap_err();
        assert_eq!(err.code, ErrorCode::Forbidden);
    }

    #[tokio::test]
    async fn order_idempotent_client_request_and_credit_path() {
        let store = MemoryPayStore::new();
        store.seed_default_products().await;
        let products = store.list_active_products().await;
        let product = &products[0];
        let user = UserId(Uuid::new_v4());
        let o1 = store
            .create_order(
                user,
                product,
                PayChannel::Mock,
                Some("req-1".into()),
            )
            .await
            .unwrap();
        let o2 = store
            .create_order(
                user,
                product,
                PayChannel::Mock,
                Some("req-1".into()),
            )
            .await
            .unwrap();
        assert_eq!(o1.id, o2.id);

        store
            .mark_paying(
                o1.id,
                &PayMode::MockComplete {
                    hint: "x".into(),
                },
                Some("t1".into()),
                serde_json::json!({}),
                None,
            )
            .await
            .unwrap();
        let (paid, replay) = store
            .mark_paid(o1.id, "t1".into(), Some("e1".into()), Some(product.amount_minor))
            .await
            .unwrap();
        assert!(!replay);
        assert_eq!(paid.status, PayOrderStatus::Paid);
        let (paid2, replay2) = store
            .mark_paid(o1.id, "t1".into(), Some("e1".into()), Some(product.amount_minor))
            .await
            .unwrap();
        assert!(replay2);
        assert_eq!(paid2.status, PayOrderStatus::Paid);
        let credited = store.mark_credited(o1.id).await.unwrap();
        assert_eq!(credited.status, PayOrderStatus::Credited);
        assert!(store.record_webhook_event(PayChannel::Mock, "e1", o1.id).await);
        assert!(!store.record_webhook_event(PayChannel::Mock, "e1", o1.id).await);
    }

    #[tokio::test]
    async fn mark_paid_rejects_amount_mismatch() {
        let store = MemoryPayStore::new();
        store.seed_default_products().await;
        let product = &store.list_active_products().await[0];
        let user = UserId(Uuid::new_v4());
        let o = store
            .create_order(user, product, PayChannel::Mock, None)
            .await
            .unwrap();
        store
            .mark_paying(
                o.id,
                &PayMode::None,
                None,
                serde_json::Value::Null,
                None,
            )
            .await
            .unwrap();
        let err = store
            .mark_paid(o.id, "t".into(), None, Some(product.amount_minor + 1))
            .await
            .unwrap_err();
        assert_eq!(err.code, ErrorCode::Validation);
    }
}
