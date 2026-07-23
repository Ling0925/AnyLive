import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:anylive_mobile/api/api_client.dart';
import 'package:anylive_mobile/api/pay_repository.dart';

void main() {
  group('PayRepository', () {
    test('listProducts parses catalog', () async {
      final mock = MockClient((request) async {
        expect(request.url.path, '/api/v1/pay/products');
        return http.Response(
          jsonEncode({
            'items': [
              {
                'id': 'p1',
                'sku': 'coins_100',
                'title': '100 coins',
                'coins': 100,
                'amount': '0.99',
                'currency': 'USD',
              },
            ],
          }),
          200,
          headers: {'content-type': 'application/json'},
        );
      });
      final repo = PayRepository(
        client: ApiClient(baseUrl: 'http://x', accessToken: 't'),
        httpClient: mock,
      );
      final products = await repo.listProducts();
      expect(products, hasLength(1));
      expect(products.first.coins, 100);
      expect(products.first.sku, 'coins_100');
    });

    test('createOrder + sandboxComplete', () async {
      final mock = MockClient((request) async {
        if (request.url.path == '/api/v1/pay/orders' &&
            request.method == 'POST') {
          final body = jsonDecode(request.body) as Map<String, dynamic>;
          expect(body['product_id'], 'p1');
          expect(body['channel'], 'mock');
          return http.Response(
            jsonEncode({
              'id': 'ord1',
              'status': 'pending',
              'coins': 100,
              'amount': '0.99',
              'currency': 'USD',
              'channel': 'mock',
              'mock_hint': 'POST sandbox-complete',
            }),
            201,
            headers: {'content-type': 'application/json'},
          );
        }
        if (request.url.path == '/api/v1/pay/orders/ord1/sandbox-complete') {
          return http.Response(
            jsonEncode({
              'id': 'ord1',
              'status': 'credited',
              'coins': 100,
              'amount': '0.99',
              'currency': 'USD',
              'channel': 'mock',
            }),
            200,
            headers: {'content-type': 'application/json'},
          );
        }
        return http.Response('nope', 404);
      });
      final repo = PayRepository(
        client: ApiClient(baseUrl: 'http://x', accessToken: 't'),
        httpClient: mock,
      );
      final order = await repo.createOrder(productId: 'p1');
      expect(order.id, 'ord1');
      expect(order.status, 'pending');
      final done = await repo.sandboxComplete(order.id);
      expect(done.isCredited, isTrue);
    });
  });
}
