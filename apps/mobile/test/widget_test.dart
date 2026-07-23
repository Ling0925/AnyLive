import 'package:anylive_mobile/app.dart';
import 'package:anylive_mobile/api/session_store.dart';
import 'package:anylive_mobile/config/app_config.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shared_preferences/shared_preferences.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  testWidgets('home shows AnyLive shell branding', (tester) async {
    SharedPreferences.setMockInitialValues({});
    const config = AppConfig(
      apiBaseUrl: 'http://localhost:8088',
      environment: 'local',
    );
    await tester.pumpWidget(
      AnyLiveApp(
        config: config,
        sessionStore: SessionStore(),
      ),
    );
    // Session restore Future completes.
    await tester.pumpAndSettle();
    expect(find.text('AnyLive'), findsOneWidget);
    expect(find.textContaining('AnyLive Mobile'), findsOneWidget);
    expect(find.textContaining('http://localhost:8088'), findsOneWidget);
  });

  testWidgets('restores session from store', (tester) async {
    SharedPreferences.setMockInitialValues({
      'anylive_session_v1':
          '{"user_id":"u1","display_name":"Restored","email":"r@example.com","access_token":"tok","refresh_token":"r","expires_in":3600}',
    });
    const config = AppConfig(
      apiBaseUrl: 'http://localhost:8088',
      environment: 'local',
    );
    await tester.pumpWidget(
      AnyLiveApp(
        config: config,
        sessionStore: SessionStore(),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.textContaining('signed in as Restored'), findsOneWidget);
    expect(find.text('Browse live rooms'), findsOneWidget);
  });
}
