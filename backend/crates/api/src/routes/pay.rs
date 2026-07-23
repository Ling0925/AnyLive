//! Pay control-plane HTTP handlers: channels, products, orders, webhooks.

use std::sync::Arc;

use anylive_pay::{
    wallet_reference_for_order, CreatePaymentRequest, MockPayProvider, PayChannel,
    PayMode, PayOrder, PayOrderStatus, PayProduct, PayStore, PaymentStatus,
};
use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::auth_user::AuthUser;
use crate::error::ApiError;
use crate::state::AppState;

// ── DTOs ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, ToSchema)]
pub struct PayChannelDto {
    pub id: String,
    pub title: String,
    pub pay_modes: Vec<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PayChannelListResponse {
    pub items: Vec<PayChannelDto>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PayProductDto {
    pub id: String,
    pub sku: String,
    pub title: String,
    pub coins: i64,
    pub amount: String,
    pub currency: String,
}

impl From<PayProduct> for PayProductDto {
    fn from(p: PayProduct) -> Self {
        let amount = p.amount_display();
        Self {
            id: p.id.to_string(),
            sku: p.sku,
            title: p.title,
            coins: p.coins,
            amount,
            currency: p.currency,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PayProductListResponse {
    pub items: Vec<PayProductDto>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreatePayOrderBody {
    pub product_id: String,
    pub channel: String,
    #[serde(default)]
    pub client_request_id: Option<String>,
    #[serde(default)]
    pub return_url: Option<String>,
    #[serde(default)]
    pub extra: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PayOrderDto {
    pub id: String,
    pub status: String,
    pub coins: i64,
    pub amount: String,
    pub currency: String,
    pub channel: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pay_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pay_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qr_content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jsapi_params: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mock_hint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paid_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credited_at: Option<String>,
}

impl PayOrderDto {
    fn from_order(order: &PayOrder, pay_mode: Option<&PayMode>) -> Self {
        let mut pay_url = None;
        let mut qr_content = None;
        let mut jsapi_params = None;
        let mut mock_hint = None;
        let mode_name = order.pay_mode.clone();
        if let Some(mode) = pay_mode {
            match mode {
                PayMode::Redirect { url } => pay_url = Some(url.clone()),
                PayMode::QrCode { content } => qr_content = Some(content.clone()),
                PayMode::Jsapi { params } => jsapi_params = Some(params.clone()),
                PayMode::MockComplete { hint } => mock_hint = Some(hint.clone()),
                PayMode::None => {}
            }
        } else if let Some(raw_hint) = order.pay_payload.get("hint").and_then(|v| v.as_str()) {
            mock_hint = Some(raw_hint.to_string());
        }
        // Reconstruct from payload for GET order when mode not in memory.
        if pay_url.is_none() {
            if let Some(u) = order.pay_payload.get("url").and_then(|v| v.as_str()) {
                pay_url = Some(u.to_string());
            }
        }
        if qr_content.is_none() {
            if let Some(c) = order.pay_payload.get("content").and_then(|v| v.as_str()) {
                qr_content = Some(c.to_string());
            }
        }
        Self {
            id: order.id.to_string(),
            status: order.status.as_str().into(),
            coins: order.coins,
            amount: order.amount_display(),
            currency: order.currency.clone(),
            channel: order.channel.as_str().into(),
            pay_mode: mode_name,
            pay_url,
            qr_content,
            jsapi_params,
            mock_hint,
            expires_at: order.expires_at.map(|t| t.to_rfc3339()),
            paid_at: order.paid_at.map(|t| t.to_rfc3339()),
            credited_at: order.credited_at.map(|t| t.to_rfc3339()),
        }
    }
}

fn pay_modes_for_channel(channel: PayChannel) -> Vec<String> {
    match channel {
        PayChannel::Mock => vec!["mock_complete".into()],
        PayChannel::Jeepay => vec!["qrcode".into(), "redirect".into(), "jsapi".into()],
        PayChannel::Epay => vec!["redirect".into(), "qrcode".into()],
        PayChannel::Tokenpay => vec!["qrcode".into()],
        PayChannel::Stripe => vec!["redirect".into()],
        PayChannel::Iap => vec!["jsapi".into()],
    }
}

// ── Routes ───────────────────────────────────────────────────────────────────

/// GET /api/v1/pay/channels
#[utoipa::path(get, path = "/api/v1/pay/channels", tag = "pay", responses((status = 200, body = PayChannelListResponse)))]
pub async fn list_pay_channels(
    State(state): State<Arc<AppState>>,
) -> Result<Json<PayChannelListResponse>, ApiError> {
    let items = state
        .pay_registry
        .enabled_channels()
        .into_iter()
        .map(|c| PayChannelDto {
            id: c.as_str().into(),
            title: c.title().into(),
            pay_modes: pay_modes_for_channel(c),
        })
        .collect();
    Ok(Json(PayChannelListResponse { items }))
}

/// GET /api/v1/pay/products
#[utoipa::path(get, path = "/api/v1/pay/products", tag = "pay", responses((status = 200, body = PayProductListResponse)))]
pub async fn list_pay_products(
    State(state): State<Arc<AppState>>,
) -> Result<Json<PayProductListResponse>, ApiError> {
    let items = state.pay.list_active_products().await;
    Ok(Json(PayProductListResponse {
        items: items.into_iter().map(PayProductDto::from).collect(),
    }))
}

/// POST /api/v1/pay/orders
#[utoipa::path(
    post,
    path = "/api/v1/pay/orders",
    tag = "pay",
    security(("bearerAuth" = [])),
    request_body = CreatePayOrderBody,
    responses((status = 201, body = PayOrderDto), (status = 200, body = PayOrderDto))
)]
pub async fn create_pay_order(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(body): Json<CreatePayOrderBody>,
) -> Result<(StatusCode, Json<PayOrderDto>), ApiError> {
    let channel = PayChannel::parse(&body.channel).ok_or_else(|| {
        ApiError(anylive_common::AppError::validation("unknown pay channel"))
    })?;
    if channel != PayChannel::Mock {
        state.features.require_real_pay().map_err(ApiError::from)?;
    }
    let provider = state.pay_registry.get(channel).ok_or_else(|| {
        ApiError(anylive_common::AppError::new(
            anylive_common::ErrorCode::ForbiddenPolicy,
            format!("pay channel not enabled: {}", channel.as_str()),
        ))
    })?;

    let product_id = Uuid::parse_str(&body.product_id)
        .map_err(|_| ApiError(anylive_common::AppError::validation("invalid product_id")))?;
    let product = state
        .pay
        .get_product(product_id)
        .await
        .ok_or_else(|| ApiError(anylive_common::AppError::not_found("product not found")))?;
    if !product.active {
        return Err(ApiError(anylive_common::AppError::validation(
            "product inactive",
        )));
    }

    // Idempotent create by client_request_id
    let existing = if let Some(ref crid) = body.client_request_id {
        state.pay.find_by_client_request(user.user_id, crid).await
    } else {
        None
    };

    let (order, is_replay) = if let Some(existing) = existing {
        // If already past pending, return as-is without re-calling provider.
        if existing.status != PayOrderStatus::Pending {
            return Ok((
                StatusCode::OK,
                Json(PayOrderDto::from_order(&existing, None)),
            ));
        }
        (existing, true)
    } else {
        let order = state
            .pay
            .create_order(
                user.user_id,
                &product,
                channel,
                body.client_request_id.clone(),
            )
            .await
            .map_err(ApiError::from)?;
        (order, false)
    };

    // If already paying/paid from race on same client_request, return.
    if order.status == PayOrderStatus::Paying
        || order.status.is_paid_or_later()
        || order.status == PayOrderStatus::Credited
    {
        return Ok((
            if is_replay {
                StatusCode::OK
            } else {
                StatusCode::CREATED
            },
            Json(PayOrderDto::from_order(&order, None)),
        ));
    }

    let notify_url = format!(
        "{}/api/v1/webhooks/pay/{}",
        state.pay_public_base.trim_end_matches('/'),
        channel.as_str()
    );
    let create_req = CreatePaymentRequest {
        order_id: order.id,
        user_id: user.user_id,
        amount_minor: order.amount_minor,
        currency: order.currency.clone(),
        coins: order.coins,
        subject: product.title.clone(),
        notify_url,
        return_url: body.return_url.clone(),
        client_ip: None,
        extra: body.extra.unwrap_or(serde_json::Value::Null),
    };

    let result = provider
        .create_payment(create_req)
        .await
        .map_err(ApiError::from)?;

    // Persist pay payload without secrets.
    let mut payload = result.raw.clone();
    if let serde_json::Value::Object(ref mut map) = payload {
        match &result.pay_mode {
            PayMode::Redirect { url } => {
                map.insert("url".into(), serde_json::Value::String(url.clone()));
            }
            PayMode::QrCode { content } => {
                map.insert(
                    "content".into(),
                    serde_json::Value::String(content.clone()),
                );
            }
            PayMode::MockComplete { hint } => {
                map.insert("hint".into(), serde_json::Value::String(hint.clone()));
            }
            PayMode::Jsapi { params } => {
                map.insert("jsapi".into(), params.clone());
            }
            PayMode::None => {}
        }
    }

    let order = state
        .pay
        .mark_paying(
            order.id,
            &result.pay_mode,
            result.provider_trade_no,
            payload,
            result.expires_at,
        )
        .await
        .map_err(ApiError::from)?;

    let status = if is_replay {
        StatusCode::OK
    } else {
        StatusCode::CREATED
    };
    Ok((
        status,
        Json(PayOrderDto::from_order(&order, Some(&result.pay_mode))),
    ))
}

/// GET /api/v1/pay/orders/{id}
#[utoipa::path(
    get,
    path = "/api/v1/pay/orders/{id}",
    tag = "pay",
    security(("bearerAuth" = [])),
    responses((status = 200, body = PayOrderDto))
)]
pub async fn get_pay_order(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<PayOrderDto>, ApiError> {
    let order_id = Uuid::parse_str(&id)
        .map_err(|_| ApiError(anylive_common::AppError::validation("invalid order id")))?;
    let order = state
        .pay
        .get_order(order_id)
        .await
        .ok_or_else(|| ApiError(anylive_common::AppError::not_found("pay order not found")))?;
    if order.user_id != user.user_id {
        return Err(ApiError(anylive_common::AppError::new(
            anylive_common::ErrorCode::Forbidden,
            "not your order",
        )));
    }
    Ok(Json(PayOrderDto::from_order(&order, None)))
}

/// Shared webhook orchestration: verify → bind order/channel → mark paid → credit → record event.
pub async fn handle_pay_notify(
    state: Arc<AppState>,
    channel: PayChannel,
    headers: HeaderMap,
    body: Bytes,
) -> Result<StatusCode, ApiError> {
    let provider = state.pay_registry.get(channel).ok_or_else(|| {
        ApiError(anylive_common::AppError::new(
            anylive_common::ErrorCode::NotFound,
            format!("pay channel not enabled: {}", channel.as_str()),
        ))
    })?;

    let event = provider
        .parse_and_verify_notify(&headers, &body)
        .await
        .map_err(ApiError::from)?;

    // Only success path credits.
    let (trade_no, event_id, paid_amount) = match &event.status {
        PaymentStatus::Success {
            provider_trade_no,
            provider_event_id,
            paid_amount_minor,
        } => (
            provider_trade_no.clone(),
            provider_event_id.clone(),
            *paid_amount_minor,
        ),
        PaymentStatus::Failed { reason } => {
            tracing::info!(%reason, order_id = %event.order_id, "pay notify failed status");
            return Ok(StatusCode::OK);
        }
        PaymentStatus::Pending | PaymentStatus::Closed => {
            return Ok(StatusCode::OK);
        }
    };

    // Bind notify to an existing order on this channel before any side effects.
    let existing = state
        .pay
        .get_order(event.order_id)
        .await
        .ok_or_else(|| ApiError(anylive_common::AppError::not_found("pay order not found")))?;
    if existing.channel != channel {
        return Err(ApiError(anylive_common::AppError::new(
            anylive_common::ErrorCode::Forbidden,
            "pay channel mismatch for order",
        )));
    }
    if existing.status == PayOrderStatus::Credited {
        // Still record event for dedupe observability (best-effort).
        if let Some(ref eid) = event_id {
            let _ = state
                .pay
                .record_webhook_event(channel, eid, event.order_id)
                .await;
        }
        return Ok(StatusCode::OK);
    }

    // Prefer provider-reported amount; when omitted (mock), enforce order snapshot.
    let amount_to_check = paid_amount
        .or(event.paid_amount_minor)
        .unwrap_or(existing.amount_minor);

    let (order, _already) = state
        .pay
        .mark_paid(
            event.order_id,
            trade_no,
            event_id.clone(),
            Some(amount_to_check),
        )
        .await
        .map_err(ApiError::from)?;

    if order.status == PayOrderStatus::Credited {
        return Ok(StatusCode::OK);
    }

    credit_order(&state, &order).await?;

    // Record event only after successful credit so crashes can be retried.
    if let Some(ref eid) = event_id {
        let _ = state
            .pay
            .record_webhook_event(channel, eid, event.order_id)
            .await;
    }
    Ok(StatusCode::OK)
}

async fn credit_order(state: &AppState, order: &PayOrder) -> Result<(), ApiError> {
    // Re-fetch to handle concurrent credit.
    let order = state
        .pay
        .get_order(order.id)
        .await
        .ok_or_else(|| ApiError(anylive_common::AppError::not_found("pay order not found")))?;
    if order.status == PayOrderStatus::Credited {
        return Ok(());
    }
    if order.status != PayOrderStatus::Paid {
        return Err(ApiError(anylive_common::AppError::new(
            anylive_common::ErrorCode::Conflict,
            format!(
                "order not ready to credit (status={})",
                order.status.as_str()
            ),
        )));
    }

    let reference = wallet_reference_for_order(order.id);
    state
        .wallet
        .credit_topup(order.user_id, order.coins, reference)
        .await
        .map_err(ApiError::from)?;
    state
        .pay
        .mark_credited(order.id)
        .await
        .map_err(ApiError::from)?;
    tracing::info!(
        order_id = %order.id,
        user_id = %order.user_id.0,
        coins = order.coins,
        "pay order credited"
    );
    Ok(())
}


/// POST /api/v1/pay/orders/{id}/sandbox-complete
///
/// Authenticated sandbox helper: signs and posts the mock webhook server-side so
/// clients never need `PAY_MOCK_SECRET`. Only available when mock channel is enabled.
#[utoipa::path(
    post,
    path = "/api/v1/pay/orders/{id}/sandbox-complete",
    tag = "pay",
    security(("bearerAuth" = [])),
    responses((status = 200, body = PayOrderDto))
)]
pub async fn sandbox_complete_pay_order(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<PayOrderDto>, ApiError> {
    if state.pay_registry.get(PayChannel::Mock).is_none() {
        return Err(ApiError(anylive_common::AppError::new(
            anylive_common::ErrorCode::ForbiddenPolicy,
            "mock pay channel not enabled",
        )));
    }
    let order_id = Uuid::parse_str(&id)
        .map_err(|_| ApiError(anylive_common::AppError::validation("invalid order id")))?;
    let order = state
        .pay
        .get_order(order_id)
        .await
        .ok_or_else(|| ApiError(anylive_common::AppError::not_found("pay order not found")))?;
    if order.user_id != user.user_id {
        return Err(ApiError(anylive_common::AppError::new(
            anylive_common::ErrorCode::Forbidden,
            "not your order",
        )));
    }
    if order.channel != PayChannel::Mock {
        return Err(ApiError(anylive_common::AppError::validation(
            "sandbox-complete only for mock channel orders",
        )));
    }
    if order.status == PayOrderStatus::Credited {
        return Ok(Json(PayOrderDto::from_order(&order, None)));
    }

    // Bound free-mint abuse on shared dogfood (production forbids mock entirely).
    state
        .pay_sandbox_limiter
        .check(&format!("pay-sandbox:{}", user.user_id.0))
        .await
        .map_err(ApiError::from)?;

    let secret = state.pay_mock_secret.as_deref().ok_or_else(|| {
        ApiError(anylive_common::AppError::new(
            anylive_common::ErrorCode::ForbiddenPolicy,
            "mock pay secret not configured",
        ))
    })?;
    let mock = MockPayProvider::new(secret, state.pay_public_base.clone());
    let sig = mock.sign_order(order_id);
    let body = serde_json::json!({
        "order_id": order_id.to_string(),
        "sig": sig,
        "amount": order.amount_display(),
    });
    let bytes = Bytes::from(serde_json::to_vec(&body).map_err(|e| {
        ApiError(anylive_common::AppError::new(
            anylive_common::ErrorCode::Internal,
            format!("serialize mock notify: {e}"),
        ))
    })?);
    handle_pay_notify(state.clone(), PayChannel::Mock, HeaderMap::new(), bytes).await?;

    let updated = state
        .pay
        .get_order(order_id)
        .await
        .ok_or_else(|| ApiError(anylive_common::AppError::not_found("pay order not found")))?;
    Ok(Json(PayOrderDto::from_order(&updated, None)))
}

/// POST /api/v1/webhooks/pay/mock
pub async fn pay_webhook_mock(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<StatusCode, ApiError> {
    handle_pay_notify(state, PayChannel::Mock, headers, body).await
}

/// POST /api/v1/webhooks/pay/jeepay — Jeepay HMAC sandbox notify.
pub async fn pay_webhook_jeepay(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<StatusCode, ApiError> {
    handle_pay_notify(state, PayChannel::Jeepay, headers, body).await
}

/// POST /api/v1/webhooks/pay/epay — EPay HMAC sandbox notify.
pub async fn pay_webhook_epay(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<StatusCode, ApiError> {
    handle_pay_notify(state, PayChannel::Epay, headers, body).await
}

/// POST /api/v1/webhooks/pay/tokenpay — TokenPay HMAC sandbox notify.
pub async fn pay_webhook_tokenpay(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<StatusCode, ApiError> {
    handle_pay_notify(state, PayChannel::Tokenpay, headers, body).await
}

/// POST /api/v1/webhooks/pay/stripe — Stripe Checkout / PaymentIntent notify.
pub async fn pay_webhook_stripe(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<StatusCode, ApiError> {
    handle_pay_notify(state, PayChannel::Stripe, headers, body).await
}

/// POST /api/v1/webhooks/pay/iap — App Store / Play sandbox receipt notify.
pub async fn pay_webhook_iap(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<StatusCode, ApiError> {
    handle_pay_notify(state, PayChannel::Iap, headers, body).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    async fn body_json(res: axum::response::Response) -> serde_json::Value {
        let bytes = res.into_body().collect().await.unwrap().to_bytes();
        if bytes.is_empty() {
            return serde_json::Value::Null;
        }
        serde_json::from_slice(&bytes).unwrap()
    }

    async fn login_token(app: axum::Router, email: &str) -> String {
        let _ = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/auth/otp/send")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(r#"{{"email":"{email}"}}"#)))
                    .unwrap(),
            )
            .await
            .unwrap();
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/auth/otp/verify")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        r#"{{"email":"{email}","code":"123456"}}"#
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let json = body_json(res).await;
        json["access_token"].as_str().unwrap().to_string()
    }

    #[tokio::test]
    async fn mock_pay_order_webhook_credits_wallet_idempotently() {
        let state = AppState::dev_ready().await;
        // Ensure mock channel registered (dev() does).
        let app = crate::build_app_with_state(state.clone());
        let token = login_token(app.clone(), "pay@example.com").await;

        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/pay/products")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let products = body_json(res).await;
        let product_id = products["items"][0]["id"].as_str().unwrap();
        let coins = products["items"][0]["coins"].as_i64().unwrap();

        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/pay/orders")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {token}"))
                    .body(Body::from(format!(
                        r#"{{"product_id":"{product_id}","channel":"mock","client_request_id":"cr-1"}}"#
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert!(
            res.status() == StatusCode::CREATED || res.status() == StatusCode::OK,
            "status={}",
            res.status()
        );
        let order_json = body_json(res).await;
        let order_id = order_json["id"].as_str().unwrap().to_string();
        assert_eq!(order_json["status"], "paying");
        assert_eq!(order_json["channel"], "mock");

        // Sign with the same secret used by AppState::dev mock registry.
        let mock_provider = anylive_pay::MockPayProvider::new(
            "anylive-dev-pay-mock-secret-change-me",
            "http://localhost:8088",
        );
        let sig = mock_provider.sign_order(Uuid::parse_str(&order_id).unwrap());

        let body = serde_json::json!({
            "order_id": order_id,
            "sig": sig,
        });
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/webhooks/pay/mock")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK, "webhook: {:?}", body_json(res).await);

        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/wallet")
                    .header("authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let wallet = body_json(res).await;
        assert_eq!(wallet["balance"].as_i64().unwrap(), coins);

        // Replay webhook — balance unchanged.
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/webhooks/pay/mock")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/wallet")
                    .header("authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let wallet2 = body_json(res).await;
        assert_eq!(wallet2["balance"].as_i64().unwrap(), coins);

        // Order status credited
        let res = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/v1/pay/orders/{order_id}"))
                    .header("authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let order2 = body_json(res).await;
        assert_eq!(order2["status"], "credited");
    }

    #[tokio::test]
    async fn sandbox_complete_credits_without_client_secret() {
        let state = AppState::dev_ready().await;
        let app = crate::build_app_with_state(state);
        let token = login_token(app.clone(), "sandbox@example.com").await;

        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/pay/products")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let products = body_json(res).await;
        let product_id = products["items"][0]["id"].as_str().unwrap();
        let coins = products["items"][0]["coins"].as_i64().unwrap();

        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/pay/orders")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {token}"))
                    .body(Body::from(format!(
                        r#"{{"product_id":"{product_id}","channel":"mock"}}"#
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert!(res.status().is_success());
        let order = body_json(res).await;
        let order_id = order["id"].as_str().unwrap();

        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/v1/pay/orders/{order_id}/sandbox-complete"))
                    .header("authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK, "{:?}", body_json(res).await);
        let done = body_json(res).await;
        assert_eq!(done["status"], "credited");

        let res = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/wallet")
                    .header("authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let wallet = body_json(res).await;
        assert_eq!(wallet["balance"].as_i64().unwrap(), coins);
    }

    #[tokio::test]
    async fn stripe_and_iap_webhook_credit_wallet() {
        use anylive_pay::{IapPayProvider, StripePayProvider};

        let state = AppState::dev_ready().await;
        let app = crate::build_app_with_state(state.clone());
        let token = login_token(app.clone(), "stripe-iap@example.com").await;

        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/pay/channels")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let channels = body_json(res).await;
        let ids: Vec<&str> = channels["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["id"].as_str().unwrap())
            .collect();
        assert!(ids.contains(&"stripe"));
        assert!(ids.contains(&"iap"));

        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/pay/products")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let products = body_json(res).await;
        let product_id = products["items"][0]["id"].as_str().unwrap().to_string();
        let coins = products["items"][0]["coins"].as_i64().unwrap();
        let amount = products["items"][0]["amount"].as_str().unwrap().to_string();

        // Stripe order + sandbox webhook
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/pay/orders")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {token}"))
                    .body(Body::from(format!(
                        r#"{{"product_id":"{product_id}","channel":"stripe","client_request_id":"stripe-1"}}"#
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert!(res.status().is_success(), "{:?}", body_json(res).await);
        let order = body_json(res).await;
        let order_id = order["id"].as_str().unwrap().to_string();
        assert_eq!(order["channel"], "stripe");
        assert!(order["pay_url"].as_str().is_some());

        let stripe = StripePayProvider::new(
            "anylive-dev-pay-mock-secret-change-me",
            "http://localhost:8088",
            "",
        );
        let sig = stripe.sign_order(Uuid::parse_str(&order_id).unwrap());
        let body = serde_json::json!({
            "order_id": order_id,
            "sig": sig,
            "amount": amount,
        });
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/webhooks/pay/stripe")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert!(res.status().is_success());

        // IAP order + receipt webhook
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/pay/orders")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {token}"))
                    .body(Body::from(format!(
                        r#"{{"product_id":"{product_id}","channel":"iap","client_request_id":"iap-1"}}"#
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert!(res.status().is_success(), "{:?}", body_json(res).await);
        let order = body_json(res).await;
        let iap_order_id = order["id"].as_str().unwrap().to_string();
        assert_eq!(order["channel"], "iap");

        let iap = IapPayProvider::new("anylive-dev-pay-mock-secret-change-me");
        let sig = iap.sign_order(Uuid::parse_str(&iap_order_id).unwrap());
        let body = serde_json::json!({
            "order_id": iap_order_id,
            "sig": sig,
            "receipt": "sandbox-receipt-token",
            "transaction_id": "tx-iap-1",
            "amount": amount,
        });
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/webhooks/pay/iap")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert!(res.status().is_success());

        let res = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/wallet")
                    .header("authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let wallet = body_json(res).await;
        assert_eq!(wallet["balance"].as_i64().unwrap(), coins * 2);
    }
}
