import 'dart:convert';

import 'package:anylive_mobile/api/api_client.dart';
import 'package:anylive_mobile/api/gifts_repository.dart';
import 'package:anylive_mobile/api/reports_repository.dart';
import 'package:anylive_mobile/api/rooms_repository.dart';
import 'package:anylive_mobile/api/social_repository.dart';
import 'package:anylive_mobile/config/app_config.dart';
import 'package:anylive_mobile/features/rooms/room_page.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';

void main() {
  const config = AppConfig(
    apiBaseUrl: 'http://localhost:8088',
    environment: 'local',
  );

  final room = Room(
    id: 'r1',
    ownerId: 'owner-1',
    title: 'Night Show',
    status: 'live',
  );

  MockClient mockClient({String roomStatus = 'live', List<String> following = const []}) {
    return MockClient((request) async {
      final path = request.url.path;
      final headers = {'content-type': 'application/json'};

      if (path == '/api/v1/rooms/r1' && request.method == 'GET') {
        return http.Response(
          jsonEncode({
            'id': 'r1',
            'owner_id': 'owner-1',
            'title': 'Night Show',
            'status': roomStatus,
          }),
          200,
          headers: headers,
        );
      }
      if (path == '/api/v1/rooms/r1/messages' && request.method == 'GET') {
        return http.Response(
          jsonEncode({
            'items': [
              {
                'id': 'm1',
                'room_id': 'r1',
                'sender_id': 'u2',
                'sender_name': 'Bob',
                'body': 'hello room',
                'created_at': 't',
              }
            ]
          }),
          200,
          headers: headers,
        );
      }
      if (path == '/api/v1/rooms/r1/messages' && request.method == 'POST') {
        final body = jsonDecode(request.body) as Map<String, dynamic>;
        return http.Response(
          jsonEncode({
            'id': 'm2',
            'room_id': 'r1',
            'sender_id': 'me',
            'sender_name': 'Me',
            'body': body['body'],
            'created_at': 't',
          }),
          201,
          headers: headers,
        );
      }
      if (path == '/api/v1/rooms/r1/media/play') {
        return http.Response(
          jsonEncode({'hls': 'http://cdn/live/r1.m3u8'}),
          200,
          headers: headers,
        );
      }
      if (path == '/api/v1/gifts') {
        return http.Response(
          jsonEncode({
            'items': [
              {'id': 'g1', 'name': 'Rose', 'price': 1},
            ]
          }),
          200,
          headers: headers,
        );
      }
      if (path == '/api/v1/wallet') {
        return http.Response(
          jsonEncode({'balance': 42}),
          200,
          headers: headers,
        );
      }
      if (path == '/api/v1/wallet/topups') {
        return http.Response(
          jsonEncode({'balance': 142}),
          200,
          headers: headers,
        );
      }
      if (path == '/api/v1/rooms/r1/gifts' && request.method == 'POST') {
        final body = jsonDecode(request.body) as Map<String, dynamic>;
        expect(body['client_request_id'], isNotEmpty);
        expect(body['gift_id'], 'g1');
        expect(body['receiver_id'], 'owner-1');
        return http.Response(
          jsonEncode({
            'id': 'o1',
            'total_coins': 1,
            'replayed': false,
          }),
          201,
          headers: headers,
        );
      }
      if (path == '/api/v1/me/following' && request.method == 'GET') {
        return http.Response(
          jsonEncode({'user_ids': following}),
          200,
          headers: headers,
        );
      }
      if (path == '/api/v1/users/owner-1/follow' && request.method == 'POST') {
        return http.Response('', 204);
      }
      if (path == '/api/v1/users/owner-1/follow' && request.method == 'DELETE') {
        return http.Response('', 204);
      }
      if (path == '/api/v1/reports' && request.method == 'POST') {
        final body = jsonDecode(request.body) as Map<String, dynamic>;
        expect(body['target_type'], 'room');
        expect(body['target_id'], 'r1');
        return http.Response(
          jsonEncode({
            'id': 'rep1',
            'target_type': body['target_type'],
            'target_id': body['target_id'],
            'reason': body['reason'],
            'status': 'open',
            'created_at': 't',
          }),
          201,
          headers: headers,
        );
      }
      return http.Response('not found: $path', 404);
    });
  }

  (RoomsRepository, GiftsRepository, SocialRepository, ReportsRepository)
      buildRepos(MockClient httpClient) {
    final api = ApiClient(
      baseUrl: config.normalizedApiBaseUrl,
      accessToken: 'tok',
    );
    return (
      RoomsRepository(client: api, httpClient: httpClient),
      GiftsRepository(client: api, httpClient: httpClient),
      SocialRepository(client: api, httpClient: httpClient),
      ReportsRepository(client: api, httpClient: httpClient),
    );
  }

  testWidgets('room page shows title status hls chat gifts wallet',
      (tester) async {
    final httpClient = mockClient();
    final (rooms, gifts, social, reports) = buildRepos(httpClient);

    await tester.pumpWidget(
      MaterialApp(
        home: RoomPage(
          config: config,
          accessToken: 'tok',
          room: room,
          roomsRepository: rooms,
          giftsRepository: gifts,
          socialRepository: social,
          reportsRepository: reports,
        ),
      ),
    );

    // Loading first.
    expect(find.byType(CircularProgressIndicator), findsOneWidget);

    await tester.pumpAndSettle();

    expect(find.text('Night Show'), findsWidgets);
    expect(find.text('live'), findsOneWidget);
    expect(find.text('http://cdn/live/r1.m3u8'), findsOneWidget);
    expect(find.textContaining('Balance: 42'), findsOneWidget);
    expect(find.text('Top up'), findsOneWidget);
    expect(find.textContaining('Bob:'), findsOneWidget);
    expect(find.textContaining('hello room'), findsOneWidget);
    expect(find.text('Rose (1)'), findsOneWidget);
    expect(find.byType(TextField), findsOneWidget);
    expect(find.text('Follow'), findsOneWidget);
  });

  testWidgets('room page send chat appends message', (tester) async {
    final httpClient = mockClient();
    final (rooms, gifts, social, reports) = buildRepos(httpClient);

    await tester.pumpWidget(
      MaterialApp(
        home: RoomPage(
          config: config,
          accessToken: 'tok',
          room: room,
          roomsRepository: rooms,
          giftsRepository: gifts,
          socialRepository: social,
          reportsRepository: reports,
        ),
      ),
    );
    await tester.pumpAndSettle();

    await tester.enterText(find.byType(TextField), 'nice stream');
    await tester.tap(find.byIcon(Icons.send));
    await tester.pumpAndSettle();

    expect(find.textContaining('nice stream'), findsOneWidget);
  });

  testWidgets('room page topup updates balance', (tester) async {
    final httpClient = mockClient();
    final (rooms, gifts, social, reports) = buildRepos(httpClient);

    await tester.pumpWidget(
      MaterialApp(
        home: RoomPage(
          config: config,
          accessToken: 'tok',
          room: room,
          roomsRepository: rooms,
          giftsRepository: gifts,
          socialRepository: social,
          reportsRepository: reports,
        ),
      ),
    );
    await tester.pumpAndSettle();

    await tester.tap(find.text('Top up'));
    await tester.pumpAndSettle();

    expect(find.textContaining('Balance: 142'), findsOneWidget);
  });

  testWidgets('room page follow toggles label', (tester) async {
    final httpClient = mockClient();
    final (rooms, gifts, social, reports) = buildRepos(httpClient);

    await tester.pumpWidget(
      MaterialApp(
        home: RoomPage(
          config: config,
          accessToken: 'tok',
          room: room,
          roomsRepository: rooms,
          giftsRepository: gifts,
          socialRepository: social,
          reportsRepository: reports,
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('Follow'), findsOneWidget);
    await tester.tap(find.text('Follow'));
    await tester.pumpAndSettle();
    expect(find.text('Unfollow'), findsOneWidget);
  });

  testWidgets('room page report dialog submits', (tester) async {
    final httpClient = mockClient();
    final (rooms, gifts, social, reports) = buildRepos(httpClient);

    await tester.pumpWidget(
      MaterialApp(
        home: RoomPage(
          config: config,
          accessToken: 'tok',
          room: room,
          roomsRepository: rooms,
          giftsRepository: gifts,
          socialRepository: social,
          reportsRepository: reports,
        ),
      ),
    );
    await tester.pumpAndSettle();

    await tester.tap(find.byIcon(Icons.flag_outlined));
    await tester.pumpAndSettle();

    expect(find.text('Report room'), findsOneWidget);
    // Dialog TextField (chat field is still under the page).
    final dialogField = find.descendant(
      of: find.byType(AlertDialog),
      matching: find.byType(TextField),
    );
    await tester.enterText(dialogField, 'spam');
    await tester.tap(find.text('Submit'));
    await tester.pumpAndSettle();

    expect(find.text('Report submitted'), findsOneWidget);
  });

  testWidgets('room page shows ended banner and hides hls', (tester) async {
    final httpClient = mockClient(roomStatus: 'closed');
    final (rooms, gifts, social, reports) = buildRepos(httpClient);
    final closedRoom = Room(
      id: 'r1',
      ownerId: 'owner-1',
      title: 'Night Show',
      status: 'closed',
    );

    await tester.pumpWidget(
      MaterialApp(
        home: RoomPage(
          config: config,
          accessToken: 'tok',
          room: closedRoom,
          roomsRepository: rooms,
          giftsRepository: gifts,
          socialRepository: social,
          reportsRepository: reports,
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('Room ended'), findsOneWidget);
    expect(find.text('http://cdn/live/r1.m3u8'), findsNothing);
  });

  testWidgets('room page copy stream url shows snackbar', (tester) async {
    final httpClient = mockClient();
    final (rooms, gifts, social, reports) = buildRepos(httpClient);

    await tester.pumpWidget(
      MaterialApp(
        home: RoomPage(
          config: config,
          accessToken: 'tok',
          room: room,
          roomsRepository: rooms,
          giftsRepository: gifts,
          socialRepository: social,
          reportsRepository: reports,
        ),
      ),
    );
    await tester.pumpAndSettle();

    final copyBtn = find.byKey(const Key('copy-stream-url'));
    expect(copyBtn, findsOneWidget);
    await tester.ensureVisible(copyBtn);
    await tester.tap(copyBtn, warnIfMissed: true);
    await tester.pump(); // schedule snackbar
    await tester.pump(const Duration(milliseconds: 750));

    expect(
      find.text('Copied - open in VLC / browser player'),
      findsOneWidget,
    );
  });
}
