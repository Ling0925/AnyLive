/// Runtime configuration for AnyLive mobile.
class AppConfig {
  const AppConfig({
    required this.apiBaseUrl,
    required this.environment,
  });

  final String apiBaseUrl;
  final String environment;

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
    return AppConfig(apiBaseUrl: api, environment: env);
  }

  bool get isLocal => environment == 'local';

  /// Normalize base URL (no trailing slash).
  String get normalizedApiBaseUrl {
    if (apiBaseUrl.endsWith('/')) {
      return apiBaseUrl.substring(0, apiBaseUrl.length - 1);
    }
    return apiBaseUrl;
  }

  Uri healthUri() => Uri.parse('$normalizedApiBaseUrl/health');
}
