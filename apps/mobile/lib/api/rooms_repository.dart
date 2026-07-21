import 'dart:convert';

import 'package:http/http.dart' as http;

import 'api_client.dart';

class Room {
  Room({
    required this.id,
    required this.ownerId,
    required this.title,
    required this.status,
  });

  final String id;
  final String ownerId;
  final String title;
  final String status;

  factory Room.fromJson(Map<String, dynamic> json) {
    return Room(
      id: json['id'] as String? ?? '',
      ownerId: json['owner_id'] as String? ?? '',
      title: json['title'] as String? ?? '',
      status: json['status'] as String? ?? 'idle',
    );
  }

  bool get isLive => status == 'live';
}

class RoomsRepository {
  RoomsRepository({
    required this.client,
    http.Client? httpClient,
  }) : httpClient = httpClient ?? http.Client();

  final ApiClient client;
  final http.Client httpClient;

  Future<Room> createRoom(String title) async {
    final res = await httpClient.post(
      client.uri('/api/v1/rooms'),
      headers: client.jsonHeaders(auth: true),
      body: jsonEncode({'title': title}),
    );
    if (res.statusCode != 201) {
      throw RoomsException('create_failed', res.statusCode, res.body);
    }
    return Room.fromJson(jsonDecode(res.body) as Map<String, dynamic>);
  }

  Future<List<Room>> listRooms({String? status}) async {
    final path = status == null
        ? '/api/v1/rooms'
        : '/api/v1/rooms?status=${Uri.encodeQueryComponent(status)}';
    final res = await httpClient.get(
      client.uri(path),
      headers: client.jsonHeaders(),
    );
    if (res.statusCode != 200) {
      throw RoomsException('list_failed', res.statusCode, res.body);
    }
    final map = jsonDecode(res.body) as Map<String, dynamic>;
    final items = map['items'] as List<dynamic>? ?? [];
    return items
        .map((e) => Room.fromJson(e as Map<String, dynamic>))
        .toList();
  }

  Future<Room> startRoom(String roomId) async {
    final res = await httpClient.post(
      client.uri('/api/v1/rooms/$roomId/start'),
      headers: client.jsonHeaders(auth: true),
    );
    if (res.statusCode != 200) {
      throw RoomsException('start_failed', res.statusCode, res.body);
    }
    return Room.fromJson(jsonDecode(res.body) as Map<String, dynamic>);
  }

  Future<Map<String, dynamic>> playUrls(String roomId) async {
    final res = await httpClient.get(
      client.uri('/api/v1/rooms/$roomId/media/play'),
      headers: client.jsonHeaders(),
    );
    if (res.statusCode != 200) {
      throw RoomsException('play_failed', res.statusCode, res.body);
    }
    return jsonDecode(res.body) as Map<String, dynamic>;
  }
}

class RoomsException implements Exception {
  RoomsException(this.code, this.statusCode, this.body);
  final String code;
  final int statusCode;
  final String body;

  @override
  String toString() => 'RoomsException($code, $statusCode)';
}
