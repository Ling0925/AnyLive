import 'dart:convert';

import 'package:shared_preferences/shared_preferences.dart';

import 'auth_repository.dart';

/// Persist / restore OTP session across app restarts (P1 dogfood UX).
///
/// Tokens only — never stores password. [savedAtMs] is wall-clock millis when
/// the last access token was written, so restore can decide whether to refresh.
class SessionStore {
  SessionStore({SharedPreferences? prefs}) : _prefs = prefs;

  SharedPreferences? _prefs;

  static const _key = 'anylive_session_v1';

  Future<SharedPreferences> _sp() async {
    return _prefs ??= await SharedPreferences.getInstance();
  }

  Future<void> save(AuthSession session) async {
    final sp = await _sp();
    await sp.setString(
      _key,
      jsonEncode({
        'user_id': session.userId,
        'display_name': session.displayName,
        'email': session.email,
        'username': session.username,
        'access_token': session.accessToken,
        'refresh_token': session.refreshToken,
        'expires_in': session.expiresIn,
        'must_change_password': session.mustChangePassword,
        'saved_at_ms': DateTime.now().millisecondsSinceEpoch,
      }),
    );
  }

  Future<AuthSession?> load() async {
    final sp = await _sp();
    final raw = sp.getString(_key);
    if (raw == null || raw.isEmpty) return null;
    try {
      final map = jsonDecode(raw) as Map<String, dynamic>;
      final access = map['access_token'] as String? ?? '';
      if (access.isEmpty) return null;
      return AuthSession(
        userId: map['user_id'] as String? ?? '',
        displayName: map['display_name'] as String? ?? '',
        email: map['email'] as String?,
        username: map['username'] as String?,
        accessToken: access,
        refreshToken: map['refresh_token'] as String? ?? '',
        expiresIn: (map['expires_in'] as num?)?.toInt() ?? 0,
        mustChangePassword: map['must_change_password'] as bool? ?? false,
      );
    } catch (_) {
      return null;
    }
  }

  /// Epoch ms when [save] last wrote, or null if missing / unreadable.
  Future<int?> savedAtMs() async {
    final sp = await _sp();
    final raw = sp.getString(_key);
    if (raw == null || raw.isEmpty) return null;
    try {
      final map = jsonDecode(raw) as Map<String, dynamic>;
      return (map['saved_at_ms'] as num?)?.toInt();
    } catch (_) {
      return null;
    }
  }

  /// True when access is likely past half of [expiresIn] (or past 10m default).
  Future<bool> accessLikelyStale({
    AuthSession? session,
    DateTime? now,
  }) async {
    final s = session ?? await load();
    if (s == null) return true;
    final saved = await savedAtMs();
    if (saved == null) return false;
    final ttl = s.expiresIn > 0 ? s.expiresIn : 600;
    // Refresh when ≥ half the lifetime has elapsed (or 5 min min window).
    final skew = ttl < 120 ? (ttl ~/ 2) : (ttl ~/ 2).clamp(60, ttl);
    final ageSec =
        ((now ?? DateTime.now()).millisecondsSinceEpoch - saved) ~/ 1000;
    return ageSec >= skew;
  }

  Future<void> clear() async {
    final sp = await _sp();
    await sp.remove(_key);
  }
}
