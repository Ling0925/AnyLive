import 'package:flutter/material.dart';
import 'package:media_kit/media_kit.dart';

import 'app.dart';
import 'config/app_config.dart';

void main() {
  WidgetsFlutterBinding.ensureInitialized();
  // Native (media_kit_libs_*) + web (HTML video + hls.js) backends.
  MediaKit.ensureInitialized();
  runApp(AnyLiveApp(config: AppConfig.fromEnvironment()));
}
