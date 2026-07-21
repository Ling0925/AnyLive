import 'package:flutter/material.dart';

import '../../api/api_client.dart';
import '../../api/auth_repository.dart';
import '../../config/app_config.dart';
import '../home/home_page.dart';

class LoginPage extends StatefulWidget {
  const LoginPage({super.key, required this.config});

  final AppConfig config;

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

  @override
  void initState() {
    super.initState();
    _api = ApiClient(baseUrl: widget.config.normalizedApiBaseUrl);
    _auth = AuthRepository(client: _api);
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
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      final session = await _auth.verifyOtp(
        email: _email.text.trim(),
        code: _code.text.trim(),
      );
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
            if (_error != null)
              Text(_error!, style: TextStyle(color: Theme.of(context).colorScheme.error)),
            const SizedBox(height: 12),
            FilledButton(
              onPressed: _busy
                  ? null
                  : () {
                      if (_otpSent) {
                        _verify();
                      } else {
                        _sendOtp();
                      }
                    },
              child: Text(_busy
                  ? 'Please wait…'
                  : (_otpSent ? 'Verify & continue' : 'Send OTP')),
            ),
          ],
        ),
      ),
    );
  }
}
