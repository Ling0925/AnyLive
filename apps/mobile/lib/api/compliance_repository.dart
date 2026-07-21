import 'dart:convert';

import 'package:http/http.dart' as http;

import 'api_client.dart';

/// Legal document link returned by `/api/v1/legal/*`.
class LegalDoc {
  LegalDoc({
    required this.url,
    required this.version,
    required this.title,
  });

  final String url;
  final String version;
  final String title;

  factory LegalDoc.fromJson(Map<String, dynamic> json) {
    return LegalDoc(
      url: json['url'] as String? ?? '',
      version: json['version'] as String? ?? '',
      title: json['title'] as String? ?? '',
    );
  }
}

/// Account export stub from `GET /api/v1/me/export`.
class AccountExport {
  AccountExport({
    required this.userId,
    required this.displayName,
    required this.email,
    required this.createdAt,
    required this.roomsOwnedCount,
    required this.note,
  });

  final String userId;
  final String displayName;
  final String? email;
  final String createdAt;
  final int roomsOwnedCount;
  final String note;

  factory AccountExport.fromJson(Map<String, dynamic> json) {
    final user = json['user'] as Map<String, dynamic>? ?? {};
    return AccountExport(
      userId: user['id'] as String? ?? '',
      displayName: user['display_name'] as String? ?? '',
      email: user['email'] as String?,
      createdAt: user['created_at'] as String? ?? '',
      roomsOwnedCount: json['rooms_owned_count'] as int? ?? 0,
      note: json['note'] as String? ?? '',
    );
  }
}

/// Compliance / DSAR API calls. [httpClient] injectable for tests.
class ComplianceRepository {
  ComplianceRepository({
    required this.client,
    http.Client? httpClient,
  }) : httpClient = httpClient ?? http.Client();

  final ApiClient client;
  final http.Client httpClient;

  Future<LegalDoc> fetchLegalPrivacy() async {
    final res = await httpClient.get(
      client.uri('/api/v1/legal/privacy'),
      headers: client.jsonHeaders(),
    );
    if (res.statusCode != 200) {
      throw ComplianceException('legal_privacy_failed', res.statusCode, res.body);
    }
    return LegalDoc.fromJson(jsonDecode(res.body) as Map<String, dynamic>);
  }

  Future<LegalDoc> fetchLegalTerms() async {
    final res = await httpClient.get(
      client.uri('/api/v1/legal/terms'),
      headers: client.jsonHeaders(),
    );
    if (res.statusCode != 200) {
      throw ComplianceException('legal_terms_failed', res.statusCode, res.body);
    }
    return LegalDoc.fromJson(jsonDecode(res.body) as Map<String, dynamic>);
  }

  Future<AccountExport> exportMe() async {
    final res = await httpClient.get(
      client.uri('/api/v1/me/export'),
      headers: client.jsonHeaders(auth: true),
    );
    if (res.statusCode != 200) {
      throw ComplianceException('export_me_failed', res.statusCode, res.body);
    }
    return AccountExport.fromJson(jsonDecode(res.body) as Map<String, dynamic>);
  }

  Future<void> deleteMe() async {
    final res = await httpClient.delete(
      client.uri('/api/v1/me'),
      headers: client.jsonHeaders(auth: true),
    );
    if (res.statusCode != 204) {
      throw ComplianceException('delete_me_failed', res.statusCode, res.body);
    }
  }
}

class ComplianceException implements Exception {
  ComplianceException(this.code, this.statusCode, this.body);
  final String code;
  final int statusCode;
  final String body;

  @override
  String toString() => 'ComplianceException($code, $statusCode)';
}
