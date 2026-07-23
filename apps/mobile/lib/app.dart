import 'package:flutter/material.dart';

import 'api/session_store.dart';
import 'config/app_config.dart';
import 'features/home/home_page.dart';
import 'navigation/app_routes.dart';
import 'theme/any_theme.dart';

class AnyLiveApp extends StatefulWidget {
  const AnyLiveApp({
    super.key,
    required this.config,
    this.sessionStore,
  });

  final AppConfig config;

  /// Injectable for tests; when null a real [SessionStore] is created.
  final SessionStore? sessionStore;

  @override
  State<AnyLiveApp> createState() => _AnyLiveAppState();
}

class _AnyLiveAppState extends State<AnyLiveApp> {
  late final SessionStore _sessions =
      widget.sessionStore ?? SessionStore();
  bool _loading = true;
  String? _accessToken;
  String? _sessionLabel;

  @override
  void initState() {
    super.initState();
    _restore();
  }

  Future<void> _restore() async {
    final session = await _sessions.load();
    if (!mounted) return;
    setState(() {
      if (session != null) {
        _accessToken = session.accessToken;
        _sessionLabel = session.displayName.isEmpty
            ? (session.email ?? session.userId)
            : session.displayName;
      }
      _loading = false;
    });
  }

  Future<void> _logout() async {
    await _sessions.clear();
    if (!mounted) return;
    setState(() {
      _accessToken = null;
      _sessionLabel = null;
    });
  }

  @override
  Widget build(BuildContext context) {
    final flavor = widget.config.flavorLabel;
    return MaterialApp(
      // OS task switcher title encodes flavor (WBS E8.1); UI chrome stays "AnyLive".
      title: flavor == 'local' ? 'AnyLive' : 'AnyLive ($flavor)',
      debugShowCheckedModeBanner: widget.config.isLocal,
      theme: anyDarkTheme(),
      // [AppRoutes.home] documents the shell root for a future go_router map.
      // HomePage is a thin MainShell shim (preserves login / test entry).
      home: _loading
          ? const Scaffold(
              body: Center(child: CircularProgressIndicator()),
            )
          : HomePage(
              key: const ValueKey(AppRoutes.home),
              config: widget.config,
              sessionLabel: _sessionLabel,
              accessToken: _accessToken,
              sessionStore: _sessions,
              onLogout: _logout,
              onSessionRestored: (token, label) {
                setState(() {
                  _accessToken = token;
                  _sessionLabel = label;
                });
              },
            ),
    );
  }
}
