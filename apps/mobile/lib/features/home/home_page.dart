import 'dart:async';

import 'package:flutter/material.dart';

import '../../api/session_store.dart';
import '../../config/app_config.dart';
import '../auth/login_page.dart';
import '../feed/feed_page.dart';
import '../profile/profile_page.dart';
import '../rooms/room_list_page.dart';
import '../wallet/wallet_page.dart';

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
    if (store == null || !_loggedIn) {
      if (mounted) setState(() => _userId = null);
      return;
    }
    final session = await store.load();
    if (!mounted) return;
    setState(() => _userId = session?.userId);
  }

  bool get _loggedIn =>
      _accessToken != null && _accessToken!.isNotEmpty;

  Future<void> _openLogin() async {
    await Navigator.of(context).push(
      MaterialPageRoute(
        builder: (_) => LoginPage(
          config: widget.config,
          sessionStore: widget.sessionStore,
          onLoggedIn: (session) {
            final label = session.displayName.isEmpty
                ? (session.email ?? session.userId)
                : session.displayName;
            setState(() {
              _accessToken = session.accessToken;
              _sessionLabel = label;
              _userId = session.userId;
            });
            widget.onSessionRestored?.call(session.accessToken, label);
          },
        ),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('AnyLive'),
        actions: [
          if (_loggedIn) ...[
            TextButton(
              onPressed: () async {
                await Navigator.of(context).push(
                  MaterialPageRoute(
                    builder: (_) => ProfilePage(
                      config: widget.config,
                      accessToken: _accessToken!,
                      sessionStore: widget.sessionStore,
                      onDisplayNameChanged: (name) {
                        setState(() => _sessionLabel = name);
                      },
                      onAccountDeleted: () async {
                        await widget.onLogout?.call();
                        if (!mounted) return;
                        setState(() {
                          _accessToken = null;
                          _sessionLabel = null;
                          _userId = null;
                        });
                      },
                    ),
                  ),
                );
              },
              child: const Text('Profile'),
            ),
            TextButton(
              key: const Key('home-logout'),
              onPressed: () async {
                await widget.onLogout?.call();
                if (!mounted) return;
                setState(() {
                  _accessToken = null;
                  _sessionLabel = null;
                  _userId = null;
                });
              },
              child: const Text('Logout'),
            ),
          ] else
            TextButton(
              onPressed: _openLogin,
              child: const Text('Login'),
            ),
        ],
      ),
      body: Center(
        child: Padding(
          padding: const EdgeInsets.all(24),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Text(
                'AnyLive Mobile',
                style: Theme.of(context).textTheme.headlineMedium,
              ),
              const SizedBox(height: 12),
              Text('env: ${widget.config.environment}'),
              Text('flavor: ${widget.config.flavorLabel}'),
              Text('api: ${widget.config.normalizedApiBaseUrl}'),
              if (_sessionLabel != null) ...[
                const SizedBox(height: 12),
                Text('signed in as $_sessionLabel'),
              ],
              const SizedBox(height: 24),
              if (_loggedIn) ...[
                FilledButton(
                  onPressed: () {
                    Navigator.of(context).push(
                      MaterialPageRoute(
                        builder: (_) => RoomListPage(
                          config: widget.config,
                          accessToken: _accessToken!,
                          userId: _userId,
                        ),
                      ),
                    );
                  },
                  child: const Text('Browse live rooms'),
                ),
                const SizedBox(height: 12),
                FilledButton.tonal(
                  onPressed: () {
                    Navigator.of(context).push(
                      MaterialPageRoute(
                        builder: (_) => FeedPage(
                          config: widget.config,
                          accessToken: _accessToken!,
                          userId: _userId,
                        ),
                      ),
                    );
                  },
                  child: const Text('Discover'),
                ),
                const SizedBox(height: 12),
                FilledButton.tonal(
                  key: const Key('home-wallet'),
                  onPressed: () {
                    Navigator.of(context).push(
                      MaterialPageRoute(
                        builder: (_) => WalletPage(
                          config: widget.config,
                          accessToken: _accessToken!,
                        ),
                      ),
                    );
                  },
                  child: const Text('Wallet'),
                ),
              ] else
                Text(
                  'Sign in with email OTP (dev code 123456).',
                  style: Theme.of(context).textTheme.bodyMedium,
                  textAlign: TextAlign.center,
                ),
            ],
          ),
        ),
      ),
    );
  }
}
