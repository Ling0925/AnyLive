import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:anylive_mobile/api/api_client.dart';
import 'package:anylive_mobile/api/auth_repository.dart';

void main() {
  group('AuthRepository', () {
    test('passwordLogin parses session and sets token', () async {
      final mock = MockClient((request) async {
        expect(request.url.path, '/api/v1/auth/password/login');
        final body = jsonDecode(request.body) as Map<String, dynamic>;
        expect(body['identifier'], 'host1');
        expect(body['password'], 'secret');
        return http.Response(
          jsonEncode({
            'user': {
              'id': 'u1',
              'display_name': 'Host',
              'username': 'host1',
              'email': 'h@example.com',
              'created_at': '2026-01-01T00:00:00Z',
            },
            'access_token': 'acc',
            'refresh_token': 'ref',
            'expires_in': 900,
            'must_change_password': true,
          }),
          200,
          headers: {'content-type': 'application/json'},
        );
      });
      final api = ApiClient(baseUrl: 'http://localhost:8088');
      final repo = AuthRepository(client: api, httpClient: mock);
      final session = await repo.passwordLogin(
        identifier: 'host1',
        password: 'secret',
      );
      expect(session.accessToken, 'acc');
      expect(session.username, 'host1');
      expect(session.mustChangePassword, isTrue);
      expect(api.accessToken, 'acc');
    });

    test('passwordLogin throws on error status', () async {
      final mock = MockClient((_) async => http.Response('nope', 401));
      final repo = AuthRepository(
        client: ApiClient(baseUrl: 'http://x'),
        httpClient: mock,
      );
      expect(
        () => repo.passwordLogin(identifier: 'x', password: 'y'),
        throwsA(isA<AuthException>()),
      );
    });

    test('changePassword expects 204', () async {
      final mock = MockClient((request) async {
        expect(request.url.path, '/api/v1/auth/password/change');
        expect(request.headers['authorization'], 'Bearer tok');
        return http.Response('', 204);
      });
      final api = ApiClient(baseUrl: 'http://localhost:8088', accessToken: 'tok');
      final repo = AuthRepository(client: api, httpClient: mock);
      await repo.changePassword(
        currentPassword: 'oldpass12',
        newPassword: 'newpass12',
      );
    });

    test('sendOtp expects 204', () async {
      final mock = MockClient((request) async {
        expect(request.url.path, '/api/v1/auth/otp/send');
        expect(jsonDecode(request.body)['email'], 'a@b.com');
        return http.Response('', 204);
      });
      final repo = AuthRepository(
        client: ApiClient(baseUrl: 'http://localhost:8088'),
        httpClient: mock,
      );
      await repo.sendOtp('a@b.com');
    });

    test('verifyOtp parses session and sets token', () async {
      final mock = MockClient((request) async {
        expect(request.url.path, '/api/v1/auth/otp/verify');
        return http.Response(
          jsonEncode({
            'user': {
              'id': 'u1',
              'display_name': 'a',
              'email': 'a@b.com',
              'created_at': '2026-01-01T00:00:00Z',
            },
            'access_token': 'acc',
            'refresh_token': 'ref',
            'expires_in': 900,
          }),
          200,
          headers: {'content-type': 'application/json'},
        );
      });
      final api = ApiClient(baseUrl: 'http://localhost:8088');
      final repo = AuthRepository(client: api, httpClient: mock);
      final session = await repo.verifyOtp(email: 'a@b.com', code: '123456');
      expect(session.accessToken, 'acc');
      expect(session.email, 'a@b.com');
      expect(api.accessToken, 'acc');
    });

    test('verifyOtp throws on error status', () async {
      final mock = MockClient((_) async => http.Response('nope', 401));
      final repo = AuthRepository(
        client: ApiClient(baseUrl: 'http://x'),
        httpClient: mock,
      );
      expect(
        () => repo.verifyOtp(email: 'a@b.com', code: '000000'),
        throwsA(isA<AuthException>()),
      );
    });
  });
}
