# Dogfood media path: OBS → SRS → H5 / Flutter

Control-plane URLs come from the API; media bytes go through SRS (not the Rust process).

## 1. Start SRS

```bash
docker compose -f deploy/docker-compose.yml up -d srs
# RTMP :1935  ·  HTTP play :8080  ·  API :1985
```

API memory mode does **not** need the full compose stack — only SRS for actual push/play.

## 2. Host: create + start room, copy publish info

```bash
cargo run -p anylive-api   # :8088, OTP 123456
# or: ./scripts/dogfood-api-smoke.sh   # prints push_url / stream_key / hls
```

Host flow:

1. OTP login as host
2. `POST /api/v1/rooms` → `POST /api/v1/rooms/{id}/start`
3. `POST /api/v1/rooms/{id}/media/publish` → copy:
   - `push_url` — full RTMP URL (includes stream name)
   - `stream_key` — room UUID (matches play path)

## 3. OBS custom RTMP

| Field | Value |
|---|---|
| Service | Custom… |
| Server | `rtmp://localhost:1935/live` (or host from `push_url` without the stream name) |
| Stream key | room UUID (`stream_key` from media/publish) |

Start streaming in OBS. Stream name must equal the room UUID.

## 4. Play HLS (H5 or Flutter)

Fan / public:

```http
GET /api/v1/rooms/{id}/media/play
→ { "hls": "http://localhost:8080/live/{room_id}.m3u8", "flv": "..." }
```

- **H5**: open watch page with room id; player attaches via `hls.js` (or native HLS).
- **Flutter**: room page is control-plane only today — copy the HLS URL into an external player, or paste into Safari/VLC until in-app media_kit lands.

## 5. Auto-stop on unpublish

When OBS stops, SRS should call the API webhook (configure callback URL to the API host):

- `POST /api/v1/webhooks/srs/on_unpublish` → room leaves live
- `POST /api/v1/webhooks/srs/on_publish` → optional gate / audit

Wire SRS `http_hooks` to the API base if not already set in the local conf; until hooks are pointed at the API, stop the room with `POST .../stop` or admin force-close.

## Smoke order

1. `cargo run -p anylive-api`
2. `./scripts/dogfood-api-smoke.sh` (control plane)
3. Compose up `srs` + OBS publish + H5/Flutter play (media plane)
