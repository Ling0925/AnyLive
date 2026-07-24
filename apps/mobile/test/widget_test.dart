import 'package:anylive_mobile/app.dart';
import 'package:anylive_mobile/l10n/locale_controller.dart';
import 'package:flutter/material.dart';
import 'package:anylive_mobile/api/api_client.dart';
import 'package:anylive_mobile/api/auth_repository.dart';
import 'package:anylive_mobile/api/session_store.dart';
import 'package:anylive_mobile/config/app_config.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shared_preferences/shared_preferences.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  const config = AppConfig(
    apiBaseUrl: 'http://localhost:8088',
    environment: 'local',
  );

  testWidgets('home shows AnyLive shell branding', (tester) async {
    SharedPreferences.setMockInitialValues({});
    await tester.pumpWidget(
      AnyLiveApp(
        localeController: LocaleController(initial: const Locale('en'), loaded: true),
        config: config,
        sessionStore: SessionStore(),
      ),
    );
    // Session restore Future completes.
    await tester.pumpAndSettle();
    expect(find.text('AnyLive'), findsOneWidget);
    // Logged-out MainShell welcome (or legacy "AnyLive Mobile" if present).
    expect(find.textContaining('AnyLive Mobile'), findsOneWidget);
    expect(find.textContaining('Sign in'), findsWidgets);
    expect(find.textContaining('http://localhost:8088'), findsOneWidget);
  });

  testWidgets('restores session from store when access still valid',
      (tester) async {
    SharedPreferences.setMockInitialValues({
      'anylive_session_v1':
          '{"user_id":"u1","display_name":"Restored","email":"r@example.com",'
          '"access_token":"tok","refresh_token":"r","expires_in":3600,'
          '"saved_at_ms":${DateTime.now().millisecondsSinceEpoch}}',
    });
    final mock = MockClient((request) async {
      if (request.url.path == '/api/v1/me') {
        return http.Response('{"id":"u1"}', 200,
            headers: {'content-type': 'application/json'});
      }
      return http.Response('{}', 200,
          headers: {'content-type': 'application/json'});
    });
    await tester.pumpWidget(
      AnyLiveApp(
        localeController: LocaleController(initial: const Locale('en'), loaded: true),
        config: config,
        sessionStore: SessionStore(),
        httpClient: mock,
      ),
    );
    await tester.pump(); // first frame of MainShell / feeds loading
    // Bottom nav shell is rooted after session restore.
    expect(find.text('Home'), findsWidgets);
    expect(find.text('You'), findsOneWidget);
    expect(find.text('Go Live'), findsOneWidget);
    // Open You tab to confirm session label surface.
    await tester.tap(find.text('You'));
    // Feeds may still be loading; pump without settle to avoid network hang.
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 100));
    expect(find.textContaining('signed in as Restored'), findsOneWidget);
  });

  testWidgets('refresh on restore when access invalid', (tester) async {
    SharedPreferences.setMockInitialValues({
      'anylive_session_v1':
          '{"user_id":"u1","display_name":"Ada","email":"a@b.com",'
          '"access_token":"old","refresh_token":"ref","expires_in":900,'
          '"saved_at_ms":0}',
    });
    var sawRefresh = false;
    final mock = MockClient((request) async {
      if (request.url.path == '/api/v1/me') {
        return http.Response('unauthorized', 401);
      }
      if (request.url.path == '/api/v1/auth/token/refresh') {
        sawRefresh = true;
        return http.Response(
          '{"access_token":"new","refresh_token":"ref2","expires_in":900}',
          200,
          headers: {'content-type': 'application/json'},
        );
      }
      return http.Response('{}', 200,
          headers: {'content-type': 'application/json'});
    });
    await tester.pumpWidget(
      AnyLiveApp(
        localeController: LocaleController(initial: const Locale('en'), loaded: true),
        config: config,
        sessionStore: SessionStore(),
        httpClient: mock,
      ),
    );
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 50));
    expect(sawRefresh, isTrue);
    expect(find.text('You'), findsOneWidget);
    final loaded = await SessionStore().load();
    expect(loaded?.accessToken, 'new');
  });

  testWidgets('clears session when refresh fails', (tester) async {
    SharedPreferences.setMockInitialValues({
      'anylive_session_v1':
          '{"user_id":"u1","display_name":"Ada","email":"a@b.com",'
          '"access_token":"old","refresh_token":"bad","expires_in":900,'
          '"saved_at_ms":0}',
    });
    final mock = MockClient((request) async {
      if (request.url.path == '/api/v1/me') {
        return http.Response('unauthorized', 401);
      }
      if (request.url.path == '/api/v1/auth/token/refresh') {
        return http.Response('nope', 401);
      }
      return http.Response('{}', 200,
          headers: {'content-type': 'application/json'});
    });
    await tester.pumpWidget(
      AnyLiveApp(
        localeController: LocaleController(initial: const Locale('en'), loaded: true),
        config: config,
        sessionStore: SessionStore(),
        httpClient: mock,
        authRepositoryFactory: (ApiClient c) =>
            AuthRepository(client: c, httpClient: mock),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.textContaining('Sign in'), findsWidgets);
    expect(await SessionStore().load(), isNull);
  });
}
