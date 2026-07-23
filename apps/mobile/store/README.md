# Store packaging metadata (WBS E12.4)

Engineering-side package identity, build flavors, and **operator self-test** install notes.
**Account login, binary upload, and review responses stay human** (see
`docs/runbooks/store-internal.md`).

## Identity

| Field | Value |
|---|---|
| Android `applicationId` | `com.anylive.anylive_mobile` |
| iOS Bundle ID | set in Xcode / `PRODUCT_BUNDLE_IDENTIFIER` (default Flutter project) |
| Display name | AnyLive |
| Category | Entertainment / Live streaming |
| Age rating target | 17+ (user-declared 18+ gate in app) |

## Build flavors (dart-define)

Config source: `apps/mobile/lib/config/app_config.dart`.

```bash
# local (host / iOS sim)
flutter run --dart-define=APP_FLAVOR=local --dart-define=API_BASE_URL=http://localhost:8088

# local — Android emulator (host loopback via 10.0.2.2)
flutter run --dart-define=APP_FLAVOR=local --dart-define=API_BASE_URL=http://10.0.2.2:8088

# local — real device LAN IP (replace with your machine)
flutter run --dart-define=APP_FLAVOR=local --dart-define=API_BASE_URL=http://192.168.1.20:8088

# stage
flutter build apk --dart-define=APP_FLAVOR=stage --dart-define=API_BASE_URL=https://api.stage.example.com

# prod
flutter build appbundle --dart-define=APP_FLAVOR=prod --dart-define=API_BASE_URL=https://api.example.com
flutter build ipa --dart-define=APP_FLAVOR=prod --dart-define=API_BASE_URL=https://api.example.com
```

### Android API reachability

| Setup | `API_BASE_URL` | Extra |
|---|---|---|
| Emulator | `http://10.0.2.2:8088` | AVD → host `:8088` |
| Real device + same Wi‑Fi | `http://<host-LAN-IP>:8088` | firewall must allow phone → host |
| Real device + USB | `http://localhost:8088` | `adb reverse tcp:8088 tcp:8088` (also reverse `8080` HLS / `8001` Centrifugo if needed) |

```bash
adb reverse tcp:8088 tcp:8088
adb reverse tcp:8080 tcp:8080
adb reverse tcp:8001 tcp:8001
```

Also documented in `apps/mobile/README.md` and `docs/runbooks/go-live-local.md` §3.

## Operator APK helper (no store upload)

From repo root:

```bash
# debug APK; default API_BASE_URL=http://10.0.2.2:8088
./scripts/build-mobile-apk.sh debug

# release APK aimed at stage API
APP_FLAVOR=stage API_BASE_URL=https://api.stage.example.com \
  ./scripts/build-mobile-apk.sh release
```

| Output | Path |
|---|---|
| Flutter artifact | `apps/mobile/build/app/outputs/flutter-apk/app-debug.apk` or `app-release.apk` |
| Dated copy | `reports/apk/anylive-<flavor>-<mode>-<timestamp>.apk` |

Install:

```bash
adb install -r reports/apk/anylive-local-debug-*.apk
```

This script is **operator-facing only** — it does not open Play Console, sign for distribution beyond Flutter defaults, or upload binaries.

## Listing stubs

See `listing-en.md` / `listing-zh.md` for short description text ready to paste
into Play Console / App Store Connect.

## Privacy labels checklist

- [ ] Account data (email)
- [ ] Purchase history (coins / IAP)
- [ ] User content (chat, reports)
- [ ] Device identifiers (push token when registered)
- [ ] Diagnostics (optional client events)

Public privacy / terms URLs must be real HTTPS before store submit
(`GET /api/v1/legal/privacy` and `/terms` on the API are the in-product source).

## Device matrix

Scaffold empty V-FL-1 reports (Pass not claimed):

```bash
./scripts/device-matrix-prefill.sh mid-android
```

See `reports/device-matrix-TEMPLATE.md`.
