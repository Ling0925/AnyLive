import 'package:anylive_mobile/api/api_client.dart';
import 'package:anylive_mobile/api/compliance_repository.dart';
import 'package:anylive_mobile/player/stream_preview.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('stream preview ends state', (tester) async {
    await tester.pumpWidget(
      const MaterialApp(
        home: Scaffold(
          body: StreamPreview(
            status: 'closed',
            hlsUrl: 'http://cdn/live/r1.m3u8',
            enableEmbeddedPlayer: false,
          ),
        ),
      ),
    );
    expect(find.text('Stream ended'), findsOneWidget);
    expect(find.text('This room was force-closed'), findsOneWidget);
    expect(find.byKey(const Key('stream-ended')), findsOneWidget);
  });

  testWidgets('stream preview idle is offline not permanent end', (tester) async {
    await tester.pumpWidget(
      const MaterialApp(
        home: Scaffold(
          body: StreamPreview(
            status: 'idle',
            hlsUrl: 'http://cdn/live/r1.m3u8',
            enableEmbeddedPlayer: false,
          ),
        ),
      ),
    );
    expect(find.text('Stream ended'), findsNothing);
    expect(find.text('Host offline'), findsOneWidget);
    expect(find.text('Host stopped — may go live again'), findsOneWidget);
    expect(find.byKey(const Key('stream-offline')), findsOneWidget);
  });

  testWidgets('stream preview shows copy url without native player',
      (tester) async {
    await tester.pumpWidget(
      const MaterialApp(
        home: Scaffold(
          body: StreamPreview(
            status: 'live',
            hlsUrl: 'http://cdn/live/r1.m3u8',
            enableEmbeddedPlayer: false,
          ),
        ),
      ),
    );
    expect(find.byKey(const Key('stream-preview-play-icon')), findsOneWidget);
    expect(find.text('http://cdn/live/r1.m3u8'), findsOneWidget);
    expect(find.byKey(const Key('copy-stream-url')), findsOneWidget);
    await tester.tap(find.byKey(const Key('copy-stream-url')));
    await tester.pump();
    expect(find.byKey(const Key('stream-url-copied-snackbar')), findsOneWidget);
  });
}

// Silence unused import if analyzer is picky in some configs.
// ignore: unused_element
void _touchCompliance() {
  ComplianceRepository(client: ApiClient(baseUrl: 'http://x'));
}
