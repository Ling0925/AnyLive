-- AnyLive E2.3 / E3.5: avatar URL on profile extras + optional room recording flag.

ALTER TABLE profile_extras
  ADD COLUMN IF NOT EXISTS avatar_url TEXT;

CREATE TABLE IF NOT EXISTS room_recording (
  room_id UUID PRIMARY KEY REFERENCES rooms(id) ON DELETE CASCADE,
  enabled BOOLEAN NOT NULL DEFAULT FALSE,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
