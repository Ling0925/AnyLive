import 'package:flutter_test/flutter_test.dart';
import 'package:anylive_mobile/api/api_client.dart';

void main() {
  group('ApiClient', () {
    test('builds absolute uri and strips trailing slash', () {
      final c = ApiClient(baseUrl: 'http://localhost:8088/');
      expect(c.uri('/health').toString(), 'http://localhost:8088/health');
      expect(c.uri('api/v1/me').toString(), 'http://localhost:8088/api/v1/me');
    });

    test('adds bearer when accessToken set', () {
      final c = ApiClient(baseUrl: 'http://x', accessToken: 'tok');
      expect(c.jsonHeaders(auth: true)['Authorization'], 'Bearer tok');
      expect(c.jsonHeaders(auth: false).containsKey('Authorization'), isFalse);
    });
  });
}
