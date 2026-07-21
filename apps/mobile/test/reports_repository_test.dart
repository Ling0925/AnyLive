import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:anylive_mobile/api/api_client.dart';
import 'package:anylive_mobile/api/reports_repository.dart';

void main() {
  group('ReportsRepository', () {
    test('createReport posts body and parses response', () async {
      final mock = MockClient((request) async {
        expect(request.url.path, '/api/v1/reports');
        expect(request.method, 'POST');
        expect(request.headers['Authorization'], 'Bearer tok');
        final body = jsonDecode(request.body) as Map<String, dynamic>;
        expect(body['target_type'], 'room');
        expect(body['target_id'], 'r1');
        expect(body['reason'], 'spam');
        return http.Response(
          jsonEncode({
            'id': 'rep1',
            'target_type': 'room',
            'target_id': 'r1',
            'reason': 'spam',
            'status': 'open',
            'created_at': '2026-01-01T00:00:00Z',
          }),
          201,
          headers: {'content-type': 'application/json'},
        );
      });
      final api = ApiClient(baseUrl: 'http://localhost:8088', accessToken: 'tok');
      final repo = ReportsRepository(client: api, httpClient: mock);
      final report = await repo.createReport(
        targetType: 'room',
        targetId: 'r1',
        reason: 'spam',
      );
      expect(report.id, 'rep1');
      expect(report.targetType, 'room');
      expect(report.targetId, 'r1');
      expect(report.reason, 'spam');
      expect(report.status, 'open');
    });

    test('createReport throws on error status', () async {
      final mock = MockClient((_) async => http.Response('bad', 400));
      final repo = ReportsRepository(
        client: ApiClient(baseUrl: 'http://x', accessToken: 't'),
        httpClient: mock,
      );
      expect(
        () => repo.createReport(
          targetType: 'room',
          targetId: 'r1',
          reason: 'x',
        ),
        throwsA(isA<ReportsException>()),
      );
    });
  });
}
