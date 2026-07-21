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
  });
}
