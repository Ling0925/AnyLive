import 'dart:convert';

import 'package:http/http.dart' as http;

import 'api_client.dart';
import 'rooms_repository.dart';

/// User profile returned by GET/PATCH `/api/v1/me`.
class UserProfile {
  UserProfile({
    required this.id,
    required this.displayName,
    required this.email,
    required this.createdAt,
    required this.ageConfirmed,
    required this.privacyAccepted,
    this.avatarUrl,
    this.region,
  });

  final String id;
  final String displayName;
  final String? email;
  final String createdAt;
  final bool ageConfirmed;
  final bool privacyAccepted;
  final String? avatarUrl;
  final String? region;

  factory UserProfile.fromJson(Map<String, dynamic> json) {
    return UserProfile(
      id: json['id'] as String? ?? '',
      displayName: json['display_name'] as String? ?? '',
      email: json['email'] as String?,
      createdAt: json['created_at'] as String? ?? '',
      ageConfirmed: json['age_confirmed'] as bool? ?? false,
      privacyAccepted: json['privacy_accepted'] as bool? ?? false,
      avatarUrl: json['avatar_url'] as String?,
      region: json['region'] as String?,
    );
  }
}

/// Response from `POST /api/v1/me/avatar/presign`.
class AvatarPresign {
  AvatarPresign({
    required this.objectKey,
    required this.uploadUrl,
    required this.publicUrl,
    required this.method,
    required this.expiresIn,
  });

  final String objectKey;
  final String uploadUrl;
  final String publicUrl;
  final String method;
  final int expiresIn;

  factory AvatarPresign.fromJson(Map<String, dynamic> json) {
    return AvatarPresign(
      objectKey: json['object_key'] as String? ?? '',
      uploadUrl: json['upload_url'] as String? ?? '',
      publicUrl: json['public_url'] as String? ?? '',
      method: json['method'] as String? ?? 'PUT',
      expiresIn: (json['expires_in'] as num?)?.toInt() ?? 0,
    );
  }
}

/// Host dashboard from GET `/api/v1/me/creator` (P4 creator center).
class CreatorStats {
  CreatorStats({
    required this.followerCount,
    required this.followingCount,
    required this.liveRooms,
    required this.totalRooms,
    required this.giftCoinsReceived,
    required this.giftCreditEntries,
    required this.rooms,
  });

  final int followerCount;
  final int followingCount;
  final int liveRooms;
  final int totalRooms;
  final int giftCoinsReceived;
  final int giftCreditEntries;
  final List<Room> rooms;

  factory CreatorStats.fromJson(Map<String, dynamic> json) {
    final roomsRaw = json['rooms'];
    final rooms = <Room>[];
    if (roomsRaw is List) {
      for (final item in roomsRaw) {
        if (item is Map<String, dynamic>) {
          rooms.add(Room.fromJson(item));
        } else if (item is Map) {
          rooms.add(Room.fromJson(Map<String, dynamic>.from(item)));
        }
      }
    }
    return CreatorStats(
      followerCount: _asInt(json['follower_count']),
      followingCount: _asInt(json['following_count']),
      liveRooms: _asInt(json['live_rooms']),
      totalRooms: _asInt(json['total_rooms']),
      giftCoinsReceived: _asInt(json['gift_coins_received']),
      giftCreditEntries: _asInt(json['gift_credit_entries']),
      rooms: rooms,
    );
  }

  static int _asInt(Object? v) {
    if (v is int) return v;
    if (v is num) return v.toInt();
    if (v is String) return int.tryParse(v) ?? 0;
    return 0;
  }
}

/// Profile API calls (GET/PATCH `/api/v1/me`, GET `/me/creator`).
/// [httpClient] injectable for tests.
class ProfileRepository {
  ProfileRepository({
    required this.client,
    http.Client? httpClient,
  }) : httpClient = httpClient ?? http.Client();

  final ApiClient client;
  final http.Client httpClient;

  /// GET `/api/v1/me`
  Future<UserProfile> getMe() async {
    final res = await httpClient.get(
      client.uri('/api/v1/me'),
      headers: client.jsonHeaders(auth: true),
    );
    if (res.statusCode != 200) {
      throw ProfileException('get_me_failed', res.statusCode, res.body);
    }
    return UserProfile.fromJson(jsonDecode(res.body) as Map<String, dynamic>);
  }

  /// GET `/api/v1/me/creator` — host dashboard stats.
  Future<CreatorStats> getCreatorStats() async {
    final res = await httpClient.get(
      client.uri('/api/v1/me/creator'),
      headers: client.jsonHeaders(auth: true),
    );
    if (res.statusCode != 200) {
      throw ProfileException('creator_stats_failed', res.statusCode, res.body);
    }
    return CreatorStats.fromJson(
      jsonDecode(res.body) as Map<String, dynamic>,
    );
  }

  /// PATCH `/api/v1/me` — at least one field required by the API.
  Future<UserProfile> patchMe({
    String? displayName,
    bool? ageConfirmed,
    bool? privacyAccepted,
    String? region,
  }) async {
    final body = <String, dynamic>{};
    if (displayName != null) body['display_name'] = displayName;
    if (ageConfirmed != null) body['age_confirmed'] = ageConfirmed;
    if (privacyAccepted != null) body['privacy_accepted'] = privacyAccepted;
    if (region != null) body['region'] = region;

    final res = await httpClient.patch(
      client.uri('/api/v1/me'),
      headers: client.jsonHeaders(auth: true),
      body: jsonEncode(body),
    );
    if (res.statusCode != 200) {
      throw ProfileException('patch_me_failed', res.statusCode, res.body);
    }
    return UserProfile.fromJson(jsonDecode(res.body) as Map<String, dynamic>);
  }

  /// POST `/api/v1/me/avatar/presign`
  Future<AvatarPresign> presignAvatar({String contentType = 'image/jpeg'}) async {
    final res = await httpClient.post(
      client.uri('/api/v1/me/avatar/presign'),
      headers: client.jsonHeaders(auth: true),
      body: jsonEncode({'content_type': contentType}),
    );
    if (res.statusCode != 200) {
      throw ProfileException('avatar_presign_failed', res.statusCode, res.body);
    }
    return AvatarPresign.fromJson(jsonDecode(res.body) as Map<String, dynamic>);
  }

  /// POST `/api/v1/me/avatar/confirm` after client PUT (or dogfood skip-upload).
  Future<UserProfile> confirmAvatar({
    required String objectKey,
    String? publicUrl,
  }) async {
    final body = <String, dynamic>{'object_key': objectKey};
    if (publicUrl != null) body['public_url'] = publicUrl;
    final res = await httpClient.post(
      client.uri('/api/v1/me/avatar/confirm'),
      headers: client.jsonHeaders(auth: true),
      body: jsonEncode(body),
    );
    if (res.statusCode != 200) {
      throw ProfileException('avatar_confirm_failed', res.statusCode, res.body);
    }
    return UserProfile.fromJson(jsonDecode(res.body) as Map<String, dynamic>);
  }
}

class ProfileException implements Exception {
  ProfileException(this.code, this.statusCode, this.body);
  final String code;
  final int statusCode;
  final String body;

  @override
  String toString() => 'ProfileException($code, $statusCode)';
}
