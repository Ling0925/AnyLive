import 'package:flutter/material.dart';

import '../api/session_store.dart';
import '../config/app_config.dart';
import '../features/auth/login_page.dart';
import '../features/feed/feed_page.dart';
import '../features/go_live/go_live_page.dart';
import '../features/you/you_page.dart';
import '../l10n/l10n.dart';
import '../theme/any_colors.dart';

/// Bottom-nav shell: Home | Following | Go Live | You (YouTube-style IA).
///
/// Owns login gate when [accessToken] is null/empty; when logged in shows
/// four-tab [IndexedStack]. Room pages are pushed full-screen (no bottom nav).
class MainShell extends StatefulWidget {
  const MainShell({
    super.key,
    required this.config,
    this.sessionLabel,
    this.accessToken,
    this.userId,
    this.sessionStore,
    this.onLogout,
    this.onSessionRestored,
    this.onDisplayNameChanged,
    this.onAccountDeleted,
    this.initialIndex = 0,
  });

  final AppConfig config;
  final String? sessionLabel;
  final String? accessToken;
  final String? userId;
  final SessionStore? sessionStore;
  final Future<void> Function()? onLogout;
  final void Function(String token, String label)? onSessionRestored;
  final void Function(String name)? onDisplayNameChanged;
  final Future<void> Function()? onAccountDeleted;
  final int initialIndex;

  @override
  State<MainShell> createState() => _MainShellState();
}

class _MainShellState extends State<MainShell> {
  late int _index = widget.initialIndex.clamp(0, 3);
  late String? _sessionLabel = widget.sessionLabel;
  late String? _accessToken = widget.accessToken;
  late String? _userId = widget.userId;

  bool get _loggedIn =>
      _accessToken != null && _accessToken!.isNotEmpty;

  @override
  void initState() {
    super.initState();
    _hydrateUserId();
  }

  @override
  void didUpdateWidget(covariant MainShell oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.accessToken != widget.accessToken) {
      _accessToken = widget.accessToken;
      _hydrateUserId();
    }
    if (oldWidget.sessionLabel != widget.sessionLabel) {
      _sessionLabel = widget.sessionLabel;
    }
    if (oldWidget.userId != widget.userId && widget.userId != null) {
      _userId = widget.userId;
    }
  }

  Future<void> _hydrateUserId() async {
    if (widget.userId != null && widget.userId!.isNotEmpty) {
      if (mounted) setState(() => _userId = widget.userId);
      return;
    }
    final store = widget.sessionStore;
    if (store == null || !_loggedIn) {
      if (mounted) setState(() => _userId = null);
      return;
    }
    final session = await store.load();
    if (!mounted) return;
    setState(() => _userId = session?.userId);
  }

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

  Future<void> _handleLogout() async {
    await widget.onLogout?.call();
    if (!mounted) return;
    setState(() {
      _accessToken = null;
      _sessionLabel = null;
      _userId = null;
      _index = 0;
    });
  }

  void _switchToHome() => setState(() => _index = 0);

  @override
  Widget build(BuildContext context) {
    final l10n = context.l10n;

    if (!_loggedIn) {
      return Scaffold(
        backgroundColor: AnyColors.bg,
        body: SafeArea(
          child: Center(
            child: Padding(
              padding: const EdgeInsets.all(28),
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Container(
                    width: 64,
                    height: 64,
                    decoration: BoxDecoration(
                      borderRadius: BorderRadius.circular(16),
                      gradient: const LinearGradient(
                        colors: [AnyColors.accent, Color(0xFF8B5CF6)],
                      ),
                    ),
                    alignment: Alignment.center,
                    child: const Text(
                      'AL',
                      style: TextStyle(
                        fontWeight: FontWeight.w800,
                        fontSize: 22,
                        color: Colors.white,
                      ),
                    ),
                  ),
                  const SizedBox(height: 20),
                  Text(
                    l10n.appTitle,
                    style: const TextStyle(
                      fontSize: 24,
                      fontWeight: FontWeight.w700,
                      color: AnyColors.textPrimary,
                    ),
                  ),
                  const SizedBox(height: 8),
                  Text(
                    l10n.appTagline,
                    textAlign: TextAlign.center,
                    style: const TextStyle(color: AnyColors.textSecondary),
                  ),
                  const SizedBox(height: 12),
                  Text(
                    l10n.envFlavorLine(
                      widget.config.environment,
                      widget.config.flavorLabel,
                    ),
                    style: const TextStyle(
                      color: AnyColors.textMuted,
                      fontSize: 12,
                    ),
                  ),
                  Text(
                    widget.config.normalizedApiBaseUrl,
                    style: const TextStyle(
                      color: AnyColors.textMuted,
                      fontSize: 12,
                    ),
                  ),
                  const SizedBox(height: 28),
                  FilledButton(
                    key: const Key('shell-login'),
                    onPressed: _openLogin,
                    child: Text(l10n.signIn),
                  ),
                ],
              ),
            ),
          ),
        ),
      );
    }

    final token = _accessToken!;
    final pages = <Widget>[
      FeedPage(
        key: const ValueKey('tab-home'),
        config: widget.config,
        accessToken: token,
        userId: _userId,
        mode: FeedMode.home,
      ),
      FeedPage(
        key: const ValueKey('tab-following'),
        config: widget.config,
        accessToken: token,
        userId: _userId,
        mode: FeedMode.following,
        onJumpHome: _switchToHome,
      ),
      GoLivePage(
        key: const ValueKey('tab-golive'),
        config: widget.config,
        accessToken: token,
        userId: _userId,
      ),
      YouPage(
        key: const ValueKey('tab-you'),
        config: widget.config,
        accessToken: token,
        sessionLabel: _sessionLabel,
        sessionStore: widget.sessionStore,
        onLogout: _handleLogout,
        onDisplayNameChanged: (name) {
          setState(() => _sessionLabel = name);
          widget.onDisplayNameChanged?.call(name);
        },
        onAccountDeleted: () async {
          await widget.onAccountDeleted?.call();
          await _handleLogout();
        },
      ),
    ];

    return Scaffold(
      backgroundColor: AnyColors.bg,
      body: IndexedStack(index: _index, children: pages),
      bottomNavigationBar: NavigationBar(
        height: 64,
        backgroundColor: AnyColors.elevated,
        indicatorColor: AnyColors.accentSoft,
        selectedIndex: _index,
        onDestinationSelected: (i) => setState(() => _index = i),
        labelBehavior: NavigationDestinationLabelBehavior.alwaysShow,
        destinations: [
          NavigationDestination(
            icon: const Icon(Icons.home_outlined),
            selectedIcon: const Icon(Icons.home),
            label: l10n.navHome,
          ),
          NavigationDestination(
            icon: const Icon(Icons.subscriptions_outlined),
            selectedIcon: const Icon(Icons.subscriptions),
            label: l10n.navFollowing,
          ),
          NavigationDestination(
            icon: const Icon(Icons.videocam_outlined),
            selectedIcon: const Icon(Icons.videocam),
            label: l10n.navGoLive,
          ),
          NavigationDestination(
            icon: const Icon(Icons.person_outline),
            selectedIcon: const Icon(Icons.person),
            label: l10n.navYou,
          ),
        ],
      ),
    );
  }
}
