import 'package:anylive_mobile/realtime/centrifugo_chat.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  group('parseChatPublication', () {
    test('envelope chat.message', () {
      final msg = parseChatPublication({
        'type': 'chat.message',
        'payload': {
          'id': 'm1',
          'body': 'hello',
          'sender_name': 'Alice',
          'sender_id': 'u1',
        },
      });
      expect(msg, isNotNull);
      expect(msg!['body'], 'hello');
      expect(msg['senderName'], 'Alice');
      expect(msg['id'], 'm1');
    });

    test('bare body', () {
      final msg = parseChatPublication({
        'id': 'm2',
        'body': 'hi',
        'senderName': 'Bob',
      });
      expect(msg?['body'], 'hi');
      expect(msg?['senderName'], 'Bob');
    });

    test('rejects empty', () {
      expect(parseChatPublication(null), isNull);
      expect(parseChatPublication({'type': 'gift.sent'}), isNull);
    });
  });
}
