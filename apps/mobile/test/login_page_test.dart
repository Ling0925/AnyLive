import 'package:anylive_mobile/config/app_config.dart';
import 'package:anylive_mobile/features/auth/login_page.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('login page shows email field and send otp', (tester) async {
    const config = AppConfig(
      apiBaseUrl: 'http://localhost:8088',
      environment: 'local',
    );
    await tester.pumpWidget(
      const MaterialApp(home: LoginPage(config: config)),
    );
    expect(find.text('AnyLive Login'), findsOneWidget);
    expect(find.byType(TextField), findsOneWidget);
    expect(find.text('Send OTP'), findsOneWidget);
    expect(find.text('Privacy Policy'), findsOneWidget);
    expect(find.text('https://anylive.example/privacy'), findsOneWidget);
    expect(find.text('Terms of Service'), findsOneWidget);
    expect(find.text('https://anylive.example/terms'), findsOneWidget);
  });
}
