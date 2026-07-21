import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:anylive_mobile/api/api_client.dart';
import 'package:anylive_mobile/api/social_repository.dart';

void main() {
  group('SocialRepository', () {
    test('follow posts to user follow endpoint', () async {
      final mock = MockClient((request) async {
        expect(request.url.path, '/api/v1/users/u2/follow');
        expect(request.method, 'POST');
        expect(request.headers['Authorization'], 'Bearer tok');
        return http.Response('', 204);
      });
      final api = ApiClient(baseUrl: 'http://localhost:8088', accessToken: 'tok');
      final repo = SocialRepository(client: api, httpClient: mock);
      await repo.follow('u2');
    });

    test('unfollow deletes follow relation', () async {
      final mock = MockClient((request) async {
        expect(request.url.path, '/api/v1/users/u2/follow');
        expect(request.method, 'DELETE');
        expect(request.headers['Authorization'], 'Bearer tok');
        return http.Response('', 204);
      });
      final api = ApiClient(baseUrl: 'http://x', accessToken: 'tok');
      final repo = SocialRepository(client: api, httpClient: mock);
      await repo.unfollow('u2');
    });

    test('listFollowing parses user_ids', () async {
      final mock = MockClient((request) async {
        expect(request.url.path, '/api/v1/me/following');
        expect(request.method, 'GET');
        expect(request.headers['Authorization'], 'Bearer tok');
        return http.Response(
          jsonEncode({
            'user_ids': ['u2', 'u3'],
          }),
          200,
          headers: {'content-type': 'application/json'},
        );
      });
      final api = ApiClient(baseUrl: 'http://x', accessToken: 'tok');
      final repo = SocialRepository(client: api, httpClient: mock);
      final ids = await repo.listFollowing();
      expect(ids, ['u2', 'u3']);
    });

    test('feedHot parses room items', () async {
      final mock = MockClient((request) async {
        expect(request.url.path, '/api/v1/feed/hot');
        expect(request.method, 'GET');
        return http.Response(
          jsonEncode({
            'items': [
              {
                'id': 'r1',
                'owner_id': 'u1',
                'title': 'Hot Show',
                'status': 'live',
              }
            ]
          }),
          200,
          headers: {'content-type': 'application/json'},
        );
      });
      final repo = SocialRepository(
        client: ApiClient(baseUrl: 'http://localhost:8088'),
        httpClient: mock,
      );
      final rooms = await repo.feedHot();
      expect(rooms.length, 1);
      expect(rooms.first.id, 'r1');
      expect(rooms.first.title, 'Hot Show');
      expect(rooms.first.isLive, isTrue);
    });

    test('feedFollowing sends auth and parses items', () async {
      final mock = MockClient((request) async {
        expect(request.url.path, '/api/v1/feed/following');
        expect(request.method, 'GET');
        expect(request.headers['Authorization'], 'Bearer tok');
        return http.Response(
          jsonEncode({
            'items': [
              {
                'id': 'r2',
                'owner_id': 'u2',
                'title': 'Followed Live',
                'status': 'live',
              }
            ]
          }),
          200,
          headers: {'content-type': 'application/json'},
        );
      });
      final api = ApiClient(baseUrl: 'http://x', accessToken: 'tok');
      final repo = SocialRepository(client: api, httpClient: mock);
      final rooms = await repo.feedFollowing();
      expect(rooms.single.title, 'Followed Live');
      expect(rooms.single.ownerId, 'u2');
    });

    test('follow throws on error status', () async {
      final mock = MockClient((_) async => http.Response('nope', 400));
      final repo = SocialRepository(
        client: ApiClient(baseUrl: 'http://x', accessToken: 't'),
        httpClient: mock,
      );
      expect(() => repo.follow('u2'), throwsA(isA<SocialException>()));
    });

    test('feedHot throws on error status', () async {
      final mock = MockClient((_) async => http.Response('err', 500));
      final repo = SocialRepository(
        client: ApiClient(baseUrl: 'http://x'),
        httpClient: mock,
      );
      expect(() => repo.feedHot(), throwsA(isA<SocialException>()));
    });
  });
}
