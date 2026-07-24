import 'package:anylive_mobile/player/hls_player_logic.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  group('shouldShowPlayer', () {
    test('true when live with non-empty url', () {
      expect(
        shouldShowPlayer('live', 'http://cdn/live/r1.m3u8'),
        isTrue,
      );
    });

    test('false when not live', () {
      expect(shouldShowPlayer('idle', 'http://cdn/live/r1.m3u8'), isFalse);
      expect(shouldShowPlayer('closed', 'http://cdn/live/r1.m3u8'), isFalse);
      expect(shouldShowPlayer('ended', 'http://cdn/live/r1.m3u8'), isFalse);
    });

    test('false when live but url missing/blank', () {
      expect(shouldShowPlayer('live', null), isFalse);
      expect(shouldShowPlayer('live', ''), isFalse);
      expect(shouldShowPlayer('live', '   '), isFalse);
    });
  });

  group('playerPlaceholderMessage', () {
    test('terminal statuses', () {
      expect(playerPlaceholderMessage('closed', null), 'Stream ended');
      expect(playerPlaceholderMessage('ended', 'x'), 'Stream ended');
      expect(
        playerPlaceholderSubline('closed'),
        'This room was force-closed',
      );
    });

    test('idle is offline (host stop), not permanent end', () {
      expect(playerPlaceholderMessage('idle', null), 'Host offline');
      expect(
        playerPlaceholderSubline('idle'),
        'Host stopped — may go live again',
      );
      expect(isRoomTerminalStatus('idle'), isFalse);
      expect(isRoomOfflineStatus('idle'), isTrue);
      expect(isRoomTerminalStatus('closed'), isTrue);
    });

    test('offline non-ended', () {
      expect(playerPlaceholderMessage('scheduled', null), 'Host offline');
    });

    test('live without url', () {
      expect(
        playerPlaceholderMessage('live', null),
        'Live — play URL unavailable',
      );
      expect(
        playerPlaceholderMessage('live', ''),
        'Live — play URL unavailable',
      );
    });

    test('live with url', () {
      expect(
        playerPlaceholderMessage('live', 'http://cdn/r1.m3u8'),
        'Open stream URL in external player',
      );
    });
  });

  group('isLikelyHlsUrl', () {
    test('detects m3u8', () {
      expect(isLikelyHlsUrl('http://cdn/live/r1.m3u8'), isTrue);
      expect(isLikelyHlsUrl('https://x.example/a/b.M3U8?token=1'), isTrue);
    });

    test('detects /hls/ path', () {
      expect(isLikelyHlsUrl('https://cdn.example/hls/stream'), isTrue);
    });

    test('rejects empty and non-hls', () {
      expect(isLikelyHlsUrl(null), isFalse);
      expect(isLikelyHlsUrl(''), isFalse);
      expect(isLikelyHlsUrl('https://cdn.example/video.mp4'), isFalse);
    });
  });

  group('normalizePlaybackUrl', () {
    test('rewrites localhost and 0.0.0.0 to preferHost', () {
      expect(
        normalizePlaybackUrl('http://localhost:8080/live/r1.m3u8'),
        'http://127.0.0.1:8080/live/r1.m3u8',
      );
      expect(
        normalizePlaybackUrl('http://0.0.0.0:8080/live/r1.m3u8'),
        'http://127.0.0.1:8080/live/r1.m3u8',
      );
      expect(
        normalizePlaybackUrl(
          'http://[::1]:8080/live/r1.m3u8',
          preferHost: '127.0.0.1',
        ),
        'http://127.0.0.1:8080/live/r1.m3u8',
      );
    });

    test('leaves non-loopback hosts unchanged', () {
      expect(
        normalizePlaybackUrl('http://cdn.example/live/r1.m3u8'),
        'http://cdn.example/live/r1.m3u8',
      );
      expect(
        normalizePlaybackUrl('http://127.0.0.1:8080/live/r1.m3u8'),
        'http://127.0.0.1:8080/live/r1.m3u8',
      );
    });
  });
}
