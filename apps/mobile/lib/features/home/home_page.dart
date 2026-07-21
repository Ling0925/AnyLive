import 'package:flutter/material.dart';

import '../../config/app_config.dart';

class HomePage extends StatelessWidget {
  const HomePage({super.key, required this.config});

  final AppConfig config;

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('AnyLive')),
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
              const SizedBox(height: 24),
              Text(
                'P0 shell — auth, rooms, player come next.',
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
