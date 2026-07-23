# Webhook & domain-event contracts (WBS E1.4)

Machine-readable shapes for media/payment inbound webhooks and NATS domain events.
HTTP routes remain the source of runtime behavior (`contracts/openapi/openapi.yaml`);
this folder freezes **payload examples** and **JSON Schema** for workers / partners.

## Layout

| Path | Purpose |
|---|---|
| `../events/gift.sent.v1.json` | NATS `gift.sent` envelope (`anylive.gift.sent.v1`) |
| `pay.mock.v1.json` | Mock pay sandbox notify body |
| `pay.hmac.v1.json` | Shared HMAC sandbox notify used by Stripe/IAP/Jeepay/EPay/TokenPay adapters |
| `srs.on_publish.v1.json` | SRS HTTP callback body (subset used by AnyLive) |

## Auth headers (runtime)

| Channel | Verification |
|---|---|
| SRS | `X-AnyLive-Webhook-Secret` or `?secret=` vs `SRS_WEBHOOK_SECRET` (empty = open, local only) |
| Pay mock | JSON `{order_id,sig}` — `sig` = hex HMAC-SHA256(`order_id`, `PAY_MOCK_SECRET`) |
| Pay Stripe sandbox | body `{order_id,sig}` **or** `Stripe-Signature` style HMAC of order id |
| Pay IAP / Jeepay / EPay / TokenPay | body `{order_id,sig}` (+ optional receipt fields) |

Credit path is always `pay:{order_id}` ledger ref (idempotent).

## NATS

- Subject: `gift.sent`
- Env: `NATS_URL` (unset → no-op publisher)
- Schema id: `anylive.gift.sent.v1` (see events schema)

## Validation

`scripts/validate-contracts.py` asserts these files exist and parse as JSON objects.
Full JSON Schema draft validation is optional CI later.
