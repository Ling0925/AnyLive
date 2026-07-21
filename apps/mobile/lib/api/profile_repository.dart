import 'dart:convert';

import 'package:http/http.dart' as http;

import 'api_client.dart';

/// User profile returned by GET/PATCH `/api/v1/me`.
class UserProfile {
  UserProfile({
    required this.id,
    required this.displayName,
    required this.email,
    required this.createdAt,
    required this.ageConfirmed,
    required this.privacyAccepted,
  });

  final String id;
  final String displayName;
  final String? email;
  final String createdAt;
  final bool ageConfirmed;
  final bool privacyAccepted;

  factory UserProfile.fromJson(Map<String, dynamic> json) {
    return UserProfile(
      id: json['id'] as String? ?? '',
      displayName: json['display_name'] as String? ?? '',
      email: json['email'] as String?,
      createdAt: json['created_at'] as String? ?? '',
      ageConfirmed: json['age_confirmed'] as bool? ?? false,
      privacyAccepted: json['privacy_accepted'] as bool? ?? false,
    );
  }
}

/// Profile API calls (GET/PATCH `/api/v1/me`). [httpClient] injectable for tests.
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

  /// PATCH `/api/v1/me` — at least one field required by the API.
  Future<UserProfile> patchMe({
    String? displayName,
    bool? ageConfirmed,
    bool? privacyAccepted,
  }) async {
    final body = <String, dynamic>{};
    if (displayName != null) body['display_name'] = displayName;
    if (ageConfirmed != null) body['age_confirmed'] = ageConfirmed;
    if (privacyAccepted != null) body['privacy_accepted'] = privacyAccepted;

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
}

class ProfileException implements Exception {
  ProfileException(this.code, this.statusCode, this.body);
  final String code;
  final int statusCode;
  final String body;

  @override
  String toString() => 'ProfileException($code, $statusCode)';
}
