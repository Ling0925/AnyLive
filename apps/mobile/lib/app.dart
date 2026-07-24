import 'package:flutter/material.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:http/http.dart' as http;

import 'api/api_client.dart';
import 'api/auth_repository.dart';
import 'api/session_store.dart';
import 'config/app_config.dart';
import 'features/home/home_page.dart';
import 'l10n/l10n.dart';
import 'l10n/locale_controller.dart';
import 'l10n/locale_scope.dart';
import 'navigation/app_routes.dart';
import 'theme/any_theme.dart';

class AnyLiveApp extends StatefulWidget {
  const AnyLiveApp({
    super.key,
    required this.config,
    this.sessionStore,
    this.httpClient,
    this.authRepositoryFactory,
    this.localeController,
  });

  final AppConfig config;

  /// Injectable for tests; when null a real [SessionStore] is created.
  final SessionStore? sessionStore;

  /// Injectable HTTP client (tests / custom stacks).
  final http.Client? httpClient;

  /// Build [AuthRepository] for restore/logout. Defaults to real HTTP.
  final AuthRepository Function(ApiClient client)? authRepositoryFactory;

  /// Injectable locale controller (tests pass pre-loaded English instance).
  final LocaleController? localeController;

  @override
  State<AnyLiveApp> createState() => _AnyLiveAppState();
}

class _AnyLiveAppState extends State<AnyLiveApp> {
  late final SessionStore _sessions =
      widget.sessionStore ?? SessionStore();
  late final LocaleController _locale =
      widget.localeController ?? LocaleController();
  bool _loading = true;
  String? _accessToken;
  String? _sessionLabel;

  AuthRepository _auth(ApiClient client) {
    final factory = widget.authRepositoryFactory;
    if (factory != null) return factory(client);
    return AuthRepository(
      client: client,
      httpClient: widget.httpClient,
    );
  }

  @override
  void initState() {
    super.initState();
    _bootstrap();
  }

  Future<void> _bootstrap() async {
    // Load locale first so the first painted frame is already Chinese (default).
    if (!_locale.isLoaded) {
      await _locale.load();
    }
    await _restore();
  }

  Future<void> _restore() async {
    final session = await _sessions.load();
    if (!mounted) return;
    if (session == null) {
      setState(() {
        _accessToken = null;
        _sessionLabel = null;
        _loading = false;
      });
      return;
    }

    final api = ApiClient(
      baseUrl: widget.config.normalizedApiBaseUrl,
      accessToken: session.accessToken,
    );
    final auth = _auth(api);

    AuthSession? active = session;
    try {
      final ok = await auth.validateAccess();
      if (!ok) {
        active = await _tryRefresh(auth, session);
      } else {
        final stale = await _sessions.accessLikelyStale(session: session);
        if (stale && session.refreshToken.isNotEmpty) {
          try {
            active = await _tryRefresh(auth, session) ?? session;
          } catch (_) {
            // Keep validated access if proactive refresh fails.
            active = session;
          }
        }
      }
    } catch (_) {
      active = await _tryRefresh(auth, session);
    }

    if (!mounted) return;
    if (active == null) {
      await _sessions.clear();
      setState(() {
        _accessToken = null;
        _sessionLabel = null;
        _loading = false;
      });
      return;
    }

    setState(() {
      _accessToken = active!.accessToken;
      _sessionLabel = active.displayName.isEmpty
          ? (active.email ?? active.userId)
          : active.displayName;
      _loading = false;
    });
  }

  Future<AuthSession?> _tryRefresh(
    AuthRepository auth,
    AuthSession previous,
  ) async {
    final rt = previous.refreshToken.trim();
    if (rt.isEmpty) return null;
    try {
      final next = await auth.refresh(
        refreshToken: rt,
        previous: previous,
      );
      await _sessions.save(next);
      return next;
    } catch (_) {
      return null;
    }
  }

  Future<void> _logout() async {
    final session = await _sessions.load();
    if (session != null) {
      final api = ApiClient(
        baseUrl: widget.config.normalizedApiBaseUrl,
        accessToken: session.accessToken,
      );
      final auth = _auth(api);
      try {
        await auth.logout(refreshToken: session.refreshToken);
      } catch (_) {
        // Best-effort server revoke; always clear local.
      }
    }
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
    return ListenableBuilder(
      listenable: _locale,
      builder: (context, _) {
        final locale = _locale.effectiveLocale;
        return MaterialApp(
          // OS task switcher title encodes flavor (WBS E8.1); UI chrome stays "AnyLive".
          onGenerateTitle: (ctx) {
            final l10n = AppLocalizations.of(ctx);
            return flavor == 'local'
                ? l10n.appTitle
                : l10n.appTitleFlavor(flavor);
          },
          locale: locale,
          supportedLocales: AppLocalizations.supportedLocales,
          localizationsDelegates: const [
            AppLocalizations.delegate,
            GlobalMaterialLocalizations.delegate,
            GlobalWidgetsLocalizations.delegate,
            GlobalCupertinoLocalizations.delegate,
          ],
          debugShowCheckedModeBanner: widget.config.isLocal,
          theme: anyDarkTheme(),
          builder: (context, child) {
            return LocaleScope(
              controller: _locale,
              child: child ?? const SizedBox.shrink(),
            );
          },
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
      },
    );
  }
}
