/// Runtime configuration for AnyLive mobile.
///
/// Flavors (WBS E8.1): pass `--dart-define=APP_FLAVOR=local|stage|prod`.
/// When unset, [flavor] falls back to [environment] / `APP_ENV`.
class AppConfig {
  const AppConfig({
    required this.apiBaseUrl,
    required this.environment,
    this.flavor = 'local',
    this.centrifugoWsUrl,
  });

  final String apiBaseUrl;
  final String environment;

  /// Build flavor: `local` | `stage` | `prod` (case-insensitive at parse time).
  final String flavor;

  /// Optional Centrifugo WS base (`ws://host:8000/connection/websocket`).
  /// When null/empty, room chat uses HTTP history poll only.
  final String? centrifugoWsUrl;

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
    final flavor = normalizeFlavor(flavorRaw.isEmpty ? env : flavorRaw);
    return AppConfig(
      apiBaseUrl: api,
      environment: env,
      flavor: flavor,
      centrifugoWsUrl: ws.isEmpty ? null : ws,
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

  Uri healthUri() => Uri.parse('$normalizedApiBaseUrl/health');
}
