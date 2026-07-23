-- AnyLive E2.5: optional ISO region / country code on profile extras.

ALTER TABLE profile_extras
  ADD COLUMN IF NOT EXISTS region TEXT;
