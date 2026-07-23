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

class RoomStats {
  RoomStats({
    required this.roomId,
    required this.onlineCount,
    required this.likeCount,
    this.recordingEnabled = false,
  });

  final String roomId;
  final int onlineCount;
  final int likeCount;
  final bool recordingEnabled;

  factory RoomStats.fromJson(Map<String, dynamic> json) {
    return RoomStats(
      roomId: json['room_id'] as String? ?? '',
      onlineCount: (json['online_count'] as num?)?.toInt() ?? 0,
      likeCount: (json['like_count'] as num?)?.toInt() ?? 0,
      recordingEnabled: json['recording_enabled'] as bool? ?? false,
    );
  }
}

class RecordingStatus {
  RecordingStatus({required this.roomId, required this.recordingEnabled});

  final String roomId;
  final bool recordingEnabled;

  factory RecordingStatus.fromJson(Map<String, dynamic> json) {
    return RecordingStatus(
      roomId: json['room_id'] as String? ?? '',
      recordingEnabled: json['recording_enabled'] as bool? ?? false,
    );
  }
}

class LikeResult {
  LikeResult({required this.accepted, required this.likeCount});

  final bool accepted;
  final int likeCount;

  factory LikeResult.fromJson(Map<String, dynamic> json) {
    return LikeResult(
      accepted: json['accepted'] as bool? ?? false,
      likeCount: (json['like_count'] as num?)?.toInt() ?? 0,
    );
  }
}

class SearchUserHit {
  SearchUserHit({required this.id, required this.displayName});

  final String id;
  final String displayName;

  factory SearchUserHit.fromJson(Map<String, dynamic> json) {
    return SearchUserHit(
      id: json['id'] as String? ?? '',
      displayName: json['display_name'] as String? ?? '',
    );
  }
}

class SearchResult {
  SearchResult({required this.users, required this.rooms});

  final List<SearchUserHit> users;
  final List<Room> rooms;

  factory SearchResult.fromJson(Map<String, dynamic> json) {
    final users = (json['users'] as List<dynamic>? ?? [])
        .map((e) => SearchUserHit.fromJson(e as Map<String, dynamic>))
        .toList();
    final rooms = (json['rooms'] as List<dynamic>? ?? [])
        .map((e) => Room.fromJson(e as Map<String, dynamic>))
        .toList();
    return SearchResult(users: users, rooms: rooms);
  }
}

/// Owner publish credentials from `POST /api/v1/rooms/{id}/media/publish`.
class PublishInfo {
  PublishInfo({
    required this.pushUrl,
    required this.streamKey,
    this.expiresAt,
  });

  final String pushUrl;
  final String streamKey;
  final String? expiresAt;

  factory PublishInfo.fromJson(Map<String, dynamic> json) {
    return PublishInfo(
      pushUrl: json['push_url'] as String? ?? '',
      streamKey: json['stream_key'] as String? ?? '',
      expiresAt: json['expires_at'] as String?,
    );
  }
}

class ChatMessage {
  ChatMessage({
    required this.id,
    required this.roomId,
    required this.senderId,
    required this.senderName,
    required this.body,
    required this.createdAt,
  });

  final String id;
  final String roomId;
  final String senderId;
  final String senderName;
  final String body;
  final String createdAt;

  factory ChatMessage.fromJson(Map<String, dynamic> json) {
    return ChatMessage(
      id: json['id'] as String? ?? '',
      roomId: json['room_id'] as String? ?? '',
      senderId: json['sender_id'] as String? ?? '',
      senderName: json['sender_name'] as String? ?? '',
      body: json['body'] as String? ?? '',
      createdAt: json['created_at'] as String? ?? '',
    );
  }
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

