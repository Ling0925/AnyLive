import 'package:flutter/material.dart';

import 'app.dart';
import 'config/app_config.dart';

void main() {
  WidgetsFlutterBinding.ensureInitialized();
  runApp(AnyLiveApp(config: AppConfig.fromEnvironment()));
}
