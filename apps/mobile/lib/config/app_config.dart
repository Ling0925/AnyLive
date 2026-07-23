/// Runtime configuration for AnyLive mobile.
///
/// Flavors (WBS E8.1): pass `--dart-define=APP_FLAVOR=local|stage|prod`.
/// When unset, [flavor] falls back to [environment] / `APP_ENV`.
///
/// [API_BASE_URL] defaults to `http://localhost:8088` (iOS Simulator / desktop).
/// Override for Android:
/// - Emulator: `--dart-define=API_BASE_URL=http://10.0.2.2:8088`
/// - Real device: host LAN IP, or `adb reverse tcp:8088 tcp:8088` + localhost
/// See `apps/mobile/README.md` and `apps/mobile/store/README.md`.
class AppConfig {
  const AppConfig({
    required this.apiBaseUrl,
    required this.environment,
    this.flavor = 'local',
    this.centrifugoWsUrl,
    this.h5BaseUrl,
  });

  final String apiBaseUrl;
  final String environment;

  /// Build flavor: `local` | `stage` | `prod` (case-insensitive at parse time).
  final String flavor;

  /// Optional Centrifugo WS base (`ws://host:8000/connection/websocket`).
  /// When null/empty, room chat uses HTTP history poll only.
  final String? centrifugoWsUrl;

  /// Optional H5 watch origin for share deep links (`http://host:5173`).
  /// When null, share derives `http://<api-host>:5173` for local dogfood.
  final String? h5BaseUrl;

  /// Reads `--dart-define` values with safe defaults for local dev.
  factory AppConfig.fromEnvironment() {
    const api = String.fromEnvironment(
      'API_BASE_URL',
      defaultValue: 'http://localhost:8088',
    );
    const env = String.fromEnvironment(
      'APP_ENV',
      defaultValue: 'local',
    );
    const flavorRaw = String.fromEnvironment(
      'APP_FLAVOR',
      defaultValue: '',
    );
    const ws = String.fromEnvironment(
      'CENTRIFUGO_WS',
      defaultValue: '',
    );
    const h5 = String.fromEnvironment(
      'H5_BASE_URL',
      defaultValue: '',
    );
    final flavor = normalizeFlavor(flavorRaw.isEmpty ? env : flavorRaw);
    return AppConfig(
      apiBaseUrl: api,
      environment: env,
      flavor: flavor,
      centrifugoWsUrl: ws.isEmpty ? null : ws,
      h5BaseUrl: h5.isEmpty ? null : h5,
    );
  }

  /// Canonicalize flavor labels used in banners / analytics.
  static String normalizeFlavor(String raw) {
    final t = raw.trim().toLowerCase();
    switch (t) {
      case 'production':
      case 'prod':
        return 'prod';
      case 'staging':
      case 'stage':
        return 'stage';
      case 'dev':
      case 'development':
      case 'local':
      case '':
        return 'local';
      default:
        return t;
    }
  }

  bool get isLocal => flavor == 'local' || environment == 'local';

  bool get isStage => flavor == 'stage';

  bool get isProd => flavor == 'prod';

  /// Short label for AppBar / debug banner (`local` / `stage` / `prod`).
  String get flavorLabel => flavor;

  /// Normalize base URL (no trailing slash).
  String get normalizedApiBaseUrl {
    if (apiBaseUrl.endsWith('/')) {
      return apiBaseUrl.substring(0, apiBaseUrl.length - 1);
    }
    return apiBaseUrl;
  }

  /// Normalized Centrifugo WS URL, or null when not configured.
  String? get normalizedCentrifugoWsUrl {
    final raw = centrifugoWsUrl?.trim();
    if (raw == null || raw.isEmpty) return null;
    if (raw.endsWith('/')) return raw.substring(0, raw.length - 1);
    return raw;
  }

  /// H5 origin used for `?room=` share links (no trailing slash).
  String get normalizedH5BaseUrl {
    final raw = h5BaseUrl?.trim();
    if (raw != null && raw.isNotEmpty) {
      return raw.endsWith('/') ? raw.substring(0, raw.length - 1) : raw;
    }
    // Local dogfood default: same host as API, Vite preview :5173.
    final api = Uri.tryParse(normalizedApiBaseUrl);
    final scheme = api?.scheme.isNotEmpty == true ? api!.scheme : 'http';
    var host = api?.host;
    if (host == null || host.isEmpty) host = 'localhost';
    // Android emulator loopback to host machine for H5 preview on laptop.
    if (host == '10.0.2.2') host = '127.0.0.1';
    return '$scheme://$host:5173';
  }

  /// Shareable H5 deep link for a room (`?room=<id>`).
  String shareRoomUrl(String roomId) {
    final id = roomId.trim();
    final base = normalizedH5BaseUrl;
    if (id.isEmpty) return base;
    return '$base/?room=$id';
  }

  Uri healthUri() => Uri.parse('$normalizedApiBaseUrl/health');
}
