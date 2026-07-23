import 'dart:convert';

import 'package:anylive_mobile/api/api_client.dart';
import 'package:anylive_mobile/api/realtime_repository.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';

void main() {
  test('connectionToken parses response', () async {
    final httpClient = MockClient((request) async {
      expect(request.url.path, '/api/v1/realtime/token');
      expect(request.headers['Authorization'], 'Bearer tok');
      return http.Response(
        jsonEncode({
          'token': 'cf-jwt',
          'expires_in': 600,
          'channels': ['room:r1'],
        }),
        200,
        headers: {'content-type': 'application/json'},
      );
    });
    final api = ApiClient(baseUrl: 'http://localhost:8088', accessToken: 'tok');
    final repo = RealtimeRepository(client: api, httpClient: httpClient);
    final token = await repo.connectionToken('r1');
    expect(token.token, 'cf-jwt');
    expect(token.expiresIn, 600);
    expect(token.channels, ['room:r1']);
  });

  test('listMessages parses items', () async {
    final httpClient = MockClient((request) async {
      expect(request.url.path, '/api/v1/rooms/r1/messages');
      return http.Response(
        jsonEncode({
          'items': [
            {
              'id': 'm1',
              'room_id': 'r1',
              'sender_id': 'u1',
              'sender_name': 'Ada',
              'body': 'hi',
              'created_at': 't',
            }
          ]
        }),
        200,
        headers: {'content-type': 'application/json'},
      );
    });
    final api = ApiClient(baseUrl: 'http://localhost:8088');
    final repo = RealtimeRepository(client: api, httpClient: httpClient);
    final msgs = await repo.listMessages('r1');
    expect(msgs, hasLength(1));
    expect(msgs.first.body, 'hi');
  });
}
