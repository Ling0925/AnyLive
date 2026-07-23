import 'dart:convert';

import 'package:http/http.dart' as http;

import 'api_client.dart';

/// One client analytics event for POST `/api/v1/events`.
class ClientEvent {
  ClientEvent({
    required this.name,
    this.occurredAt,
    this.props,
    this.clientEventId,
  });

  final String name;
  final DateTime? occurredAt;
  final Map<String, dynamic>? props;
  final String? clientEventId;

  Map<String, dynamic> toJson() {
    final m = <String, dynamic>{'name': name};
    if (occurredAt != null) {
      m['occurred_at'] = occurredAt!.toUtc().toIso8601String();
    }
    if (props != null) m['props'] = props;
    if (clientEventId != null) m['client_event_id'] = clientEventId;
    return m;
  }
}

/// Result of batch ingest.
class ClientEventIngestResult {
  ClientEventIngestResult({required this.accepted, required this.dropped});

  final int accepted;
  final int dropped;

  factory ClientEventIngestResult.fromJson(Map<String, dynamic> json) {
    return ClientEventIngestResult(
      accepted: _asInt(json['accepted']),
      dropped: _asInt(json['dropped']),
    );
  }

  static int _asInt(Object? v) {
    if (v is int) return v;
    if (v is num) return v.toInt();
    return 0;
  }
}

/// Client analytics batch ingest (P4). Failures are non-fatal for product UI.
class EventsRepository {
  EventsRepository({
    required this.client,
    http.Client? httpClient,
  }) : httpClient = httpClient ?? http.Client();

  final ApiClient client;
  final http.Client httpClient;

  /// POST `/api/v1/events` — returns accepted/dropped counts.
  Future<ClientEventIngestResult> ingest(List<ClientEvent> events) async {
    final res = await httpClient.post(
      client.uri('/api/v1/events'),
      headers: client.jsonHeaders(auth: true),
      body: jsonEncode({
        'events': events.map((e) => e.toJson()).toList(),
      }),
    );
    // 202 Accepted is success; 200 also tolerated.
    if (res.statusCode != 202 && res.statusCode != 200) {
      throw EventsException('ingest_failed', res.statusCode, res.body);
    }
    if (res.body.isEmpty) {
      return ClientEventIngestResult(accepted: events.length, dropped: 0);
    }
    return ClientEventIngestResult.fromJson(
      jsonDecode(res.body) as Map<String, dynamic>,
    );
  }

  /// Fire-and-forget single event (swallows errors).
  Future<void> track(
    String name, {
    Map<String, dynamic>? props,
    String? clientEventId,
  }) async {
    try {
      await ingest([
        ClientEvent(
          name: name,
          occurredAt: DateTime.now().toUtc(),
          props: props,
          clientEventId: clientEventId,
        ),
      ]);
    } catch (_) {
      // Analytics must not break UX.
    }
  }
}

class EventsException implements Exception {
  EventsException(this.code, this.statusCode, this.body);
  final String code;
  final int statusCode;
  final String body;

  @override
  String toString() => 'EventsException($code, $statusCode)';
}
