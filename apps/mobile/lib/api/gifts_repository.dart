import 'dart:convert';

import 'package:http/http.dart' as http;

import 'api_client.dart';

class GiftItem {
  GiftItem({required this.id, required this.name, required this.price});

  final String id;
  final String name;
  final int price;

  factory GiftItem.fromJson(Map<String, dynamic> json) {
    return GiftItem(
      id: json['id'] as String? ?? '',
      name: json['name'] as String? ?? '',
      price: json['price'] as int? ?? 0,
    );
  }
}

class GiftOrder {
  GiftOrder({
    required this.id,
    required this.totalCoins,
    required this.replayed,
  });

  final String id;
  final int totalCoins;
  final bool replayed;

  factory GiftOrder.fromJson(Map<String, dynamic> json) {
    return GiftOrder(
      id: json['id'] as String? ?? '',
      totalCoins: json['total_coins'] as int? ?? 0,
      replayed: json['replayed'] as bool? ?? false,
    );
  }
}

class GiftsRepository {
  GiftsRepository({
    required this.client,
    http.Client? httpClient,
  }) : httpClient = httpClient ?? http.Client();

  final ApiClient client;
  final http.Client httpClient;

  Future<List<GiftItem>> listGifts() async {
    final res = await httpClient.get(
      client.uri('/api/v1/gifts'),
      headers: client.jsonHeaders(),
    );
    if (res.statusCode != 200) {
      throw GiftsException('list_failed', res.statusCode, res.body);
    }
    final map = jsonDecode(res.body) as Map<String, dynamic>;
    final items = map['items'] as List<dynamic>? ?? [];
    return items
        .map((e) => GiftItem.fromJson(e as Map<String, dynamic>))
        .toList();
  }

  Future<int> walletBalance() async {
    final res = await httpClient.get(
      client.uri('/api/v1/wallet'),
      headers: client.jsonHeaders(auth: true),
    );
    if (res.statusCode != 200) {
      throw GiftsException('wallet_failed', res.statusCode, res.body);
    }
    final map = jsonDecode(res.body) as Map<String, dynamic>;
    return map['balance'] as int? ?? 0;
  }

  Future<int> topup(int amount) async {
    final res = await httpClient.post(
      client.uri('/api/v1/wallet/topups'),
      headers: client.jsonHeaders(auth: true),
      body: jsonEncode({'amount': amount}),
    );
    if (res.statusCode != 200) {
      throw GiftsException('topup_failed', res.statusCode, res.body);
    }
    final map = jsonDecode(res.body) as Map<String, dynamic>;
    return map['balance'] as int? ?? 0;
  }

  Future<GiftOrder> sendGift({
    required String roomId,
    required String giftId,
    required String receiverId,
    required String clientRequestId,
    int count = 1,
  }) async {
    final res = await httpClient.post(
      client.uri('/api/v1/rooms/$roomId/gifts'),
      headers: client.jsonHeaders(auth: true),
      body: jsonEncode({
        'gift_id': giftId,
        'receiver_id': receiverId,
        'count': count,
        'client_request_id': clientRequestId,
      }),
    );
    if (res.statusCode != 200 && res.statusCode != 201) {
      throw GiftsException('send_failed', res.statusCode, res.body);
    }
    return GiftOrder.fromJson(jsonDecode(res.body) as Map<String, dynamic>);
  }
}

class GiftsException implements Exception {
  GiftsException(this.code, this.statusCode, this.body);
  final String code;
  final int statusCode;
  final String body;

  @override
  String toString() => 'GiftsException($code, $statusCode)';
}
