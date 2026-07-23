import 'dart:async';
import 'dart:convert';

import 'package:web_socket_channel/web_socket_channel.dart';

/// Pure helpers for Centrifugo connection + message parsing (testable offline).
///
/// Clients still poll HTTP history as a fallback when WS is unavailable.

class RealtimeTokenInfo {
  RealtimeTokenInfo({
    required this.token,
    required this.expiresIn,
    required this.channels,
  });

  final String token;
  final int expiresIn;
  final List<String> channels;
}

/// Extract chat body from a Centrifugo publication data blob.
/// Supports both envelope `{ type, payload }` and bare chat message shapes.
Map<String, String>? parseChatPublication(Object? data) {
  if (data is! Map) return null;
  final root = Map<String, dynamic>.from(data);
  Map<String, dynamic> payload;
  if (root['type'] == 'chat.message' && root['payload'] is Map) {
    payload = Map<String, dynamic>.from(root['payload'] as Map);
  } else {
    payload = root;
  }
  final body = payload['body']?.toString() ?? '';
  if (body.isEmpty) return null;
  return {
    'id': payload['id']?.toString() ?? 'ws-${DateTime.now().millisecondsSinceEpoch}',
    'body': body,
    'senderName': (payload['sender_name'] ?? payload['senderName'] ?? '').toString(),
    'senderId': (payload['sender_id'] ?? payload['senderId'] ?? '').toString(),
  };
}

/// Minimal Centrifugo JSON protocol client (connect + subscribe).
/// Returns a stop function. Falls back silently if WS cannot open.
void Function() connectCentrifugoChat({
  required String wsUrl,
  required String token,
  required String channel,
  required void Function(Map<String, String> msg) onMessage,
  void Function(String status)? onStatus,
}) {
  var closed = false;
  WebSocketChannel? channelWs;
  var cmdId = 0;
  int nextId() {
    cmdId += 1;
    return cmdId;
  }

  try {
    onStatus?.call('connecting');
    channelWs = WebSocketChannel.connect(Uri.parse(wsUrl));
  } catch (_) {
    onStatus?.call('error');
    return () {
      closed = true;
    };
  }

  onStatus?.call('open');
  channelWs!.sink.add(jsonEncode({
    'id': nextId(),
    'connect': {'token': token},
  }));
  channelWs.sink.add(jsonEncode({
    'id': nextId(),
    'subscribe': {'channel': channel},
  }));

  final sub = channelWs.stream.listen(
    (ev) {
      if (closed) return;
      Object? data;
      try {
        data = jsonDecode(ev.toString());
      } catch (_) {
        return;
      }
      if (data is! Map) return;
      final root = Map<String, dynamic>.from(data);
      final push = root['push'];
      if (push is Map) {
        final pub = push['pub'];
        if (pub is Map && pub.containsKey('data')) {
          final msg = parseChatPublication(pub['data']);
          if (msg != null) onMessage(msg);
        }
      }
      final result = root['result'];
      if (result is Map && result['publications'] is List) {
        for (final pub in result['publications'] as List) {
          if (pub is Map && pub.containsKey('data')) {
            final msg = parseChatPublication(pub['data']);
            if (msg != null) onMessage(msg);
          }
        }
      }
    },
    onError: (_) => onStatus?.call('error'),
    onDone: () => onStatus?.call('closed'),
    cancelOnError: false,
  );

  return () {
    closed = true;
    try {
      sub.cancel();
      channelWs?.sink.close();
    } catch (_) {}
    channelWs = null;
  };
}
