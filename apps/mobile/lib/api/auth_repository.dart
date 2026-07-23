import 'dart:convert';

import 'package:http/http.dart' as http;

import 'api_client.dart';

class AuthSession {
  AuthSession({
    required this.userId,
    required this.displayName,
    required this.email,
    required this.accessToken,
    required this.refreshToken,
    required this.expiresIn,
    this.username,
    this.mustChangePassword = false,
  });

  final String userId;
  final String displayName;
  final String? email;
  final String? username;
  final String accessToken;
  final String refreshToken;
  final int expiresIn;
  final bool mustChangePassword;

  factory AuthSession.fromJson(Map<String, dynamic> json) {
    final user = json['user'] as Map<String, dynamic>? ?? {};
    return AuthSession(
      userId: user['id'] as String? ?? '',
      displayName: user['display_name'] as String? ?? '',
      email: user['email'] as String?,
      username: user['username'] as String?,
      accessToken: json['access_token'] as String? ?? '',
      refreshToken: json['refresh_token'] as String? ?? '',
      expiresIn: json['expires_in'] as int? ?? 0,
      mustChangePassword: json['must_change_password'] as bool? ?? false,
    );
  }
}

/// Auth API calls. [httpClient] injectable for tests.
class AuthRepository {
  AuthRepository({
    required this.client,
    http.Client? httpClient,
  }) : httpClient = httpClient ?? http.Client();

  final ApiClient client;
  final http.Client httpClient;

  /// Password login by email or username.
  Future<AuthSession> passwordLogin({
    required String identifier,
    required String password,
  }) async {
    final res = await httpClient.post(
      client.uri('/api/v1/auth/password/login'),
      headers: client.jsonHeaders(),
      body: jsonEncode({
        'identifier': identifier,
        'password': password,
      }),
    );
    if (res.statusCode != 200) {
      throw AuthException('password_login_failed', res.statusCode, res.body);
    }
    final map = jsonDecode(res.body) as Map<String, dynamic>;
    final session = AuthSession.fromJson(map);
    client.accessToken = session.accessToken;
    return session;
  }

  /// Change password for the authenticated user.
  Future<void> changePassword({
    required String currentPassword,
    required String newPassword,
  }) async {
    final res = await httpClient.post(
      client.uri('/api/v1/auth/password/change'),
      headers: client.jsonHeaders(auth: true),
      body: jsonEncode({
        'current_password': currentPassword,
        'new_password': newPassword,
      }),
    );
    if (res.statusCode != 204 && res.statusCode != 200) {
      throw AuthException('password_change_failed', res.statusCode, res.body);
    }
  }

  Future<void> sendOtp(String email) async {
    final res = await httpClient.post(
      client.uri('/api/v1/auth/otp/send'),
      headers: client.jsonHeaders(),
      body: jsonEncode({'email': email}),
    );
    if (res.statusCode != 204) {
      throw AuthException('send_otp_failed', res.statusCode, res.body);
    }
  }

  Future<AuthSession> verifyOtp({
    required String email,
    required String code,
  }) async {
    final res = await httpClient.post(
      client.uri('/api/v1/auth/otp/verify'),
      headers: client.jsonHeaders(),
      body: jsonEncode({'email': email, 'code': code}),
    );
    if (res.statusCode != 200) {
      throw AuthException('verify_otp_failed', res.statusCode, res.body);
    }
    final map = jsonDecode(res.body) as Map<String, dynamic>;
    final session = AuthSession.fromJson(map);
    client.accessToken = session.accessToken;
    return session;
  }

  /// List active refresh sessions (`GET /api/v1/me/sessions`).
  Future<List<RefreshSessionInfo>> listSessions() async {
    final res = await httpClient.get(
      client.uri('/api/v1/me/sessions'),
      headers: client.jsonHeaders(auth: true),
    );
    if (res.statusCode != 200) {
      throw AuthException('list_sessions_failed', res.statusCode, res.body);
    }
    final map = jsonDecode(res.body) as Map<String, dynamic>;
    final items = map['items'] as List<dynamic>? ?? [];
    return items
        .map((e) => RefreshSessionInfo.fromJson(e as Map<String, dynamic>))
        .toList();
  }

  /// Revoke all refresh sessions (`DELETE /api/v1/me/sessions`).
  Future<int> logoutAllSessions() async {
    final res = await httpClient.delete(
      client.uri('/api/v1/me/sessions'),
      headers: client.jsonHeaders(auth: true),
    );
    if (res.statusCode != 200) {
      throw AuthException('logout_all_failed', res.statusCode, res.body);
    }
    final map = jsonDecode(res.body) as Map<String, dynamic>;
    return (map['revoked'] as num?)?.toInt() ?? 0;
  }

  /// Revoke one refresh session by jti (`DELETE /api/v1/me/sessions/{jti}`).
  Future<void> revokeSession(String jti) async {
    final id = Uri.encodeComponent(jti.trim());
    final res = await httpClient.delete(
      client.uri('/api/v1/me/sessions/$id'),
      headers: client.jsonHeaders(auth: true),
    );
    if (res.statusCode != 204 && res.statusCode != 200) {
      throw AuthException('revoke_session_failed', res.statusCode, res.body);
    }
  }
}

class RefreshSessionInfo {
  RefreshSessionInfo({required this.jti, required this.expiresAt});

  final String jti;
  final String expiresAt;

  factory RefreshSessionInfo.fromJson(Map<String, dynamic> json) {
    return RefreshSessionInfo(
      jti: json['jti'] as String? ?? '',
      expiresAt: json['expires_at'] as String? ?? '',
    );
  }
}

class AuthException implements Exception {
  AuthException(this.code, this.statusCode, this.body);

  final String code;
  final int statusCode;
  final String body;

  @override
  String toString() => 'AuthException($code, $statusCode): $body';
}
