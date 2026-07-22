-- OTP challenges dual store (email OTP put/get/take).
-- `code` stores a SHA-256 hex digest of (pepper || email || plaintext), never the raw OTP.

CREATE TABLE IF NOT EXISTS otp_challenges (
    email TEXT PRIMARY KEY,
    code TEXT NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    attempts INT NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
