import 'dart:convert';

import 'package:anylive_mobile/api/api_client.dart';
import 'package:anylive_mobile/api/auth_repository.dart';
import 'package:anylive_mobile/api/profile_repository.dart';
import 'package:anylive_mobile/config/app_config.dart';
import 'package:anylive_mobile/features/auth/login_page.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';

void main() {
  const config = AppConfig(
    apiBaseUrl: 'http://localhost:8088',
    environment: 'local',
  );

  testWidgets('login page shows password fields and age checkboxes',
      (tester) async {
    await tester.pumpWidget(
      const MaterialApp(home: LoginPage(config: config)),
    );
    expect(find.text('AnyLive Login'), findsOneWidget);
    expect(find.text('Email or username'), findsOneWidget);
    expect(find.text('Password'), findsOneWidget);
    expect(find.text('Sign in'), findsOneWidget);
    expect(find.text('I confirm I am 18 or older'), findsOneWidget);
    expect(find.text('I accept the privacy policy'), findsOneWidget);
    expect(find.text('Privacy Policy'), findsOneWidget);
    expect(find.text('https://anylive.example/privacy'), findsOneWidget);
    expect(find.text('Terms of Service'), findsOneWidget);
    expect(find.text('https://anylive.example/terms'), findsOneWidget);
    expect(find.text('Dev OTP (local only)'), findsOneWidget);
  });

  testWidgets('sign in disabled until age confirmed', (tester) async {
    final httpClient = MockClient((request) async {
      if (request.url.path == '/api/v1/auth/password/login') {
        return http.Response(
          jsonEncode({
            'user': {
              'id': 'u1',
              'display_name': 'host',
              'username': 'host1',
              'created_at': '2026-01-01T00:00:00Z',
            },
            'access_token': 'acc',
            'refresh_token': 'ref',
            'expires_in': 900,
            'must_change_password': false,
          }),
          200,
          headers: {'content-type': 'application/json'},
        );
      }
      return http.Response('unexpected', 500);
    });
    final api = ApiClient(baseUrl: config.normalizedApiBaseUrl);
    final auth = AuthRepository(client: api, httpClient: httpClient);

    await tester.pumpWidget(
      MaterialApp(
        home: LoginPage(config: config, authRepository: auth),
      ),
    );

    await tester.enterText(
      find.widgetWithText(TextField, 'Email or username'),
      'host1',
    );
    await tester.enterText(
      find.widgetWithText(TextField, 'Password'),
      'secret-pass-1',
    );
    await tester.pump();

    final signInDisabled = tester.widget<FilledButton>(
      find.widgetWithText(FilledButton, 'Sign in'),
    );
    expect(signInDisabled.onPressed, isNull);

    await tester.tap(find.text('I confirm I am 18 or older'));
    await tester.pump();
    final signInEnabled = tester.widget<FilledButton>(
      find.widgetWithText(FilledButton, 'Sign in'),
    );
    expect(signInEnabled.onPressed, isNotNull);
  });

  testWidgets('password login patches age/privacy then calls onLoggedIn',
      (tester) async {
    var patchCalled = false;
    Map<String, dynamic>? patchBody;
    AuthSession? loggedIn;

    final httpClient = MockClient((request) async {
      final path = request.url.path;
      if (path == '/api/v1/auth/password/login') {
        return http.Response(
          jsonEncode({
            'user': {
              'id': 'u1',
              'display_name': 'host',
              'email': 'host@example.com',
              'username': 'host1',
              'created_at': '2026-01-01T00:00:00Z',
            },
            'access_token': 'acc',
            'refresh_token': 'ref',
            'expires_in': 900,
            'must_change_password': false,
          }),
          200,
          headers: {'content-type': 'application/json'},
        );
      }
      if (path == '/api/v1/me' && request.method == 'PATCH') {
        patchCalled = true;
        patchBody = jsonDecode(request.body) as Map<String, dynamic>;
        return http.Response(
          jsonEncode({
            'id': 'u1',
            'display_name': 'host',
            'email': 'host@example.com',
            'created_at': '2026-01-01T00:00:00Z',
            'age_confirmed': true,
            'privacy_accepted': true,
          }),
          200,
          headers: {'content-type': 'application/json'},
        );
      }
      return http.Response('unexpected $path', 500);
    });
    final api = ApiClient(baseUrl: config.normalizedApiBaseUrl);
    final auth = AuthRepository(client: api, httpClient: httpClient);

    await tester.pumpWidget(
      MaterialApp(
        home: LoginPage(
          config: config,
          authRepository: auth,
          profileRepositoryFactory: (c) => ProfileRepository(
            client: c,
            httpClient: httpClient,
          ),
          onLoggedIn: (s) => loggedIn = s,
        ),
      ),
    );

    await tester.enterText(
      find.widgetWithText(TextField, 'Email or username'),
      'host1',
    );
    await tester.enterText(
      find.widgetWithText(TextField, 'Password'),
      'secret-pass-1',
    );
    await tester.tap(find.text('I confirm I am 18 or older'));
    await tester.tap(find.text('I accept the privacy policy'));
    await tester.pump();
    await tester.tap(find.text('Sign in'));
    await tester.pumpAndSettle();

    expect(loggedIn, isNotNull);
    expect(loggedIn!.accessToken, 'acc');
    expect(patchCalled, isTrue);
    expect(patchBody?['age_confirmed'], isTrue);
    expect(patchBody?['privacy_accepted'], isTrue);
  });
}
