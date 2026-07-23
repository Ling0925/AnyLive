#!/usr/bin/env python3
"""Centrifugo same-room WS load runner (WBS E12.3 / P1 1k gate).

Opens N WebSocket clients against Centrifugo (not Axum), each with a JWT
from POST /api/v1/realtime/token. Measures connect success and chat fan-out
delivery for a short burst, then writes reports/ws-1k-baseline-<stamp>.md.

Usage:
  ./scripts/loadtest/ws-centrifugo-load.py
  WS_CLIENTS=200 CHAT_MSGS=5 ./scripts/loadtest/ws-centrifugo-load.py
  WS_CLIENTS=1000 HOLD_SECS=180 ./scripts/loadtest/ws-centrifugo-load.py

Requires: live API (:8088 default), Centrifugo WS (:8001), websockets package.

Notes:
  - Chat rate limit is 5 msgs / 10s per user — publishes are spaced
    (CHAT_PUBLISH_GAP default 2.1s) with 429 retry.
  - Clients hold until the global deadline and reconnect if the server closes
    the socket, so soak windows are real connection-stability tests.
"""
from __future__ import annotations

import argparse
import asyncio
import json
import os
import sys
import time
import urllib.error
import urllib.request
from datetime import datetime, timezone
from pathlib import Path

try:
    import websockets
    from websockets.exceptions import ConnectionClosed
except ImportError:
    print("FAIL: pip install websockets", file=sys.stderr)
    sys.exit(2)

ROOT = Path(__file__).resolve().parents[2]
API_BASE = os.environ.get("API_BASE", "http://localhost:8088").rstrip("/")
WS_URL = os.environ.get(
    "CENTRIFUGO_WS", "ws://localhost:8001/connection/websocket"
).rstrip("/")
OTP = os.environ.get("OTP_CODE", "123456")
REPORT_DIR = Path(os.environ.get("REPORT_DIR", ROOT / "reports"))


def http(method: str, path: str, body: dict | None = None, token: str | None = None):
    data = None
    headers = {"content-type": "application/json"}
    if token:
        headers["authorization"] = f"Bearer {token}"
    if body is not None:
        data = json.dumps(body).encode()
    req = urllib.request.Request(
        API_BASE + path, data=data, headers=headers, method=method
    )
    try:
        with urllib.request.urlopen(req, timeout=30) as resp:
            raw = resp.read().decode()
            return resp.status, (json.loads(raw) if raw else None)
    except urllib.error.HTTPError as e:
        raw = e.read().decode()
        raise RuntimeError(f"{method} {path} -> {e.code} {raw}") from e


async def client_session(
    idx: int,
    token: str,
    expected_msgs: int,
    hold_secs: float,
    ready: asyncio.Event,
    results: dict,
) -> None:
    """Connect, hold until deadline, reconnect on close; count push.pub deliveries."""
    got = 0
    connect_ok = False
    err = ""
    held_s = 0.0
    disconnects = 0
    deadline = time.monotonic() + hold_secs
    first_ready = False

    try:
        while time.monotonic() < deadline:
            t_conn = time.monotonic()
            try:
                async with websockets.connect(
                    WS_URL,
                    open_timeout=20,
                    max_size=2**20,
                    # Client-side pings keep NAT / proxy paths alive during soak.
                    ping_interval=20,
                    ping_timeout=20,
                ) as ws:
                    await ws.send(json.dumps({"id": 1, "connect": {"token": token}}))
                    raw = await asyncio.wait_for(ws.recv(), timeout=20)
                    reply = json.loads(raw)
                    if "error" in reply:
                        err = str(reply["error"])
                        if not first_ready:
                            ready.set()
                            first_ready = True
                        break
                    connect_ok = True
                    if not first_ready:
                        ready.set()
                        first_ready = True

                    # Stay open until global deadline or server close.
                    while time.monotonic() < deadline:
                        try:
                            raw = await asyncio.wait_for(ws.recv(), timeout=1.0)
                        except asyncio.TimeoutError:
                            continue
                        except ConnectionClosed:
                            disconnects += 1
                            break
                        try:
                            msg = json.loads(raw)
                        except json.JSONDecodeError:
                            continue
                        push = msg.get("push") or {}
                        pub = push.get("pub") if isinstance(push, dict) else None
                        if isinstance(pub, dict) and "data" in pub:
                            got += 1
                    held_s += max(0.0, time.monotonic() - t_conn)
            except Exception as e:  # noqa: BLE001
                err = f"{type(e).__name__}: {e}"
                if not first_ready:
                    ready.set()
                    first_ready = True
                disconnects += 1
                if time.monotonic() >= deadline:
                    break
                await asyncio.sleep(0.5)
    finally:
        if not first_ready:
            ready.set()

    results[idx] = {
        "connect_ok": connect_ok,
        "got": got,
        "err": err,
        "held_s": held_s,
        "disconnects": disconnects,
    }


