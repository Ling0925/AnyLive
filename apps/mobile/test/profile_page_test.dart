import 'package:anylive_mobile/api/api_client.dart';
import 'package:anylive_mobile/api/profile_repository.dart';
import 'package:anylive_mobile/config/app_config.dart';
import 'package:anylive_mobile/features/home/home_page.dart';
import 'package:anylive_mobile/features/profile/profile_page.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

class FakeProfileRepo extends ProfileRepository {
  FakeProfileRepo({
    this.initial,
    this.loadError,
    this.saveError,
  }) : super(client: ApiClient(baseUrl: 'http://test'));

  UserProfile? initial;
  ProfileException? loadError;
  ProfileException? saveError;
  String? lastPatchedName;
  bool? lastAgeConfirmed;
  bool? lastPrivacyAccepted;
  int getMeCalls = 0;
  int patchMeCalls = 0;

  @override
  Future<UserProfile> getMe() async {
    getMeCalls++;
    if (loadError != null) throw loadError!;
    return initial ??
        UserProfile(
          id: 'u1',
          displayName: 'Ada',
          email: 'ada@example.com',
          createdAt: '2026-01-01T00:00:00Z',
          ageConfirmed: false,
          privacyAccepted: false,
        );
  }

  @override
  Future<UserProfile> patchMe({
    String? displayName,
    bool? ageConfirmed,
    bool? privacyAccepted,
  }) async {
    patchMeCalls++;
    lastPatchedName = displayName;
    lastAgeConfirmed = ageConfirmed;
    lastPrivacyAccepted = privacyAccepted;
    if (saveError != null) throw saveError!;
    final base = initial ??
        UserProfile(
          id: 'u1',
          displayName: 'Ada',
          email: 'ada@example.com',
          createdAt: '2026-01-01T00:00:00Z',
          ageConfirmed: false,
          privacyAccepted: false,
        );
    final updated = UserProfile(
      id: base.id,
      displayName: displayName ?? base.displayName,
      email: base.email,
      createdAt: base.createdAt,
      ageConfirmed: ageConfirmed ?? base.ageConfirmed,
      privacyAccepted: privacyAccepted ?? base.privacyAccepted,
    );
    initial = updated;
    return updated;
  }
}

void main() {
  const config = AppConfig(
    apiBaseUrl: 'http://localhost:8088',
    environment: 'local',
  );

  testWidgets('profile page loads display name and saves', (tester) async {
    final fake = FakeProfileRepo(
      initial: UserProfile(
        id: 'u1',
        displayName: 'Ada',
        email: 'ada@example.com',
        createdAt: 't',
        ageConfirmed: false,
        privacyAccepted: false,
      ),
    );
    String? saved;
    await tester.pumpWidget(
      MaterialApp(
        home: ProfilePage(
          config: config,
          accessToken: 'tok',
          profileRepository: fake,
          onDisplayNameChanged: (n) => saved = n,
        ),
      ),
    );
    expect(find.byType(CircularProgressIndicator), findsOneWidget);
    await tester.pumpAndSettle();

    expect(find.text('Edit profile'), findsOneWidget);
    expect(find.text('ada@example.com'), findsOneWidget);
    expect(find.widgetWithText(TextField, 'Ada'), findsOneWidget);
    expect(find.text('I confirm I am 18 or older'), findsOneWidget);
    expect(find.text('I accept the privacy policy'), findsOneWidget);

    await tester.enterText(find.byType(TextField), 'Patched');
    await tester.tap(find.text('I confirm I am 18 or older'));
    await tester.tap(find.text('I accept the privacy policy'));
    await tester.tap(find.text('Save'));
    await tester.pumpAndSettle();

    expect(fake.patchMeCalls, 1);
    expect(fake.lastPatchedName, 'Patched');
    expect(fake.lastAgeConfirmed, isTrue);
    expect(fake.lastPrivacyAccepted, isTrue);
    expect(saved, 'Patched');
    expect(find.text('Profile saved'), findsOneWidget);
  });

  testWidgets('home shows Profile action when logged in', (tester) async {
    await tester.pumpWidget(
      const MaterialApp(
        home: HomePage(
          config: config,
          sessionLabel: 'Ada',
          accessToken: 'tok',
        ),
      ),
    );
    expect(find.text('Profile'), findsOneWidget);
    expect(find.text('signed in as Ada'), findsOneWidget);
    expect(find.text('Login'), findsNothing);
  });
}
