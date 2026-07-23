import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:anylive_mobile/api/api_client.dart';
import 'package:anylive_mobile/api/interactive_repository.dart';

void main() {
  group('InteractiveRepository', () {
    test('invite posts invitee and parses session', () async {
      final mock = MockClient((request) async {
        expect(request.url.path, '/api/v1/rooms/r1/interactive/invite');
        expect(request.method, 'POST');
        final body = jsonDecode(request.body) as Map<String, dynamic>;
        expect(body['invitee_id'], 'u2');
        return http.Response(
          jsonEncode({
            'id': 's1',
            'room_id': 'r1',
            'host_id': 'u1',
            'invitee_id': 'u2',
            'status': 'invited',
            'created_at': 't',
            'updated_at': 't',
          }),
          201,
          headers: {'content-type': 'application/json'},
        );
      });
      final repo = InteractiveRepository(
        client: ApiClient(baseUrl: 'http://x', accessToken: 'tok'),
        httpClient: mock,
      );
      final s = await repo.invite(roomId: 'r1', inviteeId: 'u2');
      expect(s.id, 's1');
      expect(s.isInvited, isTrue);
    });

    test('getPk returns null when no session', () async {
      final mock = MockClient((request) async {
        expect(request.url.path, '/api/v1/rooms/r1/pk');
        return http.Response(
          jsonEncode({'session': null}),
          200,
          headers: {'content-type': 'application/json'},
        );
      });
      final repo = InteractiveRepository(
        client: ApiClient(baseUrl: 'http://x', accessToken: 't'),
        httpClient: mock,
      );
      expect(await repo.getPk('r1'), isNull);
    });

    test('getPk returns null on 403 feature-off without throwing', () async {
      final mock = MockClient((request) async {
        expect(request.url.path, '/api/v1/rooms/r1/pk');
        return http.Response(
          '{"code":"FORBIDDEN_POLICY","message":"PK is disabled by feature flag"}',
          403,
          headers: {'content-type': 'application/json'},
        );
      });
      final repo = InteractiveRepository(
        client: ApiClient(baseUrl: 'http://x', accessToken: 't'),
        httpClient: mock,
      );
      expect(await repo.getPk('r1'), isNull);
    });

    test('startPk parses flat session dto', () async {
      final mock = MockClient((request) async {
        expect(request.url.path, '/api/v1/rooms/r1/pk/start');
        final body = jsonDecode(request.body) as Map<String, dynamic>;
        expect(body['opponent_room_id'], 'r2');
        return http.Response(
          jsonEncode({
            'id': 'pk1',
            'room_a_id': 'r1',
            'room_b_id': 'r2',
            'host_a_id': 'u1',
            'host_b_id': 'u2',
            'status': 'active',
            'score_a': 0,
            'score_b': 0,
            'started_at': 't',
            'ends_at': 't2',
            'updated_at': 't',
          }),
          201,
          headers: {'content-type': 'application/json'},
        );
      });
      final repo = InteractiveRepository(
        client: ApiClient(baseUrl: 'http://x', accessToken: 't'),
        httpClient: mock,
      );
      final pk = await repo.startPk(roomId: 'r1', opponentRoomId: 'r2');
      expect(pk.id, 'pk1');
      expect(pk.isActive, isTrue);
    });

    test('livekitJoin sends role and parses credentials', () async {
      final mock = MockClient((request) async {
        expect(request.url.path, '/api/v1/rooms/r1/livekit/join');
        final body = jsonDecode(request.body) as Map<String, dynamic>;
        expect(body['role'], 'host');
        return http.Response(
          jsonEncode({
            'url': 'ws://lk',
            'room_name': 'room-r1',
            'token': 'jwt',
            'identity': 'u1',
            'expires_at': 't',
          }),
          200,
          headers: {'content-type': 'application/json'},
        );
      });
      final repo = InteractiveRepository(
        client: ApiClient(baseUrl: 'http://x', accessToken: 't'),
        httpClient: mock,
      );
      final info = await repo.livekitJoin('r1', role: 'host');
      expect(info.token, 'jwt');
      expect(info.roomName, 'room-r1');
    });

    test('list parses items', () async {
      final mock = MockClient((_) async {
        return http.Response(
          jsonEncode({
            'items': [
              {
                'id': 's1',
                'room_id': 'r1',
                'host_id': 'u1',
                'invitee_id': 'u2',
                'status': 'active',
                'created_at': 't',
                'updated_at': 't',
              },
            ],
          }),
          200,
          headers: {'content-type': 'application/json'},
        );
      });
      final repo = InteractiveRepository(
        client: ApiClient(baseUrl: 'http://x', accessToken: 't'),
        httpClient: mock,
      );
      final items = await repo.list('r1');
      expect(items, hasLength(1));
      expect(items.first.isActive, isTrue);
    });
  });
}
