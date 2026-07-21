import 'package:flutter/material.dart';

import 'config/app_config.dart';
import 'features/home/home_page.dart';

class AnyLiveApp extends StatelessWidget {
  const AnyLiveApp({super.key, required this.config});

  final AppConfig config;

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'AnyLive',
      debugShowCheckedModeBanner: config.isLocal,
      theme: ThemeData(
        colorScheme: ColorScheme.fromSeed(
          seedColor: const Color(0xFF6C5CE7),
          brightness: Brightness.dark,
        ),
        useMaterial3: true,
      ),
      home: HomePage(config: config),
    );
  }
}
