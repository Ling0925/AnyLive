-- AnyLive P1.2: age confirmation + privacy acceptance timestamps (per user).

CREATE TABLE IF NOT EXISTS profile_extras (
  user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
  age_confirmed_at TIMESTAMPTZ,
  privacy_accepted_at TIMESTAMPTZ
);
