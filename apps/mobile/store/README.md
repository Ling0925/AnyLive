# Store packaging metadata (WBS E12.4)

Engineering-side package identity and listing copy stubs. **Account login,
binary upload, and review responses stay human** (see
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

```bash
# local
flutter run --dart-define=APP_FLAVOR=local --dart-define=API_BASE_URL=http://localhost:8088

# stage
flutter build apk --dart-define=APP_FLAVOR=stage --dart-define=API_BASE_URL=https://api.stage.example.com

# prod
flutter build appbundle --dart-define=APP_FLAVOR=prod --dart-define=API_BASE_URL=https://api.example.com
flutter build ipa --dart-define=APP_FLAVOR=prod --dart-define=API_BASE_URL=https://api.example.com
```

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
