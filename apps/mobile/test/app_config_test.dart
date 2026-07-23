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
      expect(cfg.flavor, 'local');
    });

    test('healthUri points at /health', () {
      const cfg = AppConfig(
        apiBaseUrl: 'http://example.com',
        environment: 'stage',
        flavor: 'stage',
      );
      expect(cfg.healthUri().toString(), 'http://example.com/health');
      expect(cfg.isLocal, isFalse);
      expect(cfg.isStage, isTrue);
    });

    test('normalizedCentrifugoWsUrl trims slash', () {
      const cfg = AppConfig(
        apiBaseUrl: 'http://localhost:8088',
        environment: 'local',
        centrifugoWsUrl: 'ws://localhost:8000/connection/websocket/',
      );
      expect(
        cfg.normalizedCentrifugoWsUrl,
        'ws://localhost:8000/connection/websocket',
      );
    });

    test('fromEnvironment uses defaults', () {
      final cfg = AppConfig.fromEnvironment();
      expect(cfg.apiBaseUrl, isNotEmpty);
      expect(cfg.environment, isNotEmpty);
      expect(cfg.flavor, isNotEmpty);
    });

    test('normalizeFlavor maps aliases', () {
      expect(AppConfig.normalizeFlavor('production'), 'prod');
      expect(AppConfig.normalizeFlavor('PROD'), 'prod');
      expect(AppConfig.normalizeFlavor('staging'), 'stage');
      expect(AppConfig.normalizeFlavor('dev'), 'local');
      expect(AppConfig.normalizeFlavor(''), 'local');
      expect(AppConfig.normalizeFlavor('custom'), 'custom');
    });
  });
}
