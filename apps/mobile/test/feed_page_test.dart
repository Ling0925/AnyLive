import 'package:anylive_mobile/api/api_client.dart';
import 'package:anylive_mobile/api/rooms_repository.dart';
import 'package:anylive_mobile/api/social_repository.dart';
import 'package:anylive_mobile/config/app_config.dart';
import 'package:anylive_mobile/features/feed/feed_page.dart';
import 'package:anylive_mobile/features/home/home_page.dart';
import 'package:anylive_mobile/ui/feed_skeleton.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

class FakeSocialRepo extends SocialRepository {
  FakeSocialRepo({
    this.hot = const [],
    this.following = const [],
    this.hotError,
    this.followingError,
  }) : super(client: ApiClient(baseUrl: 'http://test'));

  List<Room> hot;
  List<Room> following;
  Object? hotError;
  Object? followingError;
  int feedHotCalls = 0;
  int feedFollowingCalls = 0;

  @override
  Future<List<Room>> feedHot() async {
    feedHotCalls++;
    if (hotError != null) throw hotError!;
    return hot;
  }

  @override
  Future<List<Room>> feedFollowing() async {
    feedFollowingCalls++;
    if (followingError != null) throw followingError!;
    return following;
  }
}

void main() {
  const config = AppConfig(
    apiBaseUrl: 'http://localhost:8088',
    environment: 'local',
  );

  testWidgets('feed page shows hot rooms and tabs', (tester) async {
    final fake = FakeSocialRepo(
      hot: [
        Room(id: 'r1', ownerId: 'u1', title: 'Hot Live', status: 'live'),
      ],
      following: [
        Room(id: 'r2', ownerId: 'u2', title: 'Friend Live', status: 'live'),
      ],
    );

    await tester.pumpWidget(
      MaterialApp(
        home: FeedPage(
          config: config,
          accessToken: 'tok',
          socialRepository: fake,
        ),
      ),
    );

    // Loading uses FeedSkeleton placeholders (not CircularProgressIndicator).
    expect(find.byType(FeedSkeleton), findsWidgets);
    await tester.pumpAndSettle();

    expect(find.text('Discover'), findsOneWidget);
    expect(find.text('Hot'), findsOneWidget);
    expect(find.text('Following'), findsOneWidget);
    expect(find.text('Hot Live'), findsOneWidget);
    expect(fake.feedHotCalls, greaterThan(0));
    expect(fake.feedFollowingCalls, greaterThan(0));

    await tester.tap(find.text('Following'));
    await tester.pumpAndSettle();
    expect(find.text('Friend Live'), findsOneWidget);
  });

  testWidgets('feed page shows empty state', (tester) async {
    final fake = FakeSocialRepo(hot: const [], following: const []);

    await tester.pumpWidget(
      MaterialApp(
        home: FeedPage(
          config: config,
          accessToken: 'tok',
          socialRepository: fake,
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('No hot rooms'), findsOneWidget);
  });

  testWidgets('home shows MainShell tabs when logged in', (tester) async {
    await tester.pumpWidget(
      const MaterialApp(
        home: HomePage(
          config: config,
          sessionLabel: 'Ada',
          accessToken: 'tok',
        ),
      ),
    );
    await tester.pump();
    expect(find.text('Home'), findsWidgets);
    expect(find.text('Following'), findsOneWidget);
    expect(find.text('Go Live'), findsOneWidget);
    expect(find.text('You'), findsOneWidget);
  });
}
