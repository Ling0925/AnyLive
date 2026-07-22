-- Unique topup references per user (idempotent credit_topup).
-- Gift ledger rows use order UUIDs as reference and remain unique in practice.

CREATE UNIQUE INDEX IF NOT EXISTS wallet_ledger_user_id_reference_key
    ON wallet_ledger (user_id, reference)
    WHERE entry_type = 'topup';