  Future<Room> getRoom(String roomId) async {
    final res = await httpClient.get(
      client.uri('/api/v1/rooms/$roomId'),
      headers: client.jsonHeaders(),
    );
    if (res.statusCode != 200) {
      throw RoomsException('get_failed', res.statusCode, res.body);
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

  Future<Room> stopRoom(String roomId) async {
    final res = await httpClient.post(
      client.uri('/api/v1/rooms/$roomId/stop'),
      headers: client.jsonHeaders(auth: true),
    );
    if (res.statusCode != 200) {
      throw RoomsException('stop_failed', res.statusCode, res.body);
    }
    return Room.fromJson(jsonDecode(res.body) as Map<String, dynamic>);
  }

  /// Owner-only OBS/RTMP credentials from `POST .../media/publish`.
  Future<PublishInfo> publishInfo(String roomId) async {
    final res = await httpClient.post(
      client.uri('/api/v1/rooms/$roomId/media/publish'),
      headers: client.jsonHeaders(auth: true),
    );
    if (res.statusCode != 200) {
      throw RoomsException('publish_failed', res.statusCode, res.body);
    }
    return PublishInfo.fromJson(jsonDecode(res.body) as Map<String, dynamic>);
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

  /// Public stats: online_count + like_count.
  Future<RoomStats> roomStats(String roomId) async {
    final res = await httpClient.get(
      client.uri('/api/v1/rooms/$roomId/stats'),
      headers: client.jsonHeaders(),
    );
    if (res.statusCode != 200) {
      throw RoomsException('stats_failed', res.statusCode, res.body);
    }
    return RoomStats.fromJson(jsonDecode(res.body) as Map<String, dynamic>);
  }

  /// Presence heartbeat; returns online count.
  Future<int> presenceHeartbeat(String roomId) async {
    final res = await httpClient.post(
      client.uri('/api/v1/rooms/$roomId/presence'),
      headers: client.jsonHeaders(auth: true),
    );
    if (res.statusCode != 200) {
      throw RoomsException('presence_failed', res.statusCode, res.body);
    }
    final map = jsonDecode(res.body) as Map<String, dynamic>;
    return (map['online_count'] as num?)?.toInt() ?? 0;
  }

  /// Like the room; returns accepted + total.
  Future<LikeResult> likeRoom(String roomId) async {
    final res = await httpClient.post(
      client.uri('/api/v1/rooms/$roomId/likes'),
      headers: client.jsonHeaders(auth: true),
      body: jsonEncode({}),
    );
    if (res.statusCode != 200) {
      throw RoomsException('like_failed', res.statusCode, res.body);
    }
    return LikeResult.fromJson(jsonDecode(res.body) as Map<String, dynamic>);
  }

  /// GET `/api/v1/rooms/{id}/recording`
  Future<RecordingStatus> getRecording(String roomId) async {
    final res = await httpClient.get(
      client.uri('/api/v1/rooms/$roomId/recording'),
      headers: client.jsonHeaders(),
    );
    if (res.statusCode != 200) {
      throw RoomsException('recording_get_failed', res.statusCode, res.body);
    }
    return RecordingStatus.fromJson(
      jsonDecode(res.body) as Map<String, dynamic>,
    );
  }

  /// PUT `/api/v1/rooms/{id}/recording` — owner only.
  Future<RecordingStatus> setRecording(String roomId, bool enabled) async {
    final res = await httpClient.put(
      client.uri('/api/v1/rooms/$roomId/recording'),
      headers: client.jsonHeaders(auth: true),
      body: jsonEncode({'enabled': enabled}),
    );
    if (res.statusCode != 200) {
      throw RoomsException('recording_set_failed', res.statusCode, res.body);
    }
    return RecordingStatus.fromJson(
      jsonDecode(res.body) as Map<String, dynamic>,
    );
  }

  /// Search users/rooms by substring query.
  Future<SearchResult> search(String q, {String type = 'all', int limit = 20}) async {
    final path =
        '/api/v1/search?q=${Uri.encodeQueryComponent(q)}&type=${Uri.encodeQueryComponent(type)}&limit=$limit';
    final res = await httpClient.get(
      client.uri(path),
      headers: client.jsonHeaders(),
    );
    if (res.statusCode != 200) {
      throw RoomsException('search_failed', res.statusCode, res.body);
    }
    return SearchResult.fromJson(jsonDecode(res.body) as Map<String, dynamic>);
  }

  Future<List<ChatMessage>> listMessages(String roomId, {int limit = 50}) async {
    final res = await httpClient.get(
      client.uri('/api/v1/rooms/$roomId/messages?limit=$limit'),
      headers: client.jsonHeaders(),
    );
    if (res.statusCode != 200) {
      throw RoomsException('messages_list_failed', res.statusCode, res.body);
    }
    final map = jsonDecode(res.body) as Map<String, dynamic>;
    final items = map['items'] as List<dynamic>? ?? [];
    return items
        .map((e) => ChatMessage.fromJson(e as Map<String, dynamic>))
        .toList();
  }

  Future<ChatMessage> postMessage(String roomId, String body) async {
    final res = await httpClient.post(
      client.uri('/api/v1/rooms/$roomId/messages'),
      headers: client.jsonHeaders(auth: true),
      body: jsonEncode({'body': body}),
    );
    if (res.statusCode != 201) {
      throw RoomsException('messages_post_failed', res.statusCode, res.body);
    }
    return ChatMessage.fromJson(jsonDecode(res.body) as Map<String, dynamic>);
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
