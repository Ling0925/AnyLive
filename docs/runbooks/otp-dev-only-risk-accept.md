# Risk accept — dogfood with dev-only OTP

> **⚠ UNSIGNED DRAFT — does NOT equal V-BE-1 done.**  
> Status: **unsigned draft** (plan 06 §8.3 exit #9 / decision D2 · Wave2 **V-BE-1**).  
> **Does not** close the real-ESP gate. Signing this only accepts a **scoped** dogfood using fixed/dev OTP.  
> Scripts, CI, and agents **must not** flip `V-BE-1` to `done` while this file remains unsigned or only a draft in git.

| Gate | Link |
|---|---|
| Wave2 ID | **V-BE-1** — [p1-parallel-tracks.md](../product/p1-parallel-tracks.md) §5 / status table |
| Plan 06 | §8.3 exit **#9** — [06-P1进度评审与后续规划.md](../product/06-P1进度评审与后续规划.md) |
| Prefer close | Wire real `OTP_NOTIFIER=http\|smtp` + ESP; see [go-live-stage.md](./go-live-stage.md) |
| Signed archive | Copy filled+signed PDF/MD under `reports/risk-accept/otp-dev-only-<UTC>-signed.md` (gitignored secrets OK; keep redacted copy if needed) |

## Context

| Item | Value |
|---|---|
| Code path | `OTP_NOTIFIER=http\|smtp` + `HttpOtpNotifier` implemented; production rejects `log`/`noop` |
| Local / test stack | Dev fixed OTP (commonly `123456` via `ALLOW_DEV_OTP`) still used by `deploy-test` / dogfood scripts |
| Gap | No production-grade ESP account (SendGrid/SES/etc.) wired for non-dev delivery |
| Related | [p1-status](../product/p1-status.md) · [go-live-local](./go-live-local.md) · [dogfood-cohort](./dogfood-cohort.md) · plan 06 R3 / D2 |

## What is accepted if signed

- Internal / closed dogfood may use **dev fixed OTP** for engineers and synthetic cohort scripts.
- Control-plane smoke (`dogfood-api-smoke`, `dogfood-10min-path`, `dogfood-cohort-seed`) may continue against `ALLOW_DEV_OTP`.
- P1 exit check #9 may be marked **risk-accepted (dev-only)** — **not** “real OTP delivered”.
- V-BE-1 may be marked **done** only after this form is **signed** and a copy is archived under `reports/` (or real ESP is proven instead).

## What is **not** accepted

- Claiming “users receive email OTP in production” or non-dev public invite.
- Scaling dogfood to **~500 real humans** without a real notifier (plan 06 D2: expand only after ESP).
- Disabling production OTP guards (`ALLOW_DEV_OTP` / log notifier in `APP_ENV=production`).
- Treating this form as a substitute for ESP credentials in stage/prod runbooks.
- Auto-closing V-BE-1 from CI dogfood PASS or an unsigned draft in the repo.

## Residual risks

1. **Credential leakage:** fixed OTP is shared knowledge; any exposed test API can be logged into with known emails.
2. **False confidence:** path green on `123456` does not prove deliverability, rate limits, or bounce handling.
3. **Ban/OTP re-login tests** only prove policy against the same dev channel — not mailbox UX.
4. **Invite-only soft open** still depends on code paths that real users cannot complete without delivery.

## Mitigations while accepted

- Keep test stack off the public internet; bind to localhost / VPN.
- Prefer throwaway emails in scripts; do not reuse banned accounts (no unban API).
- Track ESP wiring as an open action; re-open this risk if dogfood leaves the engineering cohort.
- Production / stage: keep `ALLOW_DEV_OTP=0` and non-log notifier requirements (`deploy/.env.stage.example`).

## Sign-off (leave blank until decided)

| Role | Name | Date (UTC) | Signature / initials |
|---|---|---|---|
| Tech lead |  |  |  |
| Product / PM |  |  |  |
| Optional: Security / compliance |  |  |  |

**Scope of dogfood covered by this accept (fill before signing):**

- Max real humans: ________ (recommend ≤ engineering team + invited hosts only)
- Environments: ☐ local compose ☐ shared test ☐ stage (dev OTP **not** recommended)
- Environments out of scope: ________
- Expiry / re-review date (UTC): ________
- Archive path after sign: `reports/risk-accept/otp-dev-only-________-signed.md`

**Checkbox (signer only):**

- [ ] I accept residual risks above for the scoped dogfood only
- [ ] I will not mark plan 06 exit #9 as “real OTP” based on this document alone
- [ ] I will not treat CI/`DOGFOOD_*_PASS` as V-BE-1 closed
- [ ] Follow-up: wire ESP (`OTP_NOTIFIER=http|smtp` + real URL) before 500-user / public claim

Notes: _______________
