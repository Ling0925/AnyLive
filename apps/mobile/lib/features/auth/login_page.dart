import 'dart:async';

import 'package:flutter/material.dart';

import '../../api/api_client.dart';
import '../../api/auth_repository.dart';
import '../../api/events_repository.dart';
import '../../api/profile_repository.dart';
import '../../api/session_store.dart';
import '../../config/app_config.dart';
import '../home/home_page.dart';

class LoginPage extends StatefulWidget {
  const LoginPage({
    super.key,
    required this.config,
    this.authRepository,
    this.profileRepositoryFactory,
    this.sessionStore,
    this.onLoggedIn,
  });

  final AppConfig config;

  /// Injectable for tests; when null a real [AuthRepository] is created.
  final AuthRepository? authRepository;

  /// Builds a [ProfileRepository] after login (token already on [ApiClient]).
  final ProfileRepository Function(ApiClient client)? profileRepositoryFactory;

  final SessionStore? sessionStore;

  /// When set, called after successful login instead of pushReplacement alone.
  final void Function(AuthSession session)? onLoggedIn;

  @override
  State<LoginPage> createState() => _LoginPageState();
}

class _LoginPageState extends State<LoginPage> {
  final _identifier = TextEditingController();
  final _password = TextEditingController();
  final _email = TextEditingController();
  final _code = TextEditingController(text: '123456');
  late final ApiClient _api;
  late final AuthRepository _auth;
  String? _error;
  bool _busy = false;
  bool _otpSent = false;
  bool _showDevOtp = false;
  bool _ageConfirmed = false;
  bool _privacyAccepted = false;
  bool _obscurePassword = true;

  @override
  void initState() {
    super.initState();
    _api = ApiClient(baseUrl: widget.config.normalizedApiBaseUrl);
    _auth = widget.authRepository ?? AuthRepository(client: _api);
  }

  @override
  void dispose() {
    _identifier.dispose();
    _password.dispose();
    _email.dispose();
    _code.dispose();
    super.dispose();
  }

  Future<void> _completeLogin(AuthSession session, {required String method}) async {
    if (_ageConfirmed || _privacyAccepted) {
      try {
        final profileClient = ApiClient(
          baseUrl: widget.config.normalizedApiBaseUrl,
          accessToken: session.accessToken,
        );
        final profile = widget.profileRepositoryFactory != null
            ? widget.profileRepositoryFactory!(profileClient)
            : ProfileRepository(client: profileClient);
        await profile.patchMe(
          ageConfirmed: _ageConfirmed ? true : null,
          privacyAccepted: _privacyAccepted ? true : null,
        );
      } catch (_) {}
    }

    try {
      await widget.sessionStore?.save(session);
    } catch (_) {}

    try {
      final eventsClient = ApiClient(
        baseUrl: widget.config.normalizedApiBaseUrl,
        accessToken: session.accessToken,
      );
      unawaited(
        EventsRepository(client: eventsClient).track(
          'auth.login',
          props: {'method': method},
        ),
      );
    } catch (_) {}

    if (!mounted) return;
    if (widget.onLoggedIn != null) {
      widget.onLoggedIn!(session);
      Navigator.of(context).pop();
    } else {
      Navigator.of(context).pushReplacement(
        MaterialPageRoute(
          builder: (_) => HomePage(
            config: widget.config,
            sessionLabel: session.displayName.isEmpty
                ? (session.email ?? session.username ?? session.userId)
                : session.displayName,
            accessToken: session.accessToken,
            sessionStore: widget.sessionStore,
          ),
        ),
      );
    }
  }

