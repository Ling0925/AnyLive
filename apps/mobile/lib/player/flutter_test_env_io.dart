import 'dart:io' show Platform;

/// Whether the process is a `flutter test` harness.
///
/// `flutter test` injects `FLUTTER_TEST=true` into the process environment.
bool get isFlutterTestProcess =>
    Platform.environment['FLUTTER_TEST'] == 'true';
