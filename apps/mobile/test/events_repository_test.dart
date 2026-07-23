import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:anylive_mobile/api/api_client.dart';
import 'package:anylive_mobile/api/events_repository.dart';

void main() {
  group('EventsRepository', () {
    test('ingest posts batch and parses 202', () async {
      final mock = MockClient((request) async {
        expect(request.url.path, '/api/v1/events');
        expect(request.method, 'POST');
        expect(request.headers['Authorization'], 'Bearer tok');
        final body = jsonDecode(request.body) as Map<String, dynamic>;
        final events = body['events'] as List<dynamic>;
        expect(events, hasLength(1));
        expect((events.first as Map)['name'], 'room.view');
        return http.Response(
          jsonEncode({'accepted': 1, 'dropped': 0}),
          202,
          headers: {'content-type': 'application/json'},
        );
      });
      final repo = EventsRepository(
        client: ApiClient(baseUrl: 'http://localhost:8088', accessToken: 'tok'),
        httpClient: mock,
      );
      final result = await repo.ingest([
        ClientEvent(name: 'room.view', props: {'room_id': 'r1'}),
      ]);
      expect(result.accepted, 1);
      expect(result.dropped, 0);
    });

    test('ingest throws on error status', () async {
      final mock = MockClient((_) async => http.Response('denied', 403));
      final repo = EventsRepository(
        client: ApiClient(baseUrl: 'http://x', accessToken: 't'),
        httpClient: mock,
      );
      expect(
        () => repo.ingest([ClientEvent(name: 'x')]),
        throwsA(isA<EventsException>()),
      );
    });

    test('track swallows errors', () async {
      final mock = MockClient((_) async => http.Response('nope', 500));
      final repo = EventsRepository(
        client: ApiClient(baseUrl: 'http://x', accessToken: 't'),
        httpClient: mock,
      );
      await repo.track('gift.tap'); // must not throw
    });
  });
}
