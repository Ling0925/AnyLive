import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:anylive_mobile/api/api_client.dart';
import 'package:anylive_mobile/api/rooms_repository.dart';

void main() {
  group('RoomsRepository', () {
    test('listRooms parses items', () async {
      final mock = MockClient((request) async {
        expect(request.url.path, '/api/v1/rooms');
        return http.Response(
          jsonEncode({
            'items': [
              {
                'id': 'r1',
                'owner_id': 'u1',
                'title': 'Show',
                'status': 'live',
                'created_at': 't',
                'updated_at': 't',
              }
            ]
          }),
          200,
          headers: {'content-type': 'application/json'},
        );
      });
      final repo = RoomsRepository(
        client: ApiClient(baseUrl: 'http://localhost:8088'),
        httpClient: mock,
      );
      final rooms = await repo.listRooms();
      expect(rooms.length, 1);
      expect(rooms.first.isLive, isTrue);
      expect(rooms.first.title, 'Show');
    });

    test('createRoom requires auth header', () async {
      final mock = MockClient((request) async {
        expect(request.headers['Authorization'], 'Bearer tok');
        return http.Response(
          jsonEncode({
            'id': 'r2',
            'owner_id': 'u1',
            'title': 'New',
            'status': 'idle',
            'created_at': 't',
            'updated_at': 't',
          }),
          201,
          headers: {'content-type': 'application/json'},
        );
      });
      final api = ApiClient(baseUrl: 'http://x', accessToken: 'tok');
      final repo = RoomsRepository(client: api, httpClient: mock);
      final room = await repo.createRoom('New');
      expect(room.id, 'r2');
    });

    test('getRoom parses body', () async {
      final mock = MockClient((request) async {
        expect(request.url.path, '/api/v1/rooms/r1');
        return http.Response(
          jsonEncode({
            'id': 'r1',
            'owner_id': 'u1',
            'title': 'Show',
            'status': 'live',
          }),
          200,
          headers: {'content-type': 'application/json'},
        );
      });
      final repo = RoomsRepository(
        client: ApiClient(baseUrl: 'http://localhost:8088'),
        httpClient: mock,
      );
      final room = await repo.getRoom('r1');
      expect(room.title, 'Show');
      expect(room.isLive, isTrue);
    });

    test('listMessages and postMessage', () async {
      final mock = MockClient((request) async {
        if (request.method == 'GET') {
          expect(request.url.path, '/api/v1/rooms/r1/messages');
          return http.Response(
            jsonEncode({
              'items': [
                {
                  'id': 'm1',
                  'room_id': 'r1',
                  'sender_id': 'u1',
                  'sender_name': 'Alice',
                  'body': 'hi',
                  'created_at': 't',
                }
              ]
            }),
            200,
            headers: {'content-type': 'application/json'},
          );
        }
        expect(request.method, 'POST');
        expect(request.headers['Authorization'], 'Bearer tok');
        final body = jsonDecode(request.body) as Map<String, dynamic>;
        expect(body['body'], 'hello');
        return http.Response(
          jsonEncode({
            'id': 'm2',
            'room_id': 'r1',
            'sender_id': 'u1',
            'sender_name': 'Alice',
            'body': 'hello',
            'created_at': 't',
          }),
          201,
          headers: {'content-type': 'application/json'},
        );
      });
      final api = ApiClient(baseUrl: 'http://x', accessToken: 'tok');
      final repo = RoomsRepository(client: api, httpClient: mock);
      final list = await repo.listMessages('r1');
      expect(list.single.body, 'hi');
      final posted = await repo.postMessage('r1', 'hello');
      expect(posted.body, 'hello');
    });

    test('playUrls returns hls', () async {
      final mock = MockClient((request) async {
        expect(request.url.path, '/api/v1/rooms/r1/media/play');
        return http.Response(
          jsonEncode({'hls': 'http://cdn/live/r1.m3u8'}),
          200,
          headers: {'content-type': 'application/json'},
        );
      });
      final repo = RoomsRepository(
        client: ApiClient(baseUrl: 'http://x'),
        httpClient: mock,
      );
      final play = await repo.playUrls('r1');
      expect(play['hls'], 'http://cdn/live/r1.m3u8');
    });
  });
}
