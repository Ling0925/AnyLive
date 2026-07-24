/// Conditional web HLS player (HTML video + hls.js).
///
/// On web: [web_hls_player_web.dart]
/// Elsewhere: no-op stub so VM tests never touch `dart:html` / `package:web`.
library;

export 'web_hls_player_stub.dart'
    if (dart.library.js_interop) 'web_hls_player_web.dart';