async def run_load(
    n_clients: int,
    n_msgs: int,
    hold_secs: float,
    token: str,
    access: str,
    room_id: str,
) -> dict:
    results: dict[int, dict] = {}
    readies = [asyncio.Event() for _ in range(n_clients)]
    tasks = [
        asyncio.create_task(
            client_session(i, token, n_msgs, hold_secs, readies[i], results)
        )
        for i in range(n_clients)
    ]

    # Wait until all clients finished first connect attempt (success or fail), max 90s.
    async def wait_ready(ev: asyncio.Event) -> None:
        try:
            await asyncio.wait_for(ev.wait(), timeout=90)
        except asyncio.TimeoutError:
            pass

    await asyncio.gather(*(wait_ready(ev) for ev in readies))
    # Brief settle so Centrifugo registers all connections.
    await asyncio.sleep(1.0)

    # Chat is rate-limited to 5 msgs / 10s per user. Space publishes so the
    # burst fits the window (~2.1s apart) and retry on RATE_LIMITED.
    pub_ok = 0
    publish_gap = max(0.05, float(os.environ.get("CHAT_PUBLISH_GAP", "2.1")))
    for i in range(n_msgs):
        if i > 0:
            await asyncio.sleep(publish_gap)
        for attempt in range(3):
            try:
                http(
                    "POST",
                    f"/api/v1/rooms/{room_id}/messages",
                    {"body": f"load-msg-{i}-{time.time()}"},
                    access,
                )
                pub_ok += 1
                break
            except Exception as e:  # noqa: BLE001
                err_s = str(e)
                if "RATE_LIMITED" in err_s or "429" in err_s:
                    await asyncio.sleep(2.0 * (attempt + 1))
                    continue
                print(f"WARN: chat publish {i}: {e}", file=sys.stderr)
                break
        else:
            print(
                f"WARN: chat publish {i}: still rate-limited after retries",
                file=sys.stderr,
            )

    # Drain remaining observe window after last publish.
    await asyncio.gather(*tasks, return_exceptions=True)

    connects = sum(1 for r in results.values() if r.get("connect_ok"))
    total_got = sum(int(r.get("got") or 0) for r in results.values())
    # Loss is vs successfully published messages, not requested count.
    expected_total = connects * pub_ok if connects and pub_ok else 0
    loss = None
    if expected_total > 0:
        loss = max(0.0, 1.0 - (total_got / expected_total))
    held_vals = [float(r.get("held_s") or 0) for r in results.values()]
    held_min = min(held_vals) if held_vals else 0.0
    held_p50 = sorted(held_vals)[len(held_vals) // 2] if held_vals else 0.0
    disconnects = sum(int(r.get("disconnects") or 0) for r in results.values())
    errs = [
        r["err"]
        for r in results.values()
        if r.get("err") and not r.get("connect_ok")
    ]
    return {
        "clients_requested": n_clients,
        "connect_ok": connects,
        "connect_rate": connects / n_clients if n_clients else 0,
        "messages_published": pub_ok,
        "deliveries_observed": total_got,
        "expected_deliveries": expected_total,
        "message_loss_est": loss,
        "held_min_s": round(held_min, 1),
        "held_p50_s": round(held_p50, 1),
        "disconnects": disconnects,
        "sample_errors": errs[:5],
    }


def setup_room() -> tuple[str, str, str, str]:
    email = f"ws-load-host-{int(time.time())}@example.com"
    http("POST", "/api/v1/auth/otp/send", {"email": email})
    _, tok = http("POST", "/api/v1/auth/otp/verify", {"email": email, "code": OTP})
    access = tok["access_token"]
    _, room = http(
        "POST",
        "/api/v1/rooms",
        {"title": f"ws-load-{int(time.time())}"},
        access,
    )
    room_id = room["id"]
    http("POST", f"/api/v1/rooms/{room_id}/start", {}, access)
    _, rt = http("POST", "/api/v1/realtime/token", {"room_id": room_id}, access)
    return access, room_id, rt["token"], rt["channels"][0]


def write_report(stats: dict, n_clients: int, hold_secs: float, room_id: str) -> Path:
    REPORT_DIR.mkdir(parents=True, exist_ok=True)
    stamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    path = REPORT_DIR / f"ws-1k-baseline-{stamp}.md"
    loss = stats.get("message_loss_est")
    loss_s = f"{loss * 100:.3f}%" if loss is not None else "_n/a_"
    connect_gate = stats["connect_ok"] >= max(1, int(n_clients * 0.995))
    loss_gate = loss is not None and loss < 0.001
    held_p50 = float(stats.get("held_p50_s") or 0)
    # Treat hold as met when median client held ≥ 90% of requested window.
    hold_gate = held_p50 >= hold_secs * 0.9
    meet = connect_gate and loss_gate
    path.write_text(
        f"""# 1k WS room pressure — measured run

Generated: {stamp} (UTC)
API_BASE: `{API_BASE}`
CENTRIFUGO_WS: `{WS_URL}`
Mode: **live Centrifugo load** (`ws-centrifugo-load.py`)
Room: `{room_id}`

## Target (P1 exit, docs/product/04-非功能与容量.md)

- Same-room **1000** WebSocket connections stable **15 min**
- Chat message loss rate **< 0.1%**
- Plane under test: **Centrifugo** (room channel), not Axum

## This run

| Metric | Target | Actual | Notes |
|---|---|---|---|
| Concurrent WS requested | 1000 | {n_clients} | set `WS_CLIENTS` |
| Connect success | ≥99.5% | {stats['connect_ok']}/{n_clients} ({stats['connect_rate']*100:.2f}%) | Centrifugo JWT connect |
| Hold / observe window | 15 min | {hold_secs:.0f}s requested; held_min={stats.get('held_min_s')}s held_p50={stats.get('held_p50_s')}s | reconnect until deadline |
| Messages published | — | {stats['messages_published']} | via API POST .../messages |
| Deliveries observed | — | {stats['deliveries_observed']} | sum of push.pub on clients |
| Expected deliveries | — | {stats['expected_deliveries']} | connect_ok × published |
| Message loss (est.) | <0.1% | {loss_s} | 1 − deliveries/expected |
| Disconnect/reconnect | — | {stats.get('disconnects', 0)} | client-side reconnects |

## Sample connect errors

```
{json.dumps(stats.get('sample_errors') or [], indent=2)}
```

## Conclusion

- [{'x' if meet else ' '}] Meets **scaled** gate for this run size (connect + loss)
- [{'x' if n_clients >= 1000 else ' '}] Full 1000-client request size
- [{'x' if hold_gate else ' '}] Hold window met (p50 ≥ 90% of {hold_secs:.0f}s; this run p50={held_p50:.0f}s)
- [{'x' if hold_secs >= 900 and hold_gate else ' '}] Full 15-minute soak

Operator: automation (`ws-centrifugo-load.py`)
Date (UTC): {stamp}

## Notes

JWT connect auto-subscribes to `room:{{id}}` (channels claim). Clients do not
re-subscribe. Chat fan-out path: API → Centrifugo HTTP publish → WS push.
Clients reconnect until the hold deadline if the server closes the socket.
""",
        encoding="utf-8",
    )
    return path


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument(
        "--clients", type=int, default=int(os.environ.get("WS_CLIENTS", "200"))
    )
    ap.add_argument(
        "--msgs", type=int, default=int(os.environ.get("CHAT_MSGS", "5"))
    )
    ap.add_argument(
        "--hold", type=float, default=float(os.environ.get("HOLD_SECS", "30"))
    )
    args = ap.parse_args()

    print(
        f"=== Centrifugo WS load: clients={args.clients} msgs={args.msgs} hold={args.hold}s ==="
    )
    print(f"API={API_BASE} WS={WS_URL}")
    try:
        access, room_id, token, channel = setup_room()
    except Exception as e:  # noqa: BLE001
        print(f"FAIL: setup room: {e}", file=sys.stderr)
        return 1
    print(f"room={room_id} channel={channel}")

    t0 = time.time()
    stats = asyncio.run(
        run_load(args.clients, args.msgs, args.hold, token, access, room_id)
    )
    elapsed = time.time() - t0
    print(json.dumps(stats, indent=2))
    print(f"elapsed_s={elapsed:.1f}")
    report = write_report(stats, args.clients, args.hold, room_id)
    print(f"REPORT={report}")
    if stats["connect_ok"] < max(1, int(args.clients * 0.5)):
        print("FAIL: connect success < 50%", file=sys.stderr)
        return 1
    print("WS_CENTRIFUGO_LOAD_PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
