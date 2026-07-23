import 'dart:convert';

import 'package:http/http.dart' as http;

import 'api_client.dart';

/// Device push token registration (WBS E8.9 scaffold — no FCM delivery).
class PushDevice {
  PushDevice({
    required this.id,
    required this.platform,
    required this.token,
    required this.createdAt,
    required this.updatedAt,
  });

  final String id;
  final String platform;
  final String token;
  final String createdAt;
  final String updatedAt;

  factory PushDevice.fromJson(Map<String, dynamic> json) {
    return PushDevice(
      id: json['id'] as String? ?? '',
      platform: json['platform'] as String? ?? '',
      token: json['token'] as String? ?? '',
      createdAt: json['created_at'] as String? ?? '',
      updatedAt: json['updated_at'] as String? ?? '',
    );
  }
}

class PushRepository {
  PushRepository({
    required this.client,
    http.Client? httpClient,
  }) : httpClient = httpClient ?? http.Client();

  final ApiClient client;
  final http.Client httpClient;

  Future<PushDevice> register({
    required String token,
    required String platform,
  }) async {
    final res = await httpClient.post(
      client.uri('/api/v1/me/push-tokens'),
      headers: client.jsonHeaders(auth: true),
      body: jsonEncode({'token': token, 'platform': platform}),
    );
    if (res.statusCode != 200) {
      throw PushException('register_failed', res.statusCode, res.body);
    }
    return PushDevice.fromJson(jsonDecode(res.body) as Map<String, dynamic>);
  }

  Future<List<PushDevice>> list() async {
    final res = await httpClient.get(
      client.uri('/api/v1/me/push-tokens'),
      headers: client.jsonHeaders(auth: true),
    );
    if (res.statusCode != 200) {
      throw PushException('list_failed', res.statusCode, res.body);
    }
    final map = jsonDecode(res.body) as Map<String, dynamic>;
    final items = map['items'] as List<dynamic>? ?? [];
    return items
        .map((e) => PushDevice.fromJson(e as Map<String, dynamic>))
        .toList();
  }

  Future<void> unregister({
    required String token,
    String platform = 'other',
  }) async {
    final res = await httpClient.delete(
      client.uri('/api/v1/me/push-tokens'),
      headers: client.jsonHeaders(auth: true),
      body: jsonEncode({'token': token, 'platform': platform}),
    );
    if (res.statusCode != 204 && res.statusCode != 200) {
      throw PushException('unregister_failed', res.statusCode, res.body);
    }
  }
}

class PushException implements Exception {
  PushException(this.code, this.statusCode, this.body);
  final String code;
  final int statusCode;
  final String body;

  @override
  String toString() => 'PushException($code, $statusCode)';
}
