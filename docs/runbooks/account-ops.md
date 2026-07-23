# Account operations (Wave A — password + admin provision)

## Overview

AnyLive Wave A uses **admin-provisioned password accounts** as the primary login path.

| Path | Default | Notes |
|---|---|---|
| `POST /api/v1/auth/password/login` | **Primary** | `{ identifier, password }` — email or username |
| `POST /api/v1/auth/password/change` | Authenticated | Revokes all refresh sessions |
| `POST /api/v1/admin/users` | Admin only | Create user + credential |
| OTP send/verify | Secondary | Existing users only when `FEATURE_PUBLIC_REGISTER=0` |
| OAuth | Stub | Unchanged; still behind public register gate |

## Feature flags

| Env | Default (from_env) | Meaning |
|---|---|---|
| `FEATURE_PUBLIC_REGISTER` | **off** | When off, OTP/OAuth do not create new users |
| `PASSWORD_MIN_LEN` | 8 | Minimum password length |
| `LOGIN_MAX_ATTEMPTS` | 5 | Failures before lockout |
| `LOGIN_LOCK_SECS` | 900 | Lockout duration |

Local integration tests use `AppState::dev()` → `FeatureFlags::all_enabled()` so OTP self-register still works in the test suite.

## Bootstrap first admin

Chicken-and-egg: you need an admin to open accounts.

### Option A — local OTP bootstrap (dev)

```bash
# Temporary: enable public register for first operator only
export FEATURE_PUBLIC_REGISTER=1
export ALLOW_DEV_OTP=1

# OTP login as yourself, then POST /admin/grant (self bootstrap when admin set empty)
# or:
./scripts/seed-password-admin.sh
```

### Option B — seed script

```bash
./scripts/seed-password-admin.sh
# optional overrides:
ADMIN_USERNAME=ops ADMIN_PASSWORD='ChangeMe123!' ADMIN_EMAIL=ops@example.com \
  ./scripts/seed-password-admin.sh
```

Script flow: OTP bootstrap → grant admin → `POST /admin/users` with `role=admin` → password login smoke.

## Operator playbook

### Open an account

```http
POST /api/v1/admin/users
Authorization: Bearer <admin>
{
  "display_name": "Host One",
  "username": "host1",
  "email": "host1@example.com",
  "password": "optional-if-omit-generates-temp",
  "must_change_password": true
}
```

Response may include `temporary_password` **once**. Copy it out-of-band; it is not stored in audit detail.

### Reset password

```http
POST /api/v1/admin/users/{id}/reset-password
{ "must_change_password": true }
```

All refresh sessions for that user are revoked.

### Disable vs ban

| Action | Effect |
|---|---|
| `PATCH` status=`disabled` | Cannot login; sessions revoked |
| `POST /admin/ban` | Policy ban; sessions revoked; password login → 403 |
| `POST /admin/unban` | Removes ban |

### Kick sessions

```http
POST /api/v1/admin/users/{id}/revoke-sessions
```

## Client notes

- **Flutter**: LoginPage password form is primary; Dev OTP is collapsed.
- **Admin-web**: Password login primary; “Dev OTP” toggle for local; Users nav for provision/reset.

## Migration

`011_user_credentials.sql` adds:

- `users.username` / `users.status` / `users.created_by`
- `user_credentials` (argon2id PHC hash, lockout fields)
- optional device columns on `refresh_tokens` (Wave B)

Apply via normal `USE_POSTGRES=1` migrate path.

## Security checklist

- [ ] Production: `FEATURE_PUBLIC_REGISTER=0`
- [ ] Production: no `ALLOW_DEV_OTP`
- [ ] Strong unique JWT + Centrifugo secrets
- [ ] Temp passwords only over HTTPS; never log them
- [ ] Operators rotate temp passwords on first login (`must_change_password`)
