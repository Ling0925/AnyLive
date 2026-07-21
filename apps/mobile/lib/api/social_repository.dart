import 'dart:convert';

import 'package:http/http.dart' as http;

import 'api_client.dart';
import 'rooms_repository.dart';

/// Social / feed API calls. [httpClient] injectable for tests.
class SocialRepository {
  SocialRepository({
    required this.client,
    http.Client? httpClient,
  }) : httpClient = httpClient ?? http.Client();

  final ApiClient client;
  final http.Client httpClient;

  /// POST `/api/v1/users/{id}/follow` — expects 204.
  Future<void> follow(String userId) async {
    final res = await httpClient.post(
      client.uri('/api/v1/users/$userId/follow'),
      headers: client.jsonHeaders(auth: true),
    );
    if (res.statusCode != 204) {
      throw SocialException('follow_failed', res.statusCode, res.body);
    }
  }

  /// DELETE `/api/v1/users/{id}/follow` — expects 204.
  Future<void> unfollow(String userId) async {
    final res = await httpClient.delete(
      client.uri('/api/v1/users/$userId/follow'),
      headers: client.jsonHeaders(auth: true),
    );
    if (res.statusCode != 204) {
      throw SocialException('unfollow_failed', res.statusCode, res.body);
    }
  }

  /// GET `/api/v1/me/following` — returns followed user ids.
  Future<List<String>> listFollowing() async {
    final res = await httpClient.get(
      client.uri('/api/v1/me/following'),
      headers: client.jsonHeaders(auth: true),
    );
    if (res.statusCode != 200) {
      throw SocialException('list_following_failed', res.statusCode, res.body);
    }
    final map = jsonDecode(res.body) as Map<String, dynamic>;
    final ids = map['user_ids'] as List<dynamic>? ?? [];
    return ids.map((e) => e.toString()).toList();
  }

  /// GET `/api/v1/feed/hot` — currently live rooms.
  Future<List<Room>> feedHot() async {
    final res = await httpClient.get(
      client.uri('/api/v1/feed/hot'),
      headers: client.jsonHeaders(),
    );
    if (res.statusCode != 200) {
      throw SocialException('feed_hot_failed', res.statusCode, res.body);
    }
    return _parseRooms(res.body);
  }

  /// GET `/api/v1/feed/following` — live rooms from followed hosts.
  Future<List<Room>> feedFollowing() async {
    final res = await httpClient.get(
      client.uri('/api/v1/feed/following'),
      headers: client.jsonHeaders(auth: true),
    );
    if (res.statusCode != 200) {
      throw SocialException('feed_following_failed', res.statusCode, res.body);
    }
    return _parseRooms(res.body);
  }

  List<Room> _parseRooms(String body) {
    final map = jsonDecode(body) as Map<String, dynamic>;
    final items = map['items'] as List<dynamic>? ?? [];
    return items
        .map((e) => Room.fromJson(e as Map<String, dynamic>))
        .toList();
  }
}

class SocialException implements Exception {
  SocialException(this.code, this.statusCode, this.body);
  final String code;
  final int statusCode;
  final String body;

  @override
  String toString() => 'SocialException($code, $statusCode)';
}
