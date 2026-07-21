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
  });

  final String userId;
  final String displayName;
  final String? email;
  final String accessToken;
  final String refreshToken;
  final int expiresIn;

  factory AuthSession.fromJson(Map<String, dynamic> json) {
    final user = json['user'] as Map<String, dynamic>? ?? {};
    return AuthSession(
      userId: user['id'] as String? ?? '',
      displayName: user['display_name'] as String? ?? '',
      email: user['email'] as String?,
      accessToken: json['access_token'] as String? ?? '',
      refreshToken: json['refresh_token'] as String? ?? '',
      expiresIn: json['expires_in'] as int? ?? 0,
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
}

class AuthException implements Exception {
  AuthException(this.code, this.statusCode, this.body);
  final String code;
  final int statusCode;
  final String body;

  @override
  String toString() => 'AuthException($code, $statusCode)';
}
