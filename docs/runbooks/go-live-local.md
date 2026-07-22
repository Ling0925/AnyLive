# Local go-live (dogfood stack)

End-to-end host publish → SRS → HLS watch against the Docker test stack.

## 1. Start the stack

```bash
./scripts/deploy-test.sh
```

Brings up Postgres, Redis, NATS, MinIO, Centrifugo, SRS, API (`:8088`), and admin (`:8090`).

Dogfood flags are wired into the API service (also in `deploy/.env.test`):

| Env | Default (test) | Purpose |
|---|---|---|
| `ALLOW_DEV_OTP` | `1` | Fixed OTP `123456` |
| `ALLOW_MOCK_TOPUP` | `1` | Mock wallet topup for gifts |
| `OTP_NOTIFIER` | `noop` | No real email/SMS |
| `SRS_PUBLISH_SECRET` | test value | HMAC for publish stream keys |
| `SRS_WEBHOOK_SECRET` | test value | SRS hook auth header |

After up, the script prints OBS hints and runs:

- `./scripts/dogfood-api-smoke.sh`
- `./scripts/dogfood-media-smoke.sh`

Skip smokes with `SKIP_DOGFOOD_SMOKE=1 ./scripts/deploy-test.sh`.

Stop:

```bash
docker compose -f deploy/docker-compose.yml --profile app down
```

## 2. Admin bootstrap

1. Open **http://localhost:8090/**
2. Email OTP login — code **`123456`** (dev OTP)
3. On first boot (empty admin set), the UI attempts **bootstrap grant** via `POST /api/v1/admin/grant` for the logged-in user
4. Admin can force-close rooms, ban/mute, resolve reports, manage gifts

API base for the baked admin image: `http://localhost:8088` (`VITE_API_BASE`).

## 3. Mobile / H5 API base

| Client | API base |
|---|---|
| API health | `http://localhost:8088/health` |
| Mobile (Flutter) | `API_BASE_URL` default `http://localhost:8088` (`apps/mobile/lib/config/app_config.dart`) |
| H5 watch | `VITE_API_BASE` default `http://localhost:8088` |
| Admin | `VITE_API_BASE` default `http://localhost:8088` |

Device notes:

- iOS Simulator / desktop browser: `localhost:8088` works.
- Android emulator: use `http://10.0.2.2:8088` (or your LAN IP).
- Physical device: use host LAN IP, e.g. `http://192.168.x.x:8088`.

## 4. Host: create room + publish (OBS)

Control plane (host token after OTP `123456`):

1. `POST /api/v1/rooms` → create
2. `POST /api/v1/rooms/{id}/start` → `status: live`
3. `POST /api/v1/rooms/{id}/media/publish` → copy:

| Field | Use |
|---|---|
| `push_url` | Full RTMP URL including stream name |
| `stream_key` | **Signed** key `{room_id}_{exp}_{sig}` — not bare room UUID |
| `expires_at` | Key expiry; re-call publish if expired |

Or run `./scripts/dogfood-api-smoke.sh` / mobile host flow and copy printouts.

### OBS Custom RTMP

| Field | Value |
|---|---|
| Service | Custom… |
| Server | `rtmp://localhost:1935/live` |
| Stream key | **exact** `stream_key` from media/publish (signed) |

Bare room UUID as stream key is **rejected** by the API webhook (`validate_publish_stream`). SRS hooks call:

- `on_publish` / `on_unpublish` → `http://host.docker.internal:8088/api/v1/webhooks/srs/...`
- Hooks validate the signed stream name and optional `SRS_WEBHOOK_SECRET`

See `deploy/srs/srs.conf` and `scripts/dogfood-media.md`.

## 5. Watch HLS

Public play URL (after host has called media/publish):

```http
GET /api/v1/rooms/{id}/media/play
→ { "hls": "http://localhost:8080/live/{stream_key}.m3u8", "flv": "..." }
```

Play path uses the **active signed stream key** issued by media/publish so it matches
the OBS RTMP stream name on SRS. Viewers only need the HLS URL (not the RTMP key).
If publish was never issued, play falls back to bare `{room_id}.m3u8`.

| Viewer | How |
|---|---|
| H5 | Dev server for `apps/h5-web`, open `?room={room_id}` (e.g. `http://localhost:5173/?room=<uuid>`) |
| Direct HLS | Paste `hls` into Safari / VLC / ffplay |
| Flutter | Room page: copy stream URL for external player until in-app media lands |

## 6. Quick checklist

- [ ] `./scripts/deploy-test.sh` — API healthy, admin up
- [ ] Admin OTP `123456` + bootstrap grant
- [ ] Host start live + media/publish → OBS Server + **signed** stream key
- [ ] Fan/H5 `?room=` or play API HLS
- [ ] Optional: stop OBS → webhook unpublish (or host `POST .../stop`)

## Related

- `scripts/dogfood-media.md` — media plane detail
- `scripts/dogfood-api-smoke.sh` / `scripts/dogfood-media-smoke.sh`
- `deploy/.env.test` — compose env defaults
