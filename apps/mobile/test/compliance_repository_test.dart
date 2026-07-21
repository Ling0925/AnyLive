import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:anylive_mobile/api/api_client.dart';
import 'package:anylive_mobile/api/compliance_repository.dart';

void main() {
  group('ComplianceRepository', () {
    test('legalPrivacy parses doc', () async {
      final mock = MockClient((request) async {
        expect(request.url.path, '/api/v1/legal/privacy');
        expect(request.method, 'GET');
        return http.Response(
          jsonEncode({
            'url': 'https://anylive.example/privacy',
            'version': '1.0',
            'title': 'Privacy Policy',
          }),
          200,
          headers: {'content-type': 'application/json'},
        );
      });
      final repo = ComplianceRepository(
        client: ApiClient(baseUrl: 'http://localhost:8088'),
        httpClient: mock,
      );
      final doc = await repo.legalPrivacy();
      expect(doc.title, 'Privacy Policy');
      expect(doc.url, 'https://anylive.example/privacy');
      expect(doc.version, '1.0');
    });

    test('legalTerms parses doc', () async {
      final mock = MockClient((request) async {
        expect(request.url.path, '/api/v1/legal/terms');
        return http.Response(
          jsonEncode({
            'url': 'https://anylive.example/terms',
            'version': '1.0',
            'title': 'Terms of Service',
          }),
          200,
          headers: {'content-type': 'application/json'},
        );
      });
      final repo = ComplianceRepository(
        client: ApiClient(baseUrl: 'http://localhost:8088'),
        httpClient: mock,
      );
      final doc = await repo.legalTerms();
      expect(doc.title, 'Terms of Service');
      expect(doc.url, 'https://anylive.example/terms');
    });

    test('exportMe returns map and sends auth', () async {
      final mock = MockClient((request) async {
        expect(request.url.path, '/api/v1/me/export');
        expect(request.headers['Authorization'], 'Bearer tok');
        return http.Response(
          jsonEncode({
            'user': {
              'id': 'u1',
              'display_name': 'Ada',
              'email': 'ada@example.com',
              'created_at': '2026-01-01T00:00:00Z',
            },
            'rooms_owned_count': 0,
            'note': 'P1 export stub',
          }),
          200,
          headers: {'content-type': 'application/json'},
        );
      });
      final api = ApiClient(baseUrl: 'http://x', accessToken: 'tok');
      final repo = ComplianceRepository(client: api, httpClient: mock);
      final exp = await repo.exportMe();
      expect(exp['rooms_owned_count'], 0);
      expect(exp['note'], 'P1 export stub');
      final user = exp['user'] as Map<String, dynamic>;
      expect(user['id'], 'u1');
      expect(user['display_name'], 'Ada');
      expect(user['email'], 'ada@example.com');
    });

    test('deleteMe expects 204', () async {
      final mock = MockClient((request) async {
        expect(request.url.path, '/api/v1/me');
        expect(request.method, 'DELETE');
        expect(request.headers['Authorization'], 'Bearer tok');
        return http.Response('', 204);
      });
      final api = ApiClient(baseUrl: 'http://x', accessToken: 'tok');
      final repo = ComplianceRepository(client: api, httpClient: mock);
      await repo.deleteMe();
    });

    test('exportMe throws on error status', () async {
      final mock = MockClient((_) async => http.Response('nope', 401));
      final repo = ComplianceRepository(
        client: ApiClient(baseUrl: 'http://x', accessToken: 't'),
        httpClient: mock,
      );
      expect(
        () => repo.exportMe(),
        throwsA(isA<ComplianceException>()),
      );
    });

    test('deleteMe throws on non-204', () async {
      final mock = MockClient((_) async => http.Response('fail', 500));
      final repo = ComplianceRepository(
        client: ApiClient(baseUrl: 'http://x', accessToken: 't'),
        httpClient: mock,
      );
      expect(
        () => repo.deleteMe(),
        throwsA(isA<ComplianceException>()),
      );
    });
  });
}
