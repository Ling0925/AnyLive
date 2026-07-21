import 'package:anylive_mobile/config/app_config.dart';
import 'package:anylive_mobile/features/rooms/room_list_page.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('room list page shows title', (tester) async {
    const config = AppConfig(
      apiBaseUrl: 'http://localhost:8088',
      environment: 'local',
    );
    await tester.pumpWidget(
      const MaterialApp(
        home: RoomListPage(config: config, accessToken: 'tok'),
      ),
    );
    // First frame shows loading indicator before async completes.
    expect(find.text('Live rooms'), findsOneWidget);
  });
}
