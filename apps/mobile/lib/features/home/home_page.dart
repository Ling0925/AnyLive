import 'package:flutter/material.dart';

import '../../config/app_config.dart';
import '../auth/login_page.dart';
import '../feed/feed_page.dart';
import '../profile/profile_page.dart';
import '../rooms/room_list_page.dart';

class HomePage extends StatefulWidget {
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
  State<HomePage> createState() => _HomePageState();
}

class _HomePageState extends State<HomePage> {
  late String? _sessionLabel = widget.sessionLabel;

  @override
  Widget build(BuildContext context) {
    final loggedIn =
        widget.accessToken != null && widget.accessToken!.isNotEmpty;
    return Scaffold(
      appBar: AppBar(
        title: const Text('AnyLive'),
        actions: [
          if (loggedIn)
            TextButton(
              onPressed: () async {
                await Navigator.of(context).push(
                  MaterialPageRoute(
                    builder: (_) => ProfilePage(
                      config: widget.config,
                      accessToken: widget.accessToken!,
                      onDisplayNameChanged: (name) {
                        setState(() => _sessionLabel = name);
                      },
                    ),
                  ),
                );
              },
              child: const Text('Profile'),
            )
          else
            TextButton(
              onPressed: () {
                Navigator.of(context).push(
                  MaterialPageRoute(
                    builder: (_) => LoginPage(config: widget.config),
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
              Text('env: ${widget.config.environment}'),
              Text('api: ${widget.config.normalizedApiBaseUrl}'),
              if (_sessionLabel != null) ...[
                const SizedBox(height: 12),
                Text('signed in as $_sessionLabel'),
              ],
              const SizedBox(height: 24),
              if (loggedIn) ...[
                FilledButton(
                  onPressed: () {
                    Navigator.of(context).push(
                      MaterialPageRoute(
                        builder: (_) => RoomListPage(
                          config: widget.config,
                          accessToken: widget.accessToken!,
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
                          accessToken: widget.accessToken!,
                        ),
                      ),
                    );
                  },
                  child: const Text('Discover'),
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
