import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:anylive_mobile/api/api_client.dart';
import 'package:anylive_mobile/api/profile_repository.dart';

void main() {
  group('ProfileRepository', () {
    test('getMe parses profile and sends auth', () async {
      final mock = MockClient((request) async {
        expect(request.url.path, '/api/v1/me');
        expect(request.method, 'GET');
        expect(request.headers['Authorization'], 'Bearer tok');
        return http.Response(
          jsonEncode({
            'id': 'u1',
            'display_name': 'Ada',
            'email': 'ada@example.com',
            'created_at': '2026-01-01T00:00:00Z',
            'age_confirmed': true,
            'privacy_accepted': false,
          }),
          200,
          headers: {'content-type': 'application/json'},
        );
      });
      final api = ApiClient(baseUrl: 'http://localhost:8088', accessToken: 'tok');
      final repo = ProfileRepository(client: api, httpClient: mock);
      final me = await repo.getMe();
      expect(me.id, 'u1');
      expect(me.displayName, 'Ada');
      expect(me.email, 'ada@example.com');
      expect(me.ageConfirmed, isTrue);
      expect(me.privacyAccepted, isFalse);
    });

    test('patchMe sends display_name and parses response', () async {
      final mock = MockClient((request) async {
        expect(request.url.path, '/api/v1/me');
        expect(request.method, 'PATCH');
        expect(request.headers['Authorization'], 'Bearer tok');
        final body = jsonDecode(request.body) as Map<String, dynamic>;
        expect(body['display_name'], 'Patched');
        return http.Response(
          jsonEncode({
            'id': 'u1',
            'display_name': 'Patched',
            'email': 'ada@example.com',
            'created_at': '2026-01-01T00:00:00Z',
            'age_confirmed': false,
            'privacy_accepted': false,
          }),
          200,
          headers: {'content-type': 'application/json'},
        );
      });
      final api = ApiClient(baseUrl: 'http://x', accessToken: 'tok');
      final repo = ProfileRepository(client: api, httpClient: mock);
      final me = await repo.patchMe(displayName: 'Patched');
      expect(me.displayName, 'Patched');
    });

    test('patchMe can send age and privacy flags', () async {
      final mock = MockClient((request) async {
        final body = jsonDecode(request.body) as Map<String, dynamic>;
        expect(body['age_confirmed'], true);
        expect(body['privacy_accepted'], true);
        expect(body.containsKey('display_name'), isFalse);
        return http.Response(
          jsonEncode({
            'id': 'u1',
            'display_name': 'Ada',
            'email': 'ada@example.com',
            'created_at': '2026-01-01T00:00:00Z',
            'age_confirmed': true,
            'privacy_accepted': true,
          }),
          200,
          headers: {'content-type': 'application/json'},
        );
      });
      final api = ApiClient(baseUrl: 'http://x', accessToken: 'tok');
      final repo = ProfileRepository(client: api, httpClient: mock);
      final me = await repo.patchMe(ageConfirmed: true, privacyAccepted: true);
      expect(me.ageConfirmed, isTrue);
      expect(me.privacyAccepted, isTrue);
    });

    test('getMe throws on error status', () async {
      final mock = MockClient((_) async => http.Response('nope', 401));
      final repo = ProfileRepository(
        client: ApiClient(baseUrl: 'http://x', accessToken: 't'),
        httpClient: mock,
      );
      expect(() => repo.getMe(), throwsA(isA<ProfileException>()));
    });

    test('patchMe throws on error status', () async {
      final mock = MockClient((_) async => http.Response('bad', 400));
      final repo = ProfileRepository(
        client: ApiClient(baseUrl: 'http://x', accessToken: 't'),
        httpClient: mock,
      );
      expect(
        () => repo.patchMe(displayName: 'x'),
        throwsA(isA<ProfileException>()),
      );
    });
  });
}