  Future<void> _passwordLogin() async {
    if (!_ageConfirmed) return;
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      final session = await _auth.passwordLogin(
        identifier: _identifier.text.trim(),
        password: _password.text,
      );
      await _completeLogin(session, method: 'password');
    } on AuthException catch (e) {
      setState(() => _error = e.toString());
    } catch (e) {
      setState(() => _error = e.toString());
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _sendOtp() async {
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      await _auth.sendOtp(_email.text.trim());
      setState(() => _otpSent = true);
    } on AuthException catch (e) {
      setState(() => _error = e.toString());
    } catch (e) {
      setState(() => _error = e.toString());
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _verifyOtp() async {
    if (!_ageConfirmed) return;
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      final session = await _auth.verifyOtp(
        email: _email.text.trim(),
        code: _code.text.trim(),
      );
      await _completeLogin(session, method: 'otp');
    } on AuthException catch (e) {
      setState(() => _error = e.toString());
    } catch (e) {
      setState(() => _error = e.toString());
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final canPasswordLogin = !_busy &&
        _ageConfirmed &&
        _identifier.text.trim().isNotEmpty &&
        _password.text.isNotEmpty;
    final canVerifyOtp = _otpSent && _ageConfirmed && !_busy;
    final canSendOtp = !_otpSent && !_busy;

    return Scaffold(
      appBar: AppBar(title: const Text('AnyLive Login')),
      body: Padding(
        padding: const EdgeInsets.all(24),
        child: ListView(
          children: [
            TextField(
              controller: _identifier,
              keyboardType: TextInputType.emailAddress,
              autocorrect: false,
              onChanged: (_) => setState(() {}),
              decoration: const InputDecoration(
                labelText: 'Email or username',
                border: OutlineInputBorder(),
              ),
            ),
            const SizedBox(height: 12),
            TextField(
              controller: _password,
              obscureText: _obscurePassword,
              onChanged: (_) => setState(() {}),
              onSubmitted: (_) {
                if (canPasswordLogin) _passwordLogin();
              },
              decoration: InputDecoration(
                labelText: 'Password',
                border: const OutlineInputBorder(),
                suffixIcon: IconButton(
                  icon: Icon(
                    _obscurePassword ? Icons.visibility : Icons.visibility_off,
                  ),
                  onPressed: () =>
                      setState(() => _obscurePassword = !_obscurePassword),
                ),
              ),
            ),
            const SizedBox(height: 12),
            CheckboxListTile(
              contentPadding: EdgeInsets.zero,
              title: const Text('I confirm I am 18 or older'),
              value: _ageConfirmed,
              onChanged: _busy
                  ? null
                  : (v) => setState(() => _ageConfirmed = v ?? false),
              controlAffinity: ListTileControlAffinity.leading,
            ),
            CheckboxListTile(
              contentPadding: EdgeInsets.zero,
              title: const Text('I accept the privacy policy'),
              value: _privacyAccepted,
              onChanged: _busy
                  ? null
                  : (v) => setState(() => _privacyAccepted = v ?? false),
              controlAffinity: ListTileControlAffinity.leading,
            ),
            if (_error != null)
              Text(
                _error!,
                style: TextStyle(color: Theme.of(context).colorScheme.error),
              ),
            const SizedBox(height: 12),
            FilledButton(
              onPressed: canPasswordLogin ? _passwordLogin : null,
              child: Text(_busy && !_showDevOtp ? 'Please wait…' : 'Sign in'),
            ),
            const SizedBox(height: 16),
            ExpansionTile(
              title: const Text('Dev OTP (local only)'),
              initiallyExpanded: _showDevOtp,
              onExpansionChanged: (v) => setState(() => _showDevOtp = v),
              children: [
                TextField(
                  controller: _email,
                  keyboardType: TextInputType.emailAddress,
                  decoration: const InputDecoration(
                    labelText: 'Email',
                    border: OutlineInputBorder(),
                  ),
                ),
                const SizedBox(height: 12),
                if (_otpSent) ...[
                  TextField(
                    controller: _code,
                    decoration: const InputDecoration(
                      labelText: 'OTP (dev: 123456)',
                      border: OutlineInputBorder(),
                    ),
                  ),
                  const SizedBox(height: 12),
                ],
                FilledButton.tonal(
                  onPressed: _otpSent
                      ? (canVerifyOtp ? _verifyOtp : null)
                      : (canSendOtp ? _sendOtp : null),
                  child: Text(
                    _busy && _showDevOtp
                        ? 'Please wait…'
                        : (_otpSent ? 'Verify & continue' : 'Send OTP'),
                  ),
                ),
                const SizedBox(height: 8),
              ],
            ),
            const SizedBox(height: 24),
            Text(
              'Privacy Policy',
              style: Theme.of(context).textTheme.labelLarge,
              textAlign: TextAlign.center,
            ),
            const Text(
              'https://anylive.example/privacy',
              textAlign: TextAlign.center,
            ),
            const SizedBox(height: 8),
            Text(
              'Terms of Service',
              style: Theme.of(context).textTheme.labelLarge,
              textAlign: TextAlign.center,
            ),
            const Text(
              'https://anylive.example/terms',
              textAlign: TextAlign.center,
            ),
          ],
        ),
      ),
    );
  }
}
