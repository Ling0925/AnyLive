import 'package:flutter_test/flutter_test.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:anylive_mobile/api/auth_repository.dart';
import 'package:anylive_mobile/api/session_store.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  setUp(() {
    SharedPreferences.setMockInitialValues({});
  });

  test('session store save/load/clear', () async {
    final store = SessionStore();
    final session = AuthSession(
      userId: 'u1',
      displayName: 'Ada',
      email: 'ada@example.com',
      accessToken: 'access-tok',
      refreshToken: 'refresh-tok',
      expiresIn: 3600,
    );
    await store.save(session);
    final loaded = await store.load();
    expect(loaded, isNotNull);
    expect(loaded!.accessToken, 'access-tok');
    expect(loaded.displayName, 'Ada');
    expect(loaded.email, 'ada@example.com');
    expect(loaded.refreshToken, 'refresh-tok');
    expect(await store.savedAtMs(), isNotNull);

    await store.clear();
    expect(await store.load(), isNull);
  });

  test('session store returns null for empty token', () async {
    SharedPreferences.setMockInitialValues({
      'anylive_session_v1':
          '{"user_id":"u","display_name":"x","access_token":"","refresh_token":"","expires_in":0}',
    });
    final store = SessionStore();
    expect(await store.load(), isNull);
  });

  test('accessLikelyStale uses saved_at_ms and expires_in', () async {
    final nowMs = DateTime.now().millisecondsSinceEpoch;
    SharedPreferences.setMockInitialValues({
      'anylive_session_v1':
          '{"user_id":"u1","display_name":"Ada","email":"a@b.com",'
          '"access_token":"tok","refresh_token":"r","expires_in":600,'
          '"saved_at_ms":${nowMs - 400000}}',
    });
    final store = SessionStore();
    expect(await store.accessLikelyStale(), isTrue);

    SharedPreferences.setMockInitialValues({
      'anylive_session_v1':
          '{"user_id":"u1","display_name":"Ada","email":"a@b.com",'
          '"access_token":"tok","refresh_token":"r","expires_in":600,'
          '"saved_at_ms":$nowMs}',
    });
    final store2 = SessionStore();
    expect(await store2.accessLikelyStale(), isFalse);
  });
}
