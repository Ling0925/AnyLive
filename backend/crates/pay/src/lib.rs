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
    /// Overseas card path (Stripe Checkout / PaymentIntent sandbox).
    Stripe,
    /// App Store / Google Play IAP sandbox.
    Iap,
}

impl PayChannel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Mock => "mock",
            Self::Jeepay => "jeepay",
            Self::Epay => "epay",
            Self::Tokenpay => "tokenpay",
            Self::Stripe => "stripe",
            Self::Iap => "iap",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "mock" => Some(Self::Mock),
            "jeepay" => Some(Self::Jeepay),
            "epay" | "epayment" => Some(Self::Epay),
            "tokenpay" | "token_pay" => Some(Self::Tokenpay),
            "stripe" => Some(Self::Stripe),
            "iap" | "app_store" | "play_billing" | "storekit" => Some(Self::Iap),
            _ => None,
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::Mock => "Mock (sandbox)",
            Self::Jeepay => "Jeepay",
            Self::Epay => "EPay",
            Self::Tokenpay => "TokenPay",
            Self::Stripe => "Stripe",
            Self::Iap => "App Store / Play IAP",
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

// ── Stripe sandbox provider (P2 overseas card path) ──────────────────────────

/// Dev/stage Stripe adapter: HMAC-signed sandbox notify (same pattern as mock).
///
/// Production will swap body verification for Stripe-Signature + webhook secret.
/// Until then, enable with `PAY_CHANNELS=...,stripe` and set `PAY_STRIPE_SECRET`
/// (falls back to mock secret / default only in non-production dogfood).
#[derive(Debug, Clone)]
pub struct StripePayProvider {
    secret: String,
    public_base: String,
    /// Optional Checkout base; when empty, returns a synthetic redirect URL.
    checkout_base: String,
}

impl StripePayProvider {
    pub fn new(
        secret: impl Into<String>,
        public_base: impl Into<String>,
        checkout_base: impl Into<String>,
    ) -> Self {
        Self {
            secret: secret.into(),
            public_base: public_base.into().trim_end_matches('/').to_string(),
            checkout_base: checkout_base.into().trim_end_matches('/').to_string(),
        }
    }

    pub fn from_env() -> Self {
        let secret = std::env::var("PAY_STRIPE_SECRET")
            .or_else(|_| std::env::var("STRIPE_WEBHOOK_SECRET"))
            .or_else(|_| std::env::var("PAY_MOCK_SECRET"))
            .unwrap_or_else(|_| DEFAULT_PAY_MOCK_SECRET.into());
        let public_base = std::env::var("PAY_PUBLIC_BASE_URL")
            .or_else(|_| std::env::var("API_PUBLIC_BASE_URL"))
            .unwrap_or_else(|_| "http://localhost:8088".into());
        let checkout_base = std::env::var("PAY_STRIPE_CHECKOUT_BASE")
            .unwrap_or_else(|_| "https://checkout.stripe.com/c/pay".into());
        Self::new(secret, public_base, checkout_base)
    }

    pub fn secret(&self) -> &str {
        &self.secret
    }

    pub fn sign_order(&self, order_id: Uuid) -> String {
        let mut mac = HmacSha256::new_from_slice(self.secret.as_bytes())
            .expect("HMAC key length");
        mac.update(b"stripe:");
        mac.update(order_id.as_bytes());
        hex::encode(mac.finalize().into_bytes())
    }

    pub fn verify_signature(&self, order_id: Uuid, sig_hex: &str) -> bool {
        let expected = self.sign_order(order_id);
        expected.as_bytes().ct_eq(sig_hex.as_bytes()).into()
    }
}

#[async_trait]
impl PayProvider for StripePayProvider {
    fn channel(&self) -> PayChannel {
        PayChannel::Stripe
    }

