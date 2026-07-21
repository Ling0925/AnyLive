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

  testWidgets('login page shows email field send otp and age checkboxes',
      (tester) async {
    await tester.pumpWidget(
      const MaterialApp(home: LoginPage(config: config)),
    );
    expect(find.text('AnyLive Login'), findsOneWidget);
    expect(find.byType(TextField), findsOneWidget);
    expect(find.text('Send OTP'), findsOneWidget);
    expect(find.text('I confirm I am 18 or older'), findsOneWidget);
    expect(find.text('I accept the privacy policy'), findsOneWidget);
    expect(find.text('Privacy Policy'), findsOneWidget);
    expect(find.text('https://anylive.example/privacy'), findsOneWidget);
    expect(find.text('Terms of Service'), findsOneWidget);
    expect(find.text('https://anylive.example/terms'), findsOneWidget);
  });

  testWidgets('verify disabled until age confirmed', (tester) async {
    final httpClient = MockClient((request) async {
      if (request.url.path == '/api/v1/auth/otp/send') {
        return http.Response('', 204);
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

    await tester.enterText(find.byType(TextField), 'user@example.com');
    await tester.tap(find.text('Send OTP'));
    await tester.pumpAndSettle();

    expect(find.text('Verify & continue'), findsOneWidget);
    // Age not confirmed → button disabled.
    final verifyButton = tester.widget<FilledButton>(
      find.widgetWithText(FilledButton, 'Verify & continue'),
    );
    expect(verifyButton.onPressed, isNull);

    // Confirm age → button enabled.
    await tester.tap(find.text('I confirm I am 18 or older'));
    await tester.pump();
    final enabled = tester.widget<FilledButton>(
      find.widgetWithText(FilledButton, 'Verify & continue'),
    );
    expect(enabled.onPressed, isNotNull);
  });

  testWidgets('verify patches age/privacy then navigates', (tester) async {
    var patchCalled = false;
    Map<String, dynamic>? patchBody;

    final httpClient = MockClient((request) async {
      final path = request.url.path;
      if (path == '/api/v1/auth/otp/send') {
        return http.Response('', 204);
      }
      if (path == '/api/v1/auth/otp/verify') {
        return http.Response(
          jsonEncode({
            'access_token': 'tok-1',
            'refresh_token': 'ref-1',
            'expires_in': 3600,
            'user': {
              'id': 'u1',
              'display_name': 'Tester',
              'email': 'user@example.com',
            },
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
            'display_name': 'Tester',
            'email': 'user@example.com',
            'created_at': 't',
            'age_confirmed': true,
            'privacy_accepted': true,
          }),
          200,
          headers: {'content-type': 'application/json'},
        );
      }
      // Home / feed may hit other endpoints after navigation — soft-fail.
      return http.Response('{}', 200,
          headers: {'content-type': 'application/json'});
    });

    final api = ApiClient(baseUrl: config.normalizedApiBaseUrl);
    final auth = AuthRepository(client: api, httpClient: httpClient);

    await tester.pumpWidget(
      MaterialApp(
        home: LoginPage(
          config: config,
          authRepository: auth,
          profileRepositoryFactory: (client) => ProfileRepository(
            client: client,
            httpClient: httpClient,
          ),
        ),
      ),
    );

    await tester.enterText(find.byType(TextField).first, 'user@example.com');
    await tester.tap(find.text('Send OTP'));
    await tester.pumpAndSettle();

    await tester.tap(find.text('I confirm I am 18 or older'));
    await tester.tap(find.text('I accept the privacy policy'));
    await tester.pump();

    await tester.tap(find.text('Verify & continue'));
    await tester.pumpAndSettle();

    expect(patchCalled, isTrue);
    expect(patchBody?['age_confirmed'], isTrue);
    expect(patchBody?['privacy_accepted'], isTrue);
  });
}
