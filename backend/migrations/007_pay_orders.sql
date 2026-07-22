-- Pay products, orders, and webhook event dedupe (PayProvider control plane).

CREATE TABLE IF NOT EXISTS pay_products (
    id              UUID PRIMARY KEY,
    sku             TEXT NOT NULL UNIQUE,
    title           TEXT NOT NULL,
    coins           BIGINT NOT NULL CHECK (coins > 0),
    amount_minor    BIGINT NOT NULL CHECK (amount_minor > 0),
    currency        TEXT NOT NULL DEFAULT 'CNY',
    active          BOOLEAN NOT NULL DEFAULT TRUE,
    sort_order      INT NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS pay_orders (
    id                  UUID PRIMARY KEY,
    user_id             UUID NOT NULL REFERENCES users(id),
    product_id          UUID REFERENCES pay_products(id),
    channel             TEXT NOT NULL,
    status              TEXT NOT NULL,
    coins               BIGINT NOT NULL CHECK (coins > 0),
    amount_minor        BIGINT NOT NULL CHECK (amount_minor > 0),
    currency            TEXT NOT NULL,
    client_request_id   TEXT,
    provider_trade_no   TEXT,
    provider_event_id   TEXT,
    pay_mode            TEXT,
    pay_payload         JSONB NOT NULL DEFAULT '{}'::jsonb,
    expires_at          TIMESTAMPTZ,
    paid_at             TIMESTAMPTZ,
    credited_at         TIMESTAMPTZ,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX IF NOT EXISTS pay_orders_user_client_request_uidx
    ON pay_orders (user_id, client_request_id)
    WHERE client_request_id IS NOT NULL;

CREATE UNIQUE INDEX IF NOT EXISTS pay_orders_channel_trade_no_uidx
    ON pay_orders (channel, provider_trade_no)
    WHERE provider_trade_no IS NOT NULL;

CREATE INDEX IF NOT EXISTS pay_orders_user_created_idx
    ON pay_orders (user_id, created_at DESC);

CREATE INDEX IF NOT EXISTS pay_orders_status_expires_idx
    ON pay_orders (status, expires_at);

CREATE TABLE IF NOT EXISTS pay_webhook_events (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    channel             TEXT NOT NULL,
    provider_event_id   TEXT NOT NULL,
    order_id            UUID REFERENCES pay_orders(id),
    verified            BOOLEAN NOT NULL DEFAULT TRUE,
    process_status      TEXT NOT NULL DEFAULT 'processed',
    body                JSONB,
    error_message       TEXT,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (channel, provider_event_id)
);
