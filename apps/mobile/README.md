# AnyLive mobile (Flutter)

Audience + streamer client for iOS/Android. **Not** Flutter Web — public watch page is `apps/h5-web`.

## Prerequisites

- Flutter stable (see CI / team pin)
- Local or stage API: `./scripts/deploy-test.sh` or `cargo run -p anylive-api` (default `:8088`)
- Optional: `adb` for Android install / port reverse

## Config (`lib/config/app_config.dart`)

| dart-define | Default | Notes |
|---|---|---|
| `API_BASE_URL` | `http://localhost:8088` | **Must** be overridden for Android emulator / real device |
| `APP_FLAVOR` | falls back to `APP_ENV` / `local` | `local` \| `stage` \| `prod` |
| `APP_ENV` | `local` | environment label |
| `CENTRIFUGO_WS` | empty | optional; empty → HTTP chat poll only |
| `H5_BASE_URL` | derived from API host `:5173` | share deep links (`?room=`); Android emulator maps `10.0.2.2` → `127.0.0.1` |
| `ANYLIVE_EMBEDDED_PLAYER` | off in tests | set `true` for media_kit HLS embed |

## API base: emulator, adb reverse, real device

| Target | `API_BASE_URL` | Notes |
|---|---|---|
| iOS Simulator / desktop | `http://localhost:8088` | default works |
| **Android emulator** | `http://10.0.2.2:8088` | `10.0.2.2` is the host loopback from the AVD |
| **Android real device** | `http://<LAN-IP>:8088` | e.g. `http://192.168.1.20:8088` — phone and host on same Wi‑Fi |
| **Android + adb reverse** | `http://localhost:8088` | after reverse, device `localhost` maps to host |

### adb reverse (real device alternative to LAN IP)

When USB debugging is on and the API/media stack runs on the host:

```bash
adb reverse tcp:8088 tcp:8088   # API
adb reverse tcp:8080 tcp:8080   # SRS HLS (if play URLs use host:8080)
adb reverse tcp:8001 tcp:8001   # Centrifugo (if used)
```

Then you may keep `API_BASE_URL=http://localhost:8088` on the device. Without reverse, use the host LAN IP instead.

HLS play URLs returned by the API may still embed `localhost` or Docker hostnames — for real devices, prefer LAN-reachable media URLs or reverse the media ports too. See `docs/runbooks/go-live-local.md` §3 and §5.

## Run (debug)

```bash
cd apps/mobile

# Android emulator → host API
flutter run \
  --dart-define=APP_FLAVOR=local \
  --dart-define=API_BASE_URL=http://10.0.2.2:8088

# Real device via LAN IP (replace IP)
flutter run \
  --dart-define=APP_FLAVOR=local \
  --dart-define=API_BASE_URL=http://192.168.1.20:8088

# Real device via adb reverse
adb reverse tcp:8088 tcp:8088
flutter run \
  --dart-define=APP_FLAVOR=local \
  --dart-define=API_BASE_URL=http://localhost:8088
```

Dev OTP on the local test stack is typically `123456` when `ALLOW_DEV_OTP=1`.

## Build APK (operator self-test)

Prefer the helper (copies under `reports/apk/`):

```bash
# from repo root — debug APK, emulator-friendly default API
./scripts/build-mobile-apk.sh debug

# stage-shaped release APK (still no store upload)
APP_FLAVOR=stage API_BASE_URL=https://api.stage.example.com \
  ./scripts/build-mobile-apk.sh release
```

Manual equivalent and store identity: [`store/README.md`](./store/README.md).  
Play/TestFlight account steps: `docs/runbooks/store-internal.md` (human only).

## Device matrix (V-FL-1)

Do **not** claim matrix Pass from install alone.

```bash
# from repo root — scaffolds empty path cells + Pass=not claimed
./scripts/device-matrix-prefill.sh mid-android
# → reports/device-matrix-YYYYMMDD-mid-android.md
```

Template: `reports/device-matrix-TEMPLATE.md`. H5 browser rows: `apps/h5-web/README.md`.

## Human 10‑minute path (V-FL-2)

Control-plane first: `./scripts/dogfood-10min-path.sh` must PASS, then human records login→feed→HLS→chat→gift→end-state with a **recording URL** — see `docs/runbooks/dogfood-cohort.md`.

## Layout

| Path | Role |
|---|---|
| `lib/config/` | `AppConfig` / dart-defines |
| `lib/api/` | repositories |
| `lib/features/` | UI pages |
| `lib/player/` | stream preview / media_kit |
| `store/` | package id + listing stubs + build notes |
| `test/` | widget / repo tests |

## Related

- Local stack: `docs/runbooks/go-live-local.md`
- Dogfood cohort + V-FL-2: `docs/runbooks/dogfood-cohort.md`
- Parallel tracks (V-FL-1/2): `docs/product/p1-parallel-tracks.md`
