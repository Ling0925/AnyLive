-- OTP challenges dual store (email OTP put/get/take).

CREATE TABLE IF NOT EXISTS otp_challenges (
    email TEXT PRIMARY KEY,
    code TEXT NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    attempts INT NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
