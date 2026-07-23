-- Multi-role admin RBAC (WBS E7.1): role column on admin_users.
-- Existing admins become 'admin'. Allowed values: admin | moderator | ops.

ALTER TABLE admin_users
    ADD COLUMN IF NOT EXISTS role TEXT NOT NULL DEFAULT 'admin';

-- Guard future writes to known roles (soft check; app still validates).
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'admin_users_role_check'
    ) THEN
        ALTER TABLE admin_users
            ADD CONSTRAINT admin_users_role_check
            CHECK (role IN ('admin', 'moderator', 'ops'));
    END IF;
END $$;
