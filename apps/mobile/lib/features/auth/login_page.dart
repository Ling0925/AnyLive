import 'package:flutter/material.dart';

import '../../api/api_client.dart';
import '../../api/auth_repository.dart';
import '../../api/profile_repository.dart';
import '../../config/app_config.dart';
import '../home/home_page.dart';

class LoginPage extends StatefulWidget {
  const LoginPage({
    super.key,
    required this.config,
    this.authRepository,
    this.profileRepositoryFactory,
  });

  final AppConfig config;

  /// Injectable for tests; when null a real [AuthRepository] is created.
  final AuthRepository? authRepository;

  /// Builds a [ProfileRepository] after OTP verify (token already on [ApiClient]).
  /// Injectable so tests can assert the best-effort age/privacy PATCH.
  final ProfileRepository Function(ApiClient client)? profileRepositoryFactory;

  @override
  State<LoginPage> createState() => _LoginPageState();
}

class _LoginPageState extends State<LoginPage> {
  final _email = TextEditingController();
  final _code = TextEditingController(text: '123456');
  late final ApiClient _api;
  late final AuthRepository _auth;
  String? _error;
  bool _busy = false;
  bool _otpSent = false;
  bool _ageConfirmed = false;
  bool _privacyAccepted = false;

  @override
  void initState() {
    super.initState();
    _api = ApiClient(baseUrl: widget.config.normalizedApiBaseUrl);
    _auth = widget.authRepository ?? AuthRepository(client: _api);
  }

  @override
  void dispose() {
    _email.dispose();
    _code.dispose();
    super.dispose();
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

  Future<void> _verify() async {
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

      // Best-effort compliance PATCH — never block navigation on failure.
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
        } catch (_) {
          // Ignore — user can re-confirm on profile page.
        }
      }

      if (!mounted) return;
      Navigator.of(context).pushReplacement(
        MaterialPageRoute(
          builder: (_) => HomePage(
            config: widget.config,
            sessionLabel: session.displayName.isEmpty
                ? session.email ?? session.userId
                : session.displayName,
            accessToken: session.accessToken,
          ),
        ),
      );
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
    final canVerify = _otpSent && _ageConfirmed && !_busy;
    final canSend = !_otpSent && !_busy;

    return Scaffold(
      appBar: AppBar(title: const Text('AnyLive Login')),
      body: Padding(
        padding: const EdgeInsets.all(24),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
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
            // MVP age declaration — required before Verify.
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
              Text(_error!, style: TextStyle(color: Theme.of(context).colorScheme.error)),
            const SizedBox(height: 12),
            FilledButton(
              onPressed: _otpSent
                  ? (canVerify ? _verify : null)
                  : (canSend ? _sendOtp : null),
              child: Text(_busy
                  ? 'Please wait…'
                  : (_otpSent ? 'Verify & continue' : 'Send OTP')),
            ),
            const Spacer(),
            // P1 compliance: show legal URLs (no url_launcher dep yet).
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
