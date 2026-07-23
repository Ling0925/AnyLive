import 'dart:convert';

import 'package:http/http.dart' as http;

import 'api_client.dart';

/// Co-host invite session from interactive APIs.
class InteractiveSession {
  InteractiveSession({
    required this.id,
    required this.roomId,
    required this.hostId,
    required this.inviteeId,
    required this.status,
    required this.createdAt,
    required this.updatedAt,
    this.endedAt,
  });

  final String id;
  final String roomId;
  final String hostId;
  final String inviteeId;
  final String status;
  final String createdAt;
  final String updatedAt;
  final String? endedAt;

  bool get isActive => status == 'active';
  bool get isInvited => status == 'invited';

  factory InteractiveSession.fromJson(Map<String, dynamic> json) {
    return InteractiveSession(
      id: json['id'] as String? ?? '',
      roomId: json['room_id'] as String? ?? '',
      hostId: json['host_id'] as String? ?? '',
      inviteeId: json['invitee_id'] as String? ?? '',
      status: json['status'] as String? ?? '',
      createdAt: json['created_at'] as String? ?? '',
      updatedAt: json['updated_at'] as String? ?? '',
      endedAt: json['ended_at'] as String?,
    );
  }
}

/// PK battle session.
class PkSession {
  PkSession({
    required this.id,
    required this.roomAId,
    required this.roomBId,
    required this.hostAId,
    required this.hostBId,
    required this.status,
    required this.scoreA,
    required this.scoreB,
    required this.startedAt,
    required this.endsAt,
    required this.updatedAt,
    this.winnerRoomId,
    this.endedAt,
  });

  final String id;
  final String roomAId;
  final String roomBId;
  final String hostAId;
  final String hostBId;
  final String status;
  final int scoreA;
  final int scoreB;
  final String startedAt;
  final String endsAt;
  final String updatedAt;
  final String? winnerRoomId;
  final String? endedAt;

  bool get isActive => status == 'active';
  bool get isEnded => status == 'ended';

  factory PkSession.fromJson(Map<String, dynamic> json) {
    return PkSession(
      id: json['id'] as String? ?? '',
      roomAId: json['room_a_id'] as String? ?? '',
      roomBId: json['room_b_id'] as String? ?? '',
      hostAId: json['host_a_id'] as String? ?? '',
      hostBId: json['host_b_id'] as String? ?? '',
      status: json['status'] as String? ?? '',
      scoreA: _asInt(json['score_a']),
      scoreB: _asInt(json['score_b']),
      startedAt: json['started_at'] as String? ?? '',
      endsAt: json['ends_at'] as String? ?? '',
      updatedAt: json['updated_at'] as String? ?? '',
      winnerRoomId: json['winner_room_id'] as String?,
      endedAt: json['ended_at'] as String?,
    );
  }

  static int _asInt(Object? v) {
    if (v is int) return v;
    if (v is num) return v.toInt();
    return 0;
  }
}

/// LiveKit join credentials from POST `/rooms/{id}/livekit/join`.
class LivekitJoinInfo {
  LivekitJoinInfo({
    required this.url,
    required this.token,
    required this.roomName,
    this.identity,
    this.expiresAt,
  });

  final String url;
  final String token;
  final String roomName;
  final String? identity;
  final String? expiresAt;

  factory LivekitJoinInfo.fromJson(Map<String, dynamic> json) {
    return LivekitJoinInfo(
      url: json['url'] as String? ?? json['ws_url'] as String? ?? '',
      token: json['token'] as String? ?? '',
      roomName: json['room_name'] as String? ?? '',
      identity: json['identity'] as String?,
      expiresAt: json['expires_at'] as String?,
    );
  }
}

/// Co-host / PK / LiveKit control plane (P3).
class InteractiveRepository {
  InteractiveRepository({
    required this.client,
    http.Client? httpClient,
  }) : httpClient = httpClient ?? http.Client();

  final ApiClient client;
  final http.Client httpClient;

  Future<InteractiveSession> invite({
    required String roomId,
    required String inviteeId,
  }) async {
    final res = await httpClient.post(
      client.uri('/api/v1/rooms/$roomId/interactive/invite'),
      headers: client.jsonHeaders(auth: true),
      body: jsonEncode({'invitee_id': inviteeId}),
    );
    if (res.statusCode != 200 && res.statusCode != 201) {
      throw InteractiveException('invite_failed', res.statusCode, res.body);
    }
    return InteractiveSession.fromJson(
      jsonDecode(res.body) as Map<String, dynamic>,
    );
  }

