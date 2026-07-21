import 'dart:convert';

import 'package:http/http.dart' as http;

import 'api_client.dart';

/// Report created via POST `/api/v1/reports`.
class Report {
  Report({
    required this.id,
    required this.targetType,
    required this.targetId,
    required this.reason,
    required this.status,
    required this.createdAt,
  });

  final String id;
  final String targetType;
  final String targetId;
  final String reason;
  final String status;
  final String createdAt;

  factory Report.fromJson(Map<String, dynamic> json) {
    return Report(
      id: json['id'] as String? ?? '',
      targetType: json['target_type'] as String? ?? '',
      targetId: json['target_id'] as String? ?? '',
      reason: json['reason'] as String? ?? '',
      status: json['status'] as String? ?? 'open',
      createdAt: json['created_at'] as String? ?? '',
    );
  }
}

/// User reports (moderation intake). [httpClient] injectable for tests.
class ReportsRepository {
  ReportsRepository({
    required this.client,
    http.Client? httpClient,
  }) : httpClient = httpClient ?? http.Client();

  final ApiClient client;
  final http.Client httpClient;

  /// POST `/api/v1/reports` — expects 201.
  Future<Report> createReport({
    required String targetType,
    required String targetId,
    required String reason,
  }) async {
    final res = await httpClient.post(
      client.uri('/api/v1/reports'),
      headers: client.jsonHeaders(auth: true),
      body: jsonEncode({
        'target_type': targetType,
        'target_id': targetId,
        'reason': reason,
      }),
    );
    if (res.statusCode != 201) {
      throw ReportsException('create_failed', res.statusCode, res.body);
    }
    return Report.fromJson(jsonDecode(res.body) as Map<String, dynamic>);
  }
}

class ReportsException implements Exception {
  ReportsException(this.code, this.statusCode, this.body);
  final String code;
  final int statusCode;
  final String body;

  @override
  String toString() => 'ReportsException($code, $statusCode)';
}
