import 'dart:convert';

import 'package:http/http.dart' as http;

import 'api_client.dart';
import 'rooms_repository.dart';

/// Response from `POST /api/v1/realtime/token`.
class RealtimeToken {
  RealtimeToken({
    required this.token,
    required this.expiresIn,
    required this.channels,
  });

  final String token;
  final int expiresIn;
  final List<String> channels;

  factory RealtimeToken.fromJson(Map<String, dynamic> json) {
    final ch = json['channels'];
    return RealtimeToken(
      token: json['token'] as String? ?? '',
      expiresIn: json['expires_in'] as int? ?? 0,
      channels: ch is List
          ? ch.map((e) => e.toString()).toList()
          : const <String>[],
    );
  }
}

/// Realtime token + chat helpers used by room pages.
class RealtimeRepository {
  RealtimeRepository({
    required this.client,
    http.Client? httpClient,
  }) : httpClient = httpClient ?? http.Client();

  final ApiClient client;
  final http.Client httpClient;

  Future<RealtimeToken> connectionToken(String roomId) async {
    final res = await httpClient.post(
      client.uri('/api/v1/realtime/token'),
      headers: client.jsonHeaders(auth: true),
      body: jsonEncode({'room_id': roomId}),
    );
    if (res.statusCode != 200) {
      throw RealtimeException('token_failed', res.statusCode, res.body);
    }
    return RealtimeToken.fromJson(
      jsonDecode(res.body) as Map<String, dynamic>,
    );
  }

  /// Poll history (fallback when Centrifugo WS is not configured).
  Future<List<ChatMessage>> listMessages(String roomId, {int limit = 50}) async {
    final res = await httpClient.get(
      client.uri('/api/v1/rooms/$roomId/messages?limit=$limit'),
      headers: client.jsonHeaders(),
    );
    if (res.statusCode != 200) {
      throw RealtimeException('messages_list_failed', res.statusCode, res.body);
    }
    final map = jsonDecode(res.body) as Map<String, dynamic>;
    final items = map['items'] as List<dynamic>? ?? [];
    return items
        .map((e) => ChatMessage.fromJson(e as Map<String, dynamic>))
        .toList();
  }
}

class RealtimeException implements Exception {
  RealtimeException(this.code, this.statusCode, this.body);
  final String code;
  final int statusCode;
  final String body;

  @override
  String toString() => 'RealtimeException($code, $statusCode)';
}
