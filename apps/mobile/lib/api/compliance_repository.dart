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

/// Compliance / DSAR API calls. [httpClient] injectable for tests.
class ComplianceRepository {
  ComplianceRepository({
    required this.client,
    http.Client? httpClient,
  }) : httpClient = httpClient ?? http.Client();

  final ApiClient client;
  final http.Client httpClient;

  /// GET `/api/v1/legal/privacy`
  Future<LegalDoc> legalPrivacy() async {
    final res = await httpClient.get(
      client.uri('/api/v1/legal/privacy'),
      headers: client.jsonHeaders(),
    );
    if (res.statusCode != 200) {
      throw ComplianceException('legal_privacy_failed', res.statusCode, res.body);
    }
    return LegalDoc.fromJson(jsonDecode(res.body) as Map<String, dynamic>);
  }

  /// GET `/api/v1/legal/terms`
  Future<LegalDoc> legalTerms() async {
    final res = await httpClient.get(
      client.uri('/api/v1/legal/terms'),
      headers: client.jsonHeaders(),
    );
    if (res.statusCode != 200) {
      throw ComplianceException('legal_terms_failed', res.statusCode, res.body);
    }
    return LegalDoc.fromJson(jsonDecode(res.body) as Map<String, dynamic>);
  }

  /// GET `/api/v1/me/export` — returns the raw JSON map (P1 stub payload).
  Future<Map<String, dynamic>> exportMe() async {
    final res = await httpClient.get(
      client.uri('/api/v1/me/export'),
      headers: client.jsonHeaders(auth: true),
    );
    if (res.statusCode != 200) {
      throw ComplianceException('export_me_failed', res.statusCode, res.body);
    }
    return jsonDecode(res.body) as Map<String, dynamic>;
  }

  /// DELETE `/api/v1/me` — soft-delete account (P1 stub, expects 204).
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
