import 'dart:async';

import 'package:flutter/material.dart';

import '../../api/session_store.dart';
import '../../config/app_config.dart';
import '../../navigation/main_shell.dart';

/// Root entry shim — always embeds [MainShell] so login + 4-tab IA stay in one place.
///
/// Kept as a compatibility type so login / existing tests keep working.
class HomePage extends StatefulWidget {
  const HomePage({
    super.key,
    required this.config,
    this.sessionLabel,
    this.accessToken,
    this.sessionStore,
    this.onLogout,
    this.onSessionRestored,
  });

  final AppConfig config;
  final String? sessionLabel;
  final String? accessToken;
  final SessionStore? sessionStore;
  final Future<void> Function()? onLogout;
  final void Function(String token, String label)? onSessionRestored;

  @override
  State<HomePage> createState() => _HomePageState();
}

class _HomePageState extends State<HomePage> {
  late String? _sessionLabel = widget.sessionLabel;
  late String? _accessToken = widget.accessToken;
  String? _userId;

  @override
  void initState() {
    super.initState();
    unawaited(_hydrateUserId());
  }

  @override
  void didUpdateWidget(covariant HomePage oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.accessToken != widget.accessToken) {
      _accessToken = widget.accessToken;
      unawaited(_hydrateUserId());
    }
    if (oldWidget.sessionLabel != widget.sessionLabel) {
      _sessionLabel = widget.sessionLabel;
    }
  }

  Future<void> _hydrateUserId() async {
    final store = widget.sessionStore;
    if (store == null ||
        _accessToken == null ||
        _accessToken!.isEmpty) {
      if (mounted) setState(() => _userId = null);
      return;
    }
    final session = await store.load();
    if (!mounted) return;
    setState(() => _userId = session?.userId);
  }

  Future<void> _handleLogout() async {
    await widget.onLogout?.call();
    if (!mounted) return;
    setState(() {
      _accessToken = null;
      _sessionLabel = null;
      _userId = null;
    });
  }

  @override
  Widget build(BuildContext context) {
    return MainShell(
      config: widget.config,
      accessToken: _accessToken,
      userId: _userId,
      sessionLabel: _sessionLabel,
      sessionStore: widget.sessionStore,
      onLogout: _handleLogout,
      onSessionRestored: (token, label) {
        setState(() {
          _accessToken = token;
          _sessionLabel = label;
        });
        widget.onSessionRestored?.call(token, label);
        unawaited(_hydrateUserId());
      },
      onDisplayNameChanged: (name) {
        setState(() => _sessionLabel = name);
      },
      onAccountDeleted: _handleLogout,
    );
  }
}
