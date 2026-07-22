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
# or: ./scripts/dogfood-media-smoke.sh # health + publish/play consistency + optional SRS
```

Host flow:

1. OTP login as host
2. `POST /api/v1/rooms` → `POST /api/v1/rooms/{id}/start`
3. `POST /api/v1/rooms/{id}/media/publish` → copy:
   - `push_url` — full RTMP URL (includes stream name)
   - `stream_key` — **signed token** `{room_id}_{exp}_{sig}` (NOT the bare room UUID)

Bare room UUIDs are rejected by `on_publish`. The API remembers the issued key so
play URLs use the same stream name SRS writes for HLS/FLV.

## 3. OBS custom RTMP

| Field | Value |
|---|---|
| Service | Custom… |
| Server | `rtmp://localhost:1935/live` (or host from `push_url` without the stream name) |
| Stream key | **full** `stream_key` from media/publish (`{room}_{exp}_{sig}`) |

Start streaming in OBS. Stream name must equal the full signed `stream_key`.

## 4. Play HLS (H5 or Flutter)

Fan / public:

```http
GET /api/v1/rooms/{id}/media/play
→ { "hls": "http://localhost:8080/live/{stream_key}.m3u8", "flv": "..." }
```

While a publish credential is active, HLS/FLV use the **same signed stream name**
OBS pushed (SRS writes `{stream_key}.m3u8`). After stop / unpublish / force-close
the mapping is cleared and play falls back to bare room id.

- **H5**: open watch page with room id; player attaches via `hls.js` (or native HLS).
- **Flutter**: room page is control-plane only today — copy the HLS URL into an external player, or paste into Safari/VLC until in-app media_kit lands.

## 5. Auto-stop on unpublish

When OBS stops, SRS should call the API webhook (configure callback URL to the API host):

- `POST /api/v1/webhooks/srs/on_unpublish` → room leaves live + clears active stream mapping
- `POST /api/v1/webhooks/srs/on_publish` → optional gate / audit (signed key required)

Local `deploy/srs/srs.conf` enables `http_hooks` →
`http://host.docker.internal:8088/api/v1/webhooks/srs/on_publish|on_unpublish`.
On Linux Docker without that host alias, override the conf or add
`extra_hosts: ["host.docker.internal:host-gateway"]` on the `srs` service.
If hooks cannot reach the API, stop the room with `POST .../stop` or admin force-close.

## Smoke order

1. `cargo run -p anylive-api`
2. `./scripts/dogfood-api-smoke.sh` (full control plane: auth, gifts, chat, feed)
3. `./scripts/dogfood-media-smoke.sh` (media-focused: `/health`, optional SRS `:1985`, OTP → room → publish/play consistency)
4. Compose up `srs` + OBS publish + H5/Flutter play (media plane bytes)

### Media smoke details

| Piece | Path |
|---|---|
| Pure helpers (parse publish/play, signed stream_key form, SRS probe URL) | `scripts/media_smoke_lib.py` |
| Unit tests | `python3 -m unittest scripts/test_media_smoke_lib.py` |
| Live smoke script | `./scripts/dogfood-media-smoke.sh` |

Env:

| Variable | Default | Meaning |
|---|---|---|
| `API_BASE` | `http://localhost:8088` | Control-plane API |
| `OTP_CODE` | `123456` | Dev OTP (same as dogfood-api-smoke) |
| `SRS_API_BASE` | `http://127.0.0.1:1985` | Optional SRS HTTP API |
| `SKIP_SRS` | `0` | Set `1` to skip the SRS probe |

`dogfood-media-smoke` does **not** push RTMP or wait for HLS segments — it checks that
publish/play responses are consistent (signed `stream_key` form, HLS path contains the
stream key / room id, OBS server derived from `push_url`). Byte-plane verification is
still OBS → SRS → player.