    async fn create_payment(
        &self,
        req: CreatePaymentRequest,
    ) -> Result<CreatePaymentResult, AppError> {
        let expires_at = Utc::now() + Duration::seconds(DEFAULT_ORDER_TTL_SECS);
        let session_id = format!("cs_test_{}", req.order_id.simple());
        let url = if self.checkout_base.is_empty() {
            format!(
                "{}/api/v1/pay/orders/{}/sandbox-complete",
                self.public_base, req.order_id
            )
        } else {
            format!("{}/{}", self.checkout_base, session_id)
        };
        let raw = serde_json::json!({
            "order_id": req.order_id.to_string(),
            "session_id": session_id,
            "url": url,
            "hint": "stripe sandbox: complete via POST /api/v1/webhooks/pay/stripe with {order_id,sig}",
        });
        Ok(CreatePaymentResult {
            pay_mode: PayMode::Redirect { url },
            provider_trade_no: Some(session_id),
            raw,
            expires_at: Some(expires_at),
        })
    }

    async fn query_payment(&self, _order_id: Uuid) -> Result<PaymentStatus, AppError> {
        Ok(PaymentStatus::Pending)
    }

    async fn parse_and_verify_notify(
        &self,
        headers: &http::HeaderMap,
        body: &[u8],
    ) -> Result<NotifyEvent, AppError> {
        // Prefer Stripe-Signature header when present (production shape),
        // but sandbox dogfood uses JSON {order_id, sig} like mock.
        if let Some(sig_header) = headers
            .get("stripe-signature")
            .and_then(|v| v.to_str().ok())
        {
            // Minimal Stripe-Signature parser: t=...,v1=... (HMAC over "{t}.{body}").
            let mut t_val: Option<&str> = None;
            let mut v1_val: Option<&str> = None;
            for part in sig_header.split(',') {
                let mut kv = part.splitn(2, '=');
                match (kv.next().map(str::trim), kv.next().map(str::trim)) {
                    (Some("t"), Some(t)) => t_val = Some(t),
                    (Some("v1"), Some(v)) => v1_val = Some(v),
                    _ => {}
                }
            }
            let (t, v1) = match (t_val, v1_val) {
                (Some(t), Some(v)) => (t, v),
                _ => {
                    return Err(AppError::new(
                        ErrorCode::Forbidden,
                        "invalid Stripe-Signature header",
                    ));
                }
            };
            let mut mac = HmacSha256::new_from_slice(self.secret.as_bytes())
                .expect("HMAC key length");
            mac.update(t.as_bytes());
            mac.update(b".");
            mac.update(body);
            let expected = hex::encode(mac.finalize().into_bytes());
            if !bool::from(expected.as_bytes().ct_eq(v1.as_bytes())) {
                return Err(AppError::new(
                    ErrorCode::Forbidden,
                    "invalid stripe signature",
                ));
            }
            #[derive(Deserialize)]
            struct StripeEvt {
                id: Option<String>,
                #[serde(default)]
                data: serde_json::Value,
                #[serde(rename = "type")]
                event_type: Option<String>,
            }
            let evt: StripeEvt = serde_json::from_slice(body).map_err(|e| {
                AppError::validation(format!("invalid stripe event body: {e}"))
            })?;
            let order_id_str = evt
                .data
                .pointer("/object/metadata/order_id")
                .or_else(|| evt.data.pointer("/object/client_reference_id"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| AppError::validation("stripe event missing order_id metadata"))?;
            let order_id = Uuid::parse_str(order_id_str)
                .map_err(|_| AppError::validation("invalid order_id in stripe event"))?;
            let trade_no = evt
                .data
                .pointer("/object/id")
                .and_then(|v| v.as_str())
                .unwrap_or("stripe-unknown")
                .to_string();
            let event_id = evt
                .id
                .unwrap_or_else(|| format!("stripe-evt-{order_id}-{trade_no}"));
            let amount_total = evt
                .data
                .pointer("/object/amount_total")
                .and_then(|v| v.as_i64());
            let status = match evt.event_type.as_deref() {
                Some("checkout.session.completed")
                | Some("payment_intent.succeeded")
                | None => PaymentStatus::Success {
                    provider_trade_no: trade_no.clone(),
                    provider_event_id: Some(event_id.clone()),
                    paid_amount_minor: amount_total,
                },
                Some(other) => {
                    return Err(AppError::validation(format!(
                        "unsupported stripe event type: {other}"
                    )));
                }
            };
            return Ok(NotifyEvent {
                order_id,
                status,
                provider_trade_no: trade_no,
                provider_event_id: Some(event_id),
                paid_amount_minor: amount_total,
                paid_currency: evt
                    .data
                    .pointer("/object/currency")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_ascii_uppercase()),
                raw: serde_json::from_slice(body).unwrap_or(serde_json::Value::Null),
            });
        }

        #[derive(Deserialize)]
        struct SandboxBody {
            order_id: String,
            sig: String,
            #[serde(default)]
            trade_no: Option<String>,
            #[serde(default)]
            event_id: Option<String>,
            #[serde(default)]
            amount: Option<String>,
        }
        let parsed: SandboxBody = serde_json::from_slice(body).map_err(|e| {
            AppError::validation(format!("invalid stripe sandbox notify body: {e}"))
        })?;
        let order_id = Uuid::parse_str(&parsed.order_id)
            .map_err(|_| AppError::validation("invalid order_id"))?;
        if !self.verify_signature(order_id, &parsed.sig) {
            return Err(AppError::new(
                ErrorCode::Forbidden,
                "invalid stripe sandbox signature",
            ));
        }
        let paid_amount_minor = match parsed.amount.as_deref() {
            Some(a) => Some(parse_amount_to_minor(a)?),
            None => None,
        };
        let trade_no = parsed
            .trade_no
            .unwrap_or_else(|| format!("cs_test_{}", order_id.simple()));
        let event_id = parsed
            .event_id
            .unwrap_or_else(|| format!("stripe-evt-{order_id}-{trade_no}"));
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

// ── IAP sandbox provider (App Store / Play) ──────────────────────────────────

/// Store-kit style sandbox: client posts a purchase receipt token; server verifies
/// HMAC (sandbox) or later JWS/Play Developer API. Never credits wallet here.
#[derive(Debug, Clone)]
pub struct IapPayProvider {
    secret: String,
}

impl IapPayProvider {
    pub fn new(secret: impl Into<String>) -> Self {
        Self {
            secret: secret.into(),
        }
    }

    pub fn from_env() -> Self {
        let secret = std::env::var("PAY_IAP_SECRET")
            .or_else(|_| std::env::var("PAY_MOCK_SECRET"))
            .unwrap_or_else(|_| DEFAULT_PAY_MOCK_SECRET.into());
        Self::new(secret)
    }

    pub fn secret(&self) -> &str {
        &self.secret
    }

    pub fn sign_order(&self, order_id: Uuid) -> String {
        let mut mac = HmacSha256::new_from_slice(self.secret.as_bytes())
            .expect("HMAC key length");
        mac.update(b"iap:");
        mac.update(order_id.as_bytes());
        hex::encode(mac.finalize().into_bytes())
    }

    pub fn verify_signature(&self, order_id: Uuid, sig_hex: &str) -> bool {
        let expected = self.sign_order(order_id);
        expected.as_bytes().ct_eq(sig_hex.as_bytes()).into()
    }
}

#[async_trait]
impl PayProvider for IapPayProvider {
    fn channel(&self) -> PayChannel {
        PayChannel::Iap
    }

    async fn create_payment(
        &self,
        req: CreatePaymentRequest,
    ) -> Result<CreatePaymentResult, AppError> {
        let expires_at = Utc::now() + Duration::seconds(DEFAULT_ORDER_TTL_SECS);
        let product_id = req
            .extra
            .get("store_product_id")
            .and_then(|v| v.as_str())
            .unwrap_or("anylive.coins")
            .to_string();
        let raw = serde_json::json!({
            "order_id": req.order_id.to_string(),
            "store_product_id": product_id,
            "hint": "iap sandbox: POST /api/v1/webhooks/pay/iap with {order_id,sig,receipt}",
        });
        Ok(CreatePaymentResult {
            pay_mode: PayMode::Jsapi {
                params: serde_json::json!({
                    "platform": "store",
                    "product_id": product_id,
                    "order_id": req.order_id.to_string(),
                }),
            },
            provider_trade_no: Some(format!("iap-{}", req.order_id.simple())),
            raw,
            expires_at: Some(expires_at),
        })
    }

    async fn query_payment(&self, _order_id: Uuid) -> Result<PaymentStatus, AppError> {
        Ok(PaymentStatus::Pending)
    }

    async fn parse_and_verify_notify(
        &self,
        _headers: &http::HeaderMap,
        body: &[u8],
    ) -> Result<NotifyEvent, AppError> {
        #[derive(Deserialize)]
        struct IapBody {
            order_id: String,
            sig: String,
            #[serde(default)]
            receipt: Option<String>,
            #[serde(default)]
            transaction_id: Option<String>,
            #[serde(default)]
            event_id: Option<String>,
            #[serde(default)]
            amount: Option<String>,
        }
        let parsed: IapBody = serde_json::from_slice(body).map_err(|e| {
            AppError::validation(format!("invalid iap notify body: {e}"))
        })?;
        let order_id = Uuid::parse_str(&parsed.order_id)
            .map_err(|_| AppError::validation("invalid order_id"))?;
        if !self.verify_signature(order_id, &parsed.sig) {
            return Err(AppError::new(
                ErrorCode::Forbidden,
                "invalid iap sandbox signature",
            ));
        }
        // Sandbox: require a non-empty receipt token so clients exercise the path.
        let receipt = parsed.receipt.unwrap_or_default();
        if receipt.is_empty() {
            return Err(AppError::validation("iap receipt required"));
        }
        let paid_amount_minor = match parsed.amount.as_deref() {
            Some(a) => Some(parse_amount_to_minor(a)?),
            None => None,
        };
        let trade_no = parsed
            .transaction_id
            .unwrap_or_else(|| format!("iap-tx-{}", order_id.simple()));
        let event_id = parsed
            .event_id
            .unwrap_or_else(|| format!("iap-evt-{order_id}-{trade_no}"));
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

// ── Regional PSP sandboxes (Jeepay / EPay / TokenPay) ─────────────────────────
//
// Control-plane adapters: create returns QR/redirect; notify verifies HMAC.
// Real merchant APIs plug into the same PayProvider surface later.

fn hmac_hex(secret: &str, prefix: &[u8], order_id: Uuid) -> String {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC key length");
    mac.update(prefix);
    mac.update(order_id.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

fn ct_eq_hex(a: &str, b: &str) -> bool {
    a.as_bytes().ct_eq(b.as_bytes()).into()
}

/// Generic sandbox body used by regional PSPs in dogfood.
#[derive(Debug, Deserialize)]
struct RegionalNotifyBody {
    order_id: String,
    sig: String,
    #[serde(default)]
    trade_no: Option<String>,
    #[serde(default)]
    event_id: Option<String>,
    #[serde(default)]
    amount: Option<String>,
}

macro_rules! regional_pay_provider {
    ($name:ident, $channel:expr, $prefix:expr, $env_secret:expr, $create_mode:ident) => {
        #[derive(Debug, Clone)]
        pub struct $name {
            secret: String,
            public_base: String,
        }

        impl $name {
            pub fn new(secret: impl Into<String>, public_base: impl Into<String>) -> Self {
                Self {
                    secret: secret.into(),
                    public_base: public_base.into().trim_end_matches('/').to_string(),
                }
            }

            pub fn from_env() -> Self {
                let secret = std::env::var($env_secret)
                    .or_else(|_| std::env::var("PAY_MOCK_SECRET"))
                    .unwrap_or_else(|_| DEFAULT_PAY_MOCK_SECRET.into());
                let public_base = std::env::var("PAY_PUBLIC_BASE_URL")
                    .or_else(|_| std::env::var("API_PUBLIC_BASE_URL"))
                    .unwrap_or_else(|_| "http://localhost:8088".into());
                Self::new(secret, public_base)
            }

            pub fn secret(&self) -> &str {
                &self.secret
            }

            pub fn sign_order(&self, order_id: Uuid) -> String {
                hmac_hex(&self.secret, $prefix, order_id)
            }

            pub fn verify_signature(&self, order_id: Uuid, sig_hex: &str) -> bool {
                ct_eq_hex(&self.sign_order(order_id), sig_hex)
            }
        }

        #[async_trait]
        impl PayProvider for $name {
            fn channel(&self) -> PayChannel {
                $channel
            }

            async fn create_payment(
                &self,
                req: CreatePaymentRequest,
            ) -> Result<CreatePaymentResult, AppError> {
                let expires_at = Utc::now() + Duration::seconds(DEFAULT_ORDER_TTL_SECS);
                let trade = format!("{}-{}", $channel.as_str(), req.order_id.simple());
                let raw = serde_json::json!({
                    "order_id": req.order_id.to_string(),
                    "trade_no": trade,
                    "hint": format!(
                        "{} sandbox: POST /api/v1/webhooks/pay/{} with {{order_id,sig}}",
                        $channel.as_str(),
                        $channel.as_str()
                    ),
                });
                let pay_mode = match stringify!($create_mode) {
                    "qrcode" => PayMode::QrCode {
                        content: format!(
                            "{}://pay?order_id={}&amount={}",
                            $channel.as_str(),
                            req.order_id,
                            req.amount_minor
                        ),
                    },
                    _ => PayMode::Redirect {
                        url: format!(
                            "{}/pay/{}/checkout?order_id={}",
                            self.public_base,
                            $channel.as_str(),
                            req.order_id
                        ),
                    },
                };
                Ok(CreatePaymentResult {
                    pay_mode,
                    provider_trade_no: Some(trade),
                    raw,
                    expires_at: Some(expires_at),
                })
            }

            async fn query_payment(&self, _order_id: Uuid) -> Result<PaymentStatus, AppError> {
                Ok(PaymentStatus::Pending)
            }

            async fn parse_and_verify_notify(
                &self,
                _headers: &http::HeaderMap,
                body: &[u8],
            ) -> Result<NotifyEvent, AppError> {
                let parsed: RegionalNotifyBody = serde_json::from_slice(body).map_err(|e| {
                    AppError::validation(format!("{} notify body: {e}", $channel.as_str()))
                })?;
                let order_id = Uuid::parse_str(&parsed.order_id).map_err(|_| {
                    AppError::validation("invalid order_id in notify")
                })?;
                if !self.verify_signature(order_id, &parsed.sig) {
                    return Err(AppError::new(
                        ErrorCode::Forbidden,
                        format!("{} notify signature invalid", $channel.as_str()),
                    ));
                }
                let paid_amount_minor = match parsed.amount.as_deref() {
                    Some(a) => Some(parse_amount_to_minor(a)?),
                    None => None,
                };
                let trade_no = parsed
                    .trade_no
                    .unwrap_or_else(|| format!("{}-tx-{}", $channel.as_str(), order_id.simple()));
                let event_id = parsed.event_id.unwrap_or_else(|| {
                    format!("{}-evt-{order_id}-{trade_no}", $channel.as_str())
                });
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
    };
}

regional_pay_provider!(
    JeepayPayProvider,
    PayChannel::Jeepay,
    b"jeepay:",
    "PAY_JEEPAY_SECRET",
    qrcode
);
regional_pay_provider!(
    EpayPayProvider,
    PayChannel::Epay,
    b"epay:",
    "PAY_EPAY_SECRET",
    redirect
);
regional_pay_provider!(
    TokenpayPayProvider,
    PayChannel::Tokenpay,
    b"tokenpay:",
    "PAY_TOKENPAY_SECRET",
    qrcode
);

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

    /// Mark pending/paying orders past `expires_at` as `expired`. Returns count expired.
    pub async fn expire_stale_orders(&self, now: DateTime<Utc>) -> u64 {
        let mut g = self.inner.lock().await;
        let mut n = 0u64;
        for order in g.orders.values_mut() {
            if !matches!(
                order.status,
                PayOrderStatus::Pending | PayOrderStatus::Paying
            ) {
                continue;
            }
            let Some(exp) = order.expires_at else {
                continue;
            };
            if now > exp {
                order.status = PayOrderStatus::Expired;
                order.updated_at = now;
                n += 1;
            }
        }
        n
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
                PayChannel::Stripe => {
                    reg.register(Arc::new(StripePayProvider::from_env()));
                }
                PayChannel::Iap => {
                    reg.register(Arc::new(IapPayProvider::from_env()));
                }
                PayChannel::Jeepay => {
                    reg.register(Arc::new(JeepayPayProvider::from_env()));
                }
                PayChannel::Epay => {
                    reg.register(Arc::new(EpayPayProvider::from_env()));
                }
                PayChannel::Tokenpay => {
                    reg.register(Arc::new(TokenpayPayProvider::from_env()));
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

    /// Local/dev helper: mock + Stripe + IAP sandboxes with a shared HMAC secret.
    pub fn sandbox_channels(secret: &str) -> Self {
        let mut reg = Self::new();
        reg.register(Arc::new(MockPayProvider::new(
            secret,
            "http://localhost:8088",
        )));
        reg.register(Arc::new(StripePayProvider::new(
            secret,
            "http://localhost:8088",
            "",
        )));
        reg.register(Arc::new(IapPayProvider::new(secret)));
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
    /// Expire pending/paying orders past `expires_at`. Returns how many rows flipped.
    async fn expire_stale_orders(&self, now: DateTime<Utc>) -> u64;
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
    async fn expire_stale_orders(&self, now: DateTime<Utc>) -> u64 {
        MemoryPayStore::expire_stale_orders(self, now).await
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
        assert_eq!(PayChannel::parse("stripe"), Some(PayChannel::Stripe));
        assert_eq!(PayChannel::parse("iap"), Some(PayChannel::Iap));
        assert_eq!(PayChannel::parse("app_store"), Some(PayChannel::Iap));
        assert!(PayChannel::parse("bitcoin").is_none());
    }

    #[tokio::test]
    async fn stripe_sandbox_create_and_verify_notify() {
        let stripe = StripePayProvider::new("stripe-secret", "http://localhost:8088", "");
        let order_id = Uuid::new_v4();
        let result = stripe
            .create_payment(CreatePaymentRequest {
                order_id,
                user_id: UserId(Uuid::new_v4()),
                amount_minor: 99,
                currency: "USD".into(),
                coins: 100,
                subject: "100 Coins".into(),
                notify_url: "http://localhost/hook".into(),
                return_url: None,
                client_ip: None,
                extra: serde_json::Value::Null,
            })
            .await
            .unwrap();
        assert!(matches!(result.pay_mode, PayMode::Redirect { .. }));
        let sig = stripe.sign_order(order_id);
        let body = serde_json::json!({
            "order_id": order_id.to_string(),
            "sig": sig,
            "amount": "0.99",
        });
        let ev = stripe
            .parse_and_verify_notify(&http::HeaderMap::new(), &serde_json::to_vec(&body).unwrap())
            .await
            .unwrap();
        assert_eq!(ev.order_id, order_id);
        assert!(matches!(ev.status, PaymentStatus::Success { .. }));
    }

    #[tokio::test]
    async fn stripe_signature_header_path() {
        let stripe = StripePayProvider::new("whsec_test", "http://localhost:8088", "");
        let order_id = Uuid::new_v4();
        let body = serde_json::json!({
            "id": "evt_test_1",
            "type": "checkout.session.completed",
            "data": {
                "object": {
                    "id": "cs_test_abc",
                    "amount_total": 99,
                    "currency": "usd",
                    "metadata": { "order_id": order_id.to_string() }
                }
            }
        });
        let body_bytes = serde_json::to_vec(&body).unwrap();
        let t = "1710000000";
        let mut mac = HmacSha256::new_from_slice(b"whsec_test").unwrap();
        mac.update(t.as_bytes());
        mac.update(b".");
        mac.update(&body_bytes);
        let v1 = hex::encode(mac.finalize().into_bytes());
        let mut headers = http::HeaderMap::new();
        headers.insert(
            "stripe-signature",
            format!("t={t},v1={v1}").parse().unwrap(),
        );
        let ev = stripe
            .parse_and_verify_notify(&headers, &body_bytes)
            .await
            .unwrap();
        assert_eq!(ev.order_id, order_id);
        assert_eq!(ev.provider_trade_no, "cs_test_abc");
    }

    #[tokio::test]
    async fn iap_sandbox_requires_receipt_and_sig() {
        let iap = IapPayProvider::new("iap-secret");
        let order_id = Uuid::new_v4();
        let result = iap
            .create_payment(CreatePaymentRequest {
                order_id,
                user_id: UserId(Uuid::new_v4()),
                amount_minor: 99,
                currency: "USD".into(),
                coins: 100,
                subject: "100 Coins".into(),
                notify_url: "http://localhost/hook".into(),
                return_url: None,
                client_ip: None,
                extra: serde_json::json!({"store_product_id": "coins_100"}),
            })
            .await
            .unwrap();
        assert!(matches!(result.pay_mode, PayMode::Jsapi { .. }));

        let sig = iap.sign_order(order_id);
        let bad = serde_json::json!({
            "order_id": order_id.to_string(),
            "sig": sig,
            "receipt": "",
        });
        let err = iap
            .parse_and_verify_notify(&http::HeaderMap::new(), &serde_json::to_vec(&bad).unwrap())
            .await
            .unwrap_err();
        assert_eq!(err.code, ErrorCode::Validation);

        let ok = serde_json::json!({
            "order_id": order_id.to_string(),
            "sig": sig,
            "receipt": "base64-fake-receipt",
            "transaction_id": "1000000123",
        });
        let ev = iap
            .parse_and_verify_notify(&http::HeaderMap::new(), &serde_json::to_vec(&ok).unwrap())
            .await
            .unwrap();
        assert_eq!(ev.order_id, order_id);
        assert_eq!(ev.provider_trade_no, "1000000123");
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

    #[tokio::test]
    async fn expire_stale_orders_flips_pending() {
        let store = MemoryPayStore::new();
        store.seed_default_products().await;
        let product = &store.list_active_products().await[0];
        let user = UserId(Uuid::new_v4());
        let o = store
            .create_order(user, product, PayChannel::Mock, None)
            .await
            .unwrap();
        // Force expires_at into the past via mark_paying.
        let past = Utc::now() - Duration::seconds(60);
        store
            .mark_paying(
                o.id,
                &PayMode::None,
                None,
                serde_json::Value::Null,
                Some(past),
            )
            .await
            .unwrap();
        let n = store.expire_stale_orders(Utc::now()).await;
        assert_eq!(n, 1);
        let got = store.get_order(o.id).await.unwrap();
        assert_eq!(got.status, PayOrderStatus::Expired);
        // Second run is a no-op.
        assert_eq!(store.expire_stale_orders(Utc::now()).await, 0);
    }

    #[tokio::test]
    async fn jeepay_epay_tokenpay_sandbox_notify() {
        for (provider, channel) in [
            (
                Arc::new(JeepayPayProvider::new("jeepay-s", "http://localhost:8088"))
                    as Arc<dyn PayProvider>,
                PayChannel::Jeepay,
            ),
            (
                Arc::new(EpayPayProvider::new("epay-s", "http://localhost:8088"))
                    as Arc<dyn PayProvider>,
                PayChannel::Epay,
            ),
            (
                Arc::new(TokenpayPayProvider::new("token-s", "http://localhost:8088"))
                    as Arc<dyn PayProvider>,
                PayChannel::Tokenpay,
            ),
        ] {
            assert_eq!(provider.channel(), channel);
            let order_id = Uuid::new_v4();
            let created = provider
                .create_payment(CreatePaymentRequest {
                    order_id,
                    user_id: UserId(Uuid::new_v4()),
                    amount_minor: 600,
                    currency: "CNY".into(),
                    coins: 100,
                    subject: "coins".into(),
                    notify_url: "http://localhost/n".into(),
                    return_url: None,
                    client_ip: None,
                    extra: serde_json::json!({}),
                })
                .await
                .unwrap();
            assert!(created.provider_trade_no.is_some());

            let sig = match channel {
                PayChannel::Jeepay => JeepayPayProvider::new("jeepay-s", "http://x").sign_order(order_id),
                PayChannel::Epay => EpayPayProvider::new("epay-s", "http://x").sign_order(order_id),
                PayChannel::Tokenpay => {
                    TokenpayPayProvider::new("token-s", "http://x").sign_order(order_id)
                }
                _ => unreachable!(),
            };
            let body = serde_json::json!({
                "order_id": order_id.to_string(),
                "sig": sig,
            });
            let ev = provider
                .parse_and_verify_notify(&http::HeaderMap::new(), &serde_json::to_vec(&body).unwrap())
                .await
                .unwrap();
            assert_eq!(ev.order_id, order_id);
            assert!(matches!(ev.status, PaymentStatus::Success { .. }));

            // Bad sig rejected
            let bad = serde_json::json!({
                "order_id": order_id.to_string(),
                "sig": "00",
            });
            let err = provider
                .parse_and_verify_notify(&http::HeaderMap::new(), &serde_json::to_vec(&bad).unwrap())
                .await
                .unwrap_err();
            assert_eq!(err.code, ErrorCode::Forbidden);
        }
    }
}
