import 'dart:convert';

import 'package:http/http.dart' as http;

import 'api_client.dart';

/// Coin package from GET `/api/v1/pay/products`.
class PayProduct {
  PayProduct({
    required this.id,
    required this.sku,
    required this.title,
    required this.coins,
    required this.amount,
    required this.currency,
  });

  final String id;
  final String sku;
  final String title;
  final int coins;
  final String amount;
  final String currency;

  factory PayProduct.fromJson(Map<String, dynamic> json) {
    return PayProduct(
      id: json['id'] as String? ?? '',
      sku: json['sku'] as String? ?? '',
      title: json['title'] as String? ?? '',
      coins: _asInt(json['coins']),
      amount: json['amount'] as String? ?? '',
      currency: json['currency'] as String? ?? '',
    );
  }

  static int _asInt(Object? v) {
    if (v is int) return v;
    if (v is num) return v.toInt();
    return 0;
  }
}

/// Pay order from create / get / sandbox-complete.
class PayOrder {
  PayOrder({
    required this.id,
    required this.status,
    required this.coins,
    required this.amount,
    required this.currency,
    required this.channel,
    this.payMode,
    this.payUrl,
    this.mockHint,
  });

  final String id;
  final String status;
  final int coins;
  final String amount;
  final String currency;
  final String channel;
  final String? payMode;
  final String? payUrl;
  final String? mockHint;

  bool get isCredited => status == 'credited' || status == 'paid';

  factory PayOrder.fromJson(Map<String, dynamic> json) {
    return PayOrder(
      id: json['id'] as String? ?? '',
      status: json['status'] as String? ?? '',
      coins: PayProduct._asInt(json['coins']),
      amount: json['amount'] as String? ?? '',
      currency: json['currency'] as String? ?? '',
      channel: json['channel'] as String? ?? '',
      payMode: json['pay_mode'] as String?,
      payUrl: json['pay_url'] as String?,
      mockHint: json['mock_hint'] as String?,
    );
  }
}

/// Pay control plane (coin packages + sandbox complete).
class PayRepository {
  PayRepository({
    required this.client,
    http.Client? httpClient,
  }) : httpClient = httpClient ?? http.Client();

  final ApiClient client;
  final http.Client httpClient;

  Future<List<PayProduct>> listProducts() async {
    final res = await httpClient.get(
      client.uri('/api/v1/pay/products'),
      headers: client.jsonHeaders(),
    );
    if (res.statusCode != 200) {
      throw PayException('products_failed', res.statusCode, res.body);
    }
    final map = jsonDecode(res.body) as Map<String, dynamic>;
    final items = map['items'] as List<dynamic>? ?? [];
    return items
        .map((e) => PayProduct.fromJson(e as Map<String, dynamic>))
        .toList();
  }

  Future<PayOrder> createOrder({
    required String productId,
    String channel = 'mock',
    String? clientRequestId,
  }) async {
    final body = <String, dynamic>{
      'product_id': productId,
      'channel': channel,
    };
    if (clientRequestId != null) body['client_request_id'] = clientRequestId;
    final res = await httpClient.post(
      client.uri('/api/v1/pay/orders'),
      headers: client.jsonHeaders(auth: true),
      body: jsonEncode(body),
    );
    if (res.statusCode != 200 && res.statusCode != 201) {
      throw PayException('create_order_failed', res.statusCode, res.body);
    }
    return PayOrder.fromJson(jsonDecode(res.body) as Map<String, dynamic>);
  }

  Future<PayOrder> getOrder(String orderId) async {
    final res = await httpClient.get(
      client.uri('/api/v1/pay/orders/$orderId'),
      headers: client.jsonHeaders(auth: true),
    );
    if (res.statusCode != 200) {
      throw PayException('get_order_failed', res.statusCode, res.body);
    }
    return PayOrder.fromJson(jsonDecode(res.body) as Map<String, dynamic>);
  }

  /// Dev/sandbox only: complete mock order and credit wallet.
  Future<PayOrder> sandboxComplete(String orderId) async {
    final res = await httpClient.post(
      client.uri('/api/v1/pay/orders/$orderId/sandbox-complete'),
      headers: client.jsonHeaders(auth: true),
    );
    if (res.statusCode != 200) {
      throw PayException('sandbox_complete_failed', res.statusCode, res.body);
    }
    return PayOrder.fromJson(jsonDecode(res.body) as Map<String, dynamic>);
  }
}

class PayException implements Exception {
  PayException(this.code, this.statusCode, this.body);
  final String code;
  final int statusCode;
  final String body;

  @override
  String toString() => 'PayException($code, $statusCode)';
}