  Future<InteractiveSession> respond({
    required String roomId,
    required bool accept,
  }) async {
    final res = await httpClient.post(
      client.uri('/api/v1/rooms/$roomId/interactive/respond'),
      headers: client.jsonHeaders(auth: true),
      body: jsonEncode({'accept': accept}),
    );
    if (res.statusCode != 200) {
      throw InteractiveException('respond_failed', res.statusCode, res.body);
    }
    return InteractiveSession.fromJson(
      jsonDecode(res.body) as Map<String, dynamic>,
    );
  }

  Future<InteractiveSession> leave(String roomId) async {
    final res = await httpClient.post(
      client.uri('/api/v1/rooms/$roomId/interactive/leave'),
      headers: client.jsonHeaders(auth: true),
    );
    if (res.statusCode != 200 && res.statusCode != 204) {
      throw InteractiveException('leave_failed', res.statusCode, res.body);
    }
    if (res.body.isEmpty) {
      return InteractiveSession(
        id: '',
        roomId: roomId,
        hostId: '',
        inviteeId: '',
        status: 'ended',
        createdAt: '',
        updatedAt: '',
      );
    }
    return InteractiveSession.fromJson(
      jsonDecode(res.body) as Map<String, dynamic>,
    );
  }

  Future<List<InteractiveSession>> list(String roomId) async {
    final res = await httpClient.get(
      client.uri('/api/v1/rooms/$roomId/interactive'),
      headers: client.jsonHeaders(auth: true),
    );
    if (res.statusCode != 200) {
      throw InteractiveException('list_failed', res.statusCode, res.body);
    }
    final body = jsonDecode(res.body) as Map<String, dynamic>;
    final items = body['items'] as List<dynamic>? ?? [];
    return items
        .map((e) => InteractiveSession.fromJson(e as Map<String, dynamic>))
        .toList();
  }

  Future<PkSession?> getPk(String roomId) async {
    final res = await httpClient.get(
      client.uri('/api/v1/rooms/$roomId/pk'),
      headers: client.jsonHeaders(auth: true),
    );
    // Soft-tolerate feature-off / missing session (P3 default off).
    if (res.statusCode == 403 || res.statusCode == 404) {
      return null;
    }
    if (res.statusCode != 200) {
      throw InteractiveException('get_pk_failed', res.statusCode, res.body);
    }
    final body = jsonDecode(res.body) as Map<String, dynamic>;
    final session = body['session'];
    if (session == null) return null;
    return PkSession.fromJson(session as Map<String, dynamic>);
  }

  Future<PkSession> startPk({
    required String roomId,
    required String opponentRoomId,
    int? durationSecs,
  }) async {
    final payload = <String, dynamic>{'opponent_room_id': opponentRoomId};
    if (durationSecs != null) payload['duration_secs'] = durationSecs;
    final res = await httpClient.post(
      client.uri('/api/v1/rooms/$roomId/pk/start'),
      headers: client.jsonHeaders(auth: true),
      body: jsonEncode(payload),
    );
    if (res.statusCode != 200 && res.statusCode != 201) {
      throw InteractiveException('start_pk_failed', res.statusCode, res.body);
    }
    final body = jsonDecode(res.body) as Map<String, dynamic>;
    if (body['session'] is Map<String, dynamic>) {
      return PkSession.fromJson(body['session'] as Map<String, dynamic>);
    }
    return PkSession.fromJson(body);
  }

  Future<PkSession> endPk(String roomId) async {
    final res = await httpClient.post(
      client.uri('/api/v1/rooms/$roomId/pk/end'),
      headers: client.jsonHeaders(auth: true),
    );
    if (res.statusCode != 200) {
      throw InteractiveException('end_pk_failed', res.statusCode, res.body);
    }
    final body = jsonDecode(res.body) as Map<String, dynamic>;
    if (body['session'] is Map<String, dynamic>) {
      return PkSession.fromJson(body['session'] as Map<String, dynamic>);
    }
    return PkSession.fromJson(body);
  }

  /// [role] is `viewer` | `host` | `cohost` (API default viewer).
  Future<LivekitJoinInfo> livekitJoin(
    String roomId, {
    String role = 'viewer',
  }) async {
    final res = await httpClient.post(
      client.uri('/api/v1/rooms/$roomId/livekit/join'),
      headers: client.jsonHeaders(auth: true),
      body: jsonEncode({'role': role}),
    );
    if (res.statusCode != 200) {
      throw InteractiveException(
        'livekit_join_failed',
        res.statusCode,
        res.body,
      );
    }
    return LivekitJoinInfo.fromJson(
      jsonDecode(res.body) as Map<String, dynamic>,
    );
  }
}

class InteractiveException implements Exception {
  InteractiveException(this.code, this.statusCode, this.body);
  final String code;
  final int statusCode;
  final String body;

  @override
  String toString() => 'InteractiveException($code, $statusCode)';
}
