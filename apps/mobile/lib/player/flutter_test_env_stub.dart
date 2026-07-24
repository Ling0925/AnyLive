/// Whether the process is a `flutter test` harness (VM only).
///
/// Web stub always returns false — browsers never set FLUTTER_TEST.
bool get isFlutterTestProcess => false;
