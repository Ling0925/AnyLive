import 'flutter_test_env_stub.dart'
    if (dart.library.io) 'flutter_test_env_io.dart' as env;

/// Shared env probes for player / platform gates (web-safe).
bool get isFlutterTestProcess => env.isFlutterTestProcess;
