import 'package:flutter/widgets.dart';

/// Non-web stub — embedded web HLS is unavailable.
class WebHlsPlayerController {
  WebHlsPlayerController();

  String? get error => null;
  bool get playing => false;
  bool get ready => false;

  Stream<void> get changes => const Stream.empty();

  Future<void> open(String url, {bool muted = true}) async {}

  Future<void> play() async {}

  Future<void> setMuted(bool muted) async {}

  void dispose() {}
}

/// Always returns a placeholder; real view is web-only.
Widget buildWebHlsView(WebHlsPlayerController controller) {
  return const SizedBox.expand();
}

bool get isWebHlsPlayerSupported => false;
