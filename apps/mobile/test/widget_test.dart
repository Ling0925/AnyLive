import 'package:anylive_mobile/app.dart';
import 'package:anylive_mobile/config/app_config.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('home shows AnyLive shell branding', (tester) async {
    const config = AppConfig(
      apiBaseUrl: 'http://localhost:8088',
      environment: 'local',
    );
    await tester.pumpWidget(const AnyLiveApp(config: config));
    expect(find.text('AnyLive'), findsOneWidget);
    expect(find.textContaining('AnyLive Mobile'), findsOneWidget);
    expect(find.textContaining('http://localhost:8088'), findsOneWidget);
  });
}
