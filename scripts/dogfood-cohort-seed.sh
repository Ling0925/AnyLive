#!/usr/bin/env bash
# Synthetic dogfood cohort seed (P1 · 20 hosts / 500 users control-plane).
#
# Creates N_HOSTS live rooms + N_USERS registered accounts against a running API.
# Writes reports/dogfood-cohort-<stamp>.md with counts + reconcile snapshot.
# Does **not** replace real OBS week or human cohort sign-off.
#
# Usage:
#   ./scripts/dogfood-cohort-seed.sh
#   N_HOSTS=20 N_USERS=500 API_BASE=http://localhost:8088 ./scripts/dogfood-cohort-seed.sh

set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

API_BASE="${API_BASE:-http://localhost:8088}"
API_BASE="${API_BASE%/}"
OTP_CODE="${OTP_CODE:-123456}"
N_HOSTS="${N_HOSTS:-20}"
N_USERS="${N_USERS:-500}"
REPORT_DIR="${REPORT_DIR:-$ROOT/reports}"
STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
REPORT="$REPORT_DIR/dogfood-cohort-$STAMP.md"

mkdir -p "$REPORT_DIR"

python3 - "$API_BASE" "$OTP_CODE" "$N_HOSTS" "$N_USERS" "$REPORT" "$STAMP" <<'PY'
import json, sys, time, urllib.request, urllib.error

api_base, otp, n_hosts, n_users, report, stamp = sys.argv[1:7]
n_hosts, n_users = int(n_hosts), int(n_users)

def api(method, path, body=None, token=None):
    data = None
    headers = {"content-type": "application/json"}
    if token:
        headers["authorization"] = f"Bearer {token}"
    if body is not None:
        data = json.dumps(body).encode()
    req = urllib.request.Request(api_base + path, data=data, headers=headers, method=method)
    with urllib.request.urlopen(req, timeout=60) as r:
        raw = r.read().decode()
        return r.status, (json.loads(raw) if raw else None)

def api_retry(method, path, body=None, token=None, retries=6):
    """OTP IP limiter is 20/min — back off on 429 and retry."""
    last = None
    for attempt in range(retries):
        try:
            return api(method, path, body=body, token=token)
        except urllib.error.HTTPError as e:
            last = e
            if e.code == 429:
                # Sliding window is 60s / 20 hits; wait enough to drain a slot.
                time.sleep(min(8.0, 1.5 * (attempt + 1)))
                continue
            raise
        except Exception as e:
            last = e
            time.sleep(0.5 * (attempt + 1))
    raise last if last else RuntimeError("api_retry failed")

def login(email: str) -> str:
    api_retry("POST", "/api/v1/auth/otp/send", {"email": email})
    # Pace OTP sends under DEFAULT_OTP_IP_MAX=20 / 60s.
    time.sleep(3.2)
    _, tok = api_retry("POST", "/api/v1/auth/otp/verify", {"email": email, "code": otp})
    return tok["access_token"]

hosts = []
users_ok = 0
errors = []
t0 = time.time()

print(f"=== dogfood cohort seed hosts={n_hosts} users={n_users} ===", flush=True)
print(f"API={api_base}", flush=True)

# hosts + live rooms
for i in range(n_hosts):
    email = f"cohort-host-{stamp}-{i}@example.com"
    try:
        token = login(email)
        _, room = api("POST", "/api/v1/rooms", {"title": f"cohort-host-{i}"}, token)
        api("POST", f"/api/v1/rooms/{room['id']}/start", {}, token)
        hosts.append({"email": email, "room_id": room["id"]})
        if (i + 1) % 5 == 0:
            print(f"  hosts {i+1}/{n_hosts}", flush=True)
    except Exception as e:
        errors.append(f"host {i}: {e}")

# plain users
for i in range(n_users):
    email = f"cohort-user-{stamp}-{i}@example.com"
    try:
        login(email)
        users_ok += 1
        if (i + 1) % 50 == 0:
            print(f"  users {i+1}/{n_users}", flush=True)
    except Exception as e:
        errors.append(f"user {i}: {e}")

# reconcile via one host promoted if possible
reconcile = {"balanced": None, "error": None}
try:
    # use first host
    if hosts:
        htok = login(hosts[0]["email"])
        try:
            api("POST", "/api/v1/admin/bootstrap", {}, htok)
        except Exception:
            pass
        try:
            st, body = api("GET", "/api/v1/admin/wallet/reconcile", token=htok)
            reconcile = {
                "balanced": body.get("balanced"),
                "checked": body.get("checked_accounts") or body.get("checked"),
                "imbalance": body.get("imbalance_count") or body.get("imbalances"),
            }
        except Exception as e:
            reconcile["error"] = str(e)
except Exception as e:
    reconcile["error"] = str(e)

elapsed = time.time() - t0
hosts_ok = len(hosts)
text = f"""# Dogfood cohort seed — {stamp}

Synthetic control-plane seed (not a substitute for real OBS week / human sign-off).

| Metric | Target | Actual |
|---|---|---|
| Hosts with live room | ≥ 20 | {hosts_ok} |
| Registered users | ≥ 500 | {users_ok} |
| Elapsed | — | {elapsed:.1f}s |
| API | — | `{api_base}` |

## Reconcile

```json
{json.dumps(reconcile, indent=2)}
```

## Host sample (first 5)

```json
{json.dumps(hosts[:5], indent=2)}
```

## Errors ({len(errors)})

```
{chr(10).join(errors[:20]) or '(none)'}
```

## Sign-off

- [x] Control-plane seed executed ({stamp})
- [ ] Real OBS ≥7d continuous (human)
- [ ] Product / eng human cohort form (`docs/runbooks/dogfood-cohort.md`)

Operator: `scripts/dogfood-cohort-seed.sh`
"""
open(report, "w", encoding="utf-8").write(text)
print(f"hosts_ok={hosts_ok} users_ok={users_ok} errors={len(errors)}")
print(f"REPORT={report}")
if hosts_ok < n_hosts or users_ok < n_users:
    print("FAIL: did not reach requested cohort sizes", file=sys.stderr)
    sys.exit(1)
print("DOGFOOD_COHORT_SEED_PASS")
PY
