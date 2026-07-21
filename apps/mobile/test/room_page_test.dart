import 'dart:convert';

import 'package:anylive_mobile/api/api_client.dart';
import 'package:anylive_mobile/api/gifts_repository.dart';
import 'package:anylive_mobile/api/rooms_repository.dart';
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

  MockClient mockClient() {
    return MockClient((request) async {
      final path = request.url.path;
      final headers = {'content-type': 'application/json'};

      if (path == '/api/v1/rooms/r1' && request.method == 'GET') {
        return http.Response(
          jsonEncode({
            'id': 'r1',
            'owner_id': 'owner-1',
            'title': 'Night Show',
            'status': 'live',
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
      return http.Response('not found: $path', 404);
    });
  }

  testWidgets('room page shows title status hls chat gifts wallet',
      (tester) async {
    final httpClient = mockClient();
    final api = ApiClient(
      baseUrl: config.normalizedApiBaseUrl,
      accessToken: 'tok',
    );
    final rooms = RoomsRepository(client: api, httpClient: httpClient);
    final gifts = GiftsRepository(client: api, httpClient: httpClient);

    await tester.pumpWidget(
      MaterialApp(
        home: RoomPage(
          config: config,
          accessToken: 'tok',
          room: room,
          roomsRepository: rooms,
          giftsRepository: gifts,
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
  });

  testWidgets('room page send chat appends message', (tester) async {
    final httpClient = mockClient();
    final api = ApiClient(
      baseUrl: config.normalizedApiBaseUrl,
      accessToken: 'tok',
    );
    final rooms = RoomsRepository(client: api, httpClient: httpClient);
    final gifts = GiftsRepository(client: api, httpClient: httpClient);

    await tester.pumpWidget(
      MaterialApp(
        home: RoomPage(
          config: config,
          accessToken: 'tok',
          room: room,
          roomsRepository: rooms,
          giftsRepository: gifts,
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
    final api = ApiClient(
      baseUrl: config.normalizedApiBaseUrl,
      accessToken: 'tok',
    );
    final rooms = RoomsRepository(client: api, httpClient: httpClient);
    final gifts = GiftsRepository(client: api, httpClient: httpClient);

    await tester.pumpWidget(
      MaterialApp(
        home: RoomPage(
          config: config,
          accessToken: 'tok',
          room: room,
          roomsRepository: rooms,
          giftsRepository: gifts,
        ),
      ),
    );
    await tester.pumpAndSettle();

    await tester.tap(find.text('Top up'));
    await tester.pumpAndSettle();

    expect(find.textContaining('Balance: 142'), findsOneWidget);
  });
}
