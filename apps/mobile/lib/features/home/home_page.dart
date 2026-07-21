import 'package:flutter/material.dart';

import '../../config/app_config.dart';
import '../auth/login_page.dart';

class HomePage extends StatelessWidget {
  const HomePage({
    super.key,
    required this.config,
    this.sessionLabel,
    this.accessToken,
  });

  final AppConfig config;
  final String? sessionLabel;
  final String? accessToken;

  @override
  Widget build(BuildContext context) {
    final loggedIn = accessToken != null && accessToken!.isNotEmpty;
    return Scaffold(
      appBar: AppBar(
        title: const Text('AnyLive'),
        actions: [
          if (!loggedIn)
            TextButton(
              onPressed: () {
                Navigator.of(context).push(
                  MaterialPageRoute(
                    builder: (_) => LoginPage(config: config),
                  ),
                );
              },
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
              Text('env: ${config.environment}'),
              Text('api: ${config.normalizedApiBaseUrl}'),
              if (sessionLabel != null) ...[
                const SizedBox(height: 12),
                Text('signed in as $sessionLabel'),
              ],
              const SizedBox(height: 24),
              Text(
                loggedIn
                    ? 'P1 control plane ready — rooms/chat/gifts via API.'
                    : 'Sign in with email OTP (dev code 123456).',
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
