import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:anylive_mobile/api/api_client.dart';
import 'package:anylive_mobile/api/gifts_repository.dart';

void main() {
  group('GiftsRepository', () {
    test('listGifts parses catalog', () async {
      final mock = MockClient((request) async {
        expect(request.url.path, '/api/v1/gifts');
        return http.Response(
          jsonEncode({
            'items': [
              {'id': 'g1', 'name': 'Rose', 'price': 1},
            ]
          }),
          200,
          headers: {'content-type': 'application/json'},
        );
      });
      final repo = GiftsRepository(
        client: ApiClient(baseUrl: 'http://localhost:8088'),
        httpClient: mock,
      );
      final gifts = await repo.listGifts();
      expect(gifts.single.name, 'Rose');
      expect(gifts.single.price, 1);
    });

    test('sendGift posts idempotency key', () async {
      final mock = MockClient((request) async {
        expect(request.url.path, '/api/v1/rooms/r1/gifts');
        final body = jsonDecode(request.body) as Map<String, dynamic>;
        expect(body['client_request_id'], 'idem-1');
        return http.Response(
          jsonEncode({
            'id': 'o1',
            'room_id': 'r1',
            'sender_id': 's',
            'receiver_id': 'r',
            'gift_id': 'g1',
            'count': 1,
            'total_coins': 1,
            'client_request_id': 'idem-1',
            'replayed': false,
          }),
          201,
          headers: {'content-type': 'application/json'},
        );
      });
      final api = ApiClient(baseUrl: 'http://x', accessToken: 't');
      final repo = GiftsRepository(client: api, httpClient: mock);
      final order = await repo.sendGift(
        roomId: 'r1',
        giftId: 'g1',
        receiverId: 'r',
        clientRequestId: 'idem-1',
      );
      expect(order.id, 'o1');
      expect(order.replayed, isFalse);
    });

    test('listLedger parses items', () async {
      final mock = MockClient((request) async {
        expect(request.url.path, '/api/v1/wallet/ledger');
        expect(request.headers['Authorization'], 'Bearer t');
        return http.Response(
          jsonEncode({
            'items': [
              {
                'id': 'le1',
                'user_id': 'u1',
                'amount': 100,
                'balance_after': 100,
                'entry_type': 'topup',
                'reference': 'mock-topup',
                'created_at': '2026-01-01T00:00:00Z',
              },
            ]
          }),
          200,
          headers: {'content-type': 'application/json'},
        );
      });
      final api = ApiClient(baseUrl: 'http://x', accessToken: 't');
      final repo = GiftsRepository(client: api, httpClient: mock);
      final entries = await repo.listLedger();
      expect(entries.single.id, 'le1');
      expect(entries.single.amount, 100);
      expect(entries.single.entryType, 'topup');
    });
  });
}
