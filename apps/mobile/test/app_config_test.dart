import 'package:anylive_mobile/config/app_config.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  group('AppConfig', () {
    test('normalizes trailing slash on api base url', () {
      const cfg = AppConfig(
        apiBaseUrl: 'http://localhost:8088/',
        environment: 'local',
      );
      expect(cfg.normalizedApiBaseUrl, 'http://localhost:8088');
      expect(cfg.isLocal, isTrue);
    });

    test('healthUri points at /health', () {
      const cfg = AppConfig(
        apiBaseUrl: 'http://example.com',
        environment: 'stage',
      );
      expect(cfg.healthUri().toString(), 'http://example.com/health');
      expect(cfg.isLocal, isFalse);
    });

    test('fromEnvironment uses defaults', () {
      final cfg = AppConfig.fromEnvironment();
      expect(cfg.apiBaseUrl, isNotEmpty);
      expect(cfg.environment, isNotEmpty);
    });
  });
}
