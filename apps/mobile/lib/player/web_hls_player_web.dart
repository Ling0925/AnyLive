import 'dart:async';
import 'dart:js_interop';
import 'dart:js_interop_unsafe';
import 'dart:ui_web' as ui_web;

import 'package:flutter/widgets.dart';
import 'package:web/web.dart' as web;

/// Web-only HLS controller: HTML `<video>` + global `Hls` (hls.js).
///
/// media_kit's web backend often falls through to `video.src = m3u8` on
/// desktop Chrome (no native HLS) when hls.js is not ready, which yields
/// "Failed to load because no supported source was found." This path loads
/// hls.js from `/hls.min.js` (copied into `web/`) and always uses MSE.
class WebHlsPlayerController {
  WebHlsPlayerController() {
    _viewType = 'anylive-hls-${_nextId++}';
    _video = web.HTMLVideoElement()
      ..autoplay = true
      ..muted = true
      ..controls = true
      ..setAttribute('playsinline', 'true')
      ..setAttribute('webkit-playsinline', 'true')
      ..style.width = '100%'
      ..style.height = '100%'
      ..style.objectFit = 'contain'
      ..style.backgroundColor = '#000';

    _video.addEventListener(
      'error',
      ((web.Event _) {
        final mediaError = _video.error;
        final code = mediaError?.code ?? 0;
        final msg = mediaError?.message.trim() ?? '';
        _error = msg.isNotEmpty ? msg : 'Video error (code $code)';
        _ready = false;
        _playing = false;
        _notify();
      }).toJS,
    );
    _video.addEventListener(
      'playing',
      ((web.Event _) {
        _playing = true;
        _ready = true;
        _error = null;
        _notify();
      }).toJS,
    );
    _video.addEventListener(
      'pause',
      ((web.Event _) {
        _playing = false;
        _notify();
      }).toJS,
    );
    _video.addEventListener(
      'waiting',
      ((web.Event _) {
        _ready = false;
        _notify();
      }).toJS,
    );
    _video.addEventListener(
      'canplay',
      ((web.Event _) {
        _ready = true;
        _error = null;
        _notify();
      }).toJS,
    );

    ui_web.platformViewRegistry.registerViewFactory(
      _viewType,
      (int viewId) => _video,
    );
  }

  static int _nextId = 0;

  late final String _viewType;
  late final web.HTMLVideoElement _video;
  HlsJs? _hls;
  String? _openedUrl;
  String? _error;
  bool _playing = false;
  bool _ready = false;
  bool _disposed = false;
  final _changeController = StreamController<void>.broadcast();

  String get viewType => _viewType;
  String? get error => _error;
  bool get playing => _playing;
  bool get ready => _ready;
  Stream<void> get changes => _changeController.stream;

  void _notify() {
    if (!_changeController.isClosed) {
      _changeController.add(null);
    }
  }

  Future<void> open(String url, {bool muted = true}) async {
    if (_disposed) return;
    final playUrl = url.trim();
    if (playUrl.isEmpty) return;
    if (playUrl == _openedUrl && _error == null) {
      await play();
      return;
    }

    _openedUrl = playUrl;
    _error = null;
    _ready = false;
    _playing = false;
    _notify();

    try {
      await ensureHlsJsLoaded();
    } catch (e) {
      _error = 'hls.js load failed: $e';
      _notify();
      return;
    }

    _destroyHls();
    _video.muted = muted;
    _video.removeAttribute('src');
    _video.load();

    try {
      if (isHlsJsSupported()) {
        final hls = createHlsJs();
        _hls = hls;
        // hls.js event name for fatal/non-fatal errors.
        hls.on(
          'hlsError',
          ((JSAny event, JSAny data) {
            try {
              final map = data.dartify();
              if (map is Map && map['fatal'] != true) return;
              final detailStr = map is Map
                  ? (map['details']?.toString() ?? '')
                  : '';
              _error = detailStr.isEmpty
                  ? 'HLS playback error'
                  : 'HLS error: $detailStr';
              _ready = false;
              _playing = false;
              _notify();
            } catch (_) {
              _error = 'HLS playback error';
              _ready = false;
              _playing = false;
              _notify();
            }
          }).toJS,
        );
        hls.loadSource(playUrl);
        hls.attachMedia(_video);
      } else if (_video.canPlayType('application/vnd.apple.mpegurl').isNotEmpty) {
        _video.src = playUrl;
      } else {
        _error = 'HLS not supported in this browser';
        _notify();
        return;
      }
    } catch (e) {
      _error = 'Open failed: $e';
      _notify();
      return;
    }

    await play();
  }

  Future<void> play() async {
    if (_disposed) return;
    try {
      await _video.play().toDart;
      _playing = true;
      _error = null;
      _notify();
    } catch (e) {
      _error = 'Play failed (tap Play / retry): $e';
      _playing = false;
      _notify();
    }
  }

  Future<void> setMuted(bool muted) async {
    if (_disposed) return;
    _video.muted = muted;
    if (muted) {
      _video.volume = 0;
    } else if (_video.volume == 0) {
      _video.volume = 1;
    }
  }

  void _destroyHls() {
    final hls = _hls;
    _hls = null;
    if (hls != null) {
      try {
        hls.destroy();
      } catch (_) {}
    }
  }

  void dispose() {
    if (_disposed) return;
    _disposed = true;
    _destroyHls();
    try {
      _video.pause();
      _video.removeAttribute('src');
      _video.load();
    } catch (_) {}
    _changeController.close();
  }
}

Widget buildWebHlsView(WebHlsPlayerController controller) {
  return HtmlElementView(
    key: ValueKey(controller.viewType),
    viewType: controller.viewType,
  );
}

bool get isWebHlsPlayerSupported => true;

// --- hls.js global bindings -------------------------------------------------

@JS('Hls')
@staticInterop
class HlsJs {
  external factory HlsJs();
}

extension HlsJsExt on HlsJs {
  external void loadSource(String src);
  external void attachMedia(web.HTMLVideoElement media);
  external void destroy();
  external void on(String event, JSFunction callback);
}

@JS('Hls.isSupported')
external bool _hlsIsSupported();

HlsJs createHlsJs() => HlsJs();

bool isHlsJsSupported() {
  try {
    if (!_hlsGlobalPresent()) return false;
    return _hlsIsSupported();
  } catch (_) {
    return false;
  }
}

bool _hlsGlobalPresent() {
  try {
    final hls = globalContext.getProperty('Hls'.toJS);
    return hls != null && !hls.isUndefinedOrNull;
  } catch (_) {
    return false;
  }
}

Completer<void>? _hlsLoadCompleter;
bool _hlsScriptRequested = false;

/// Load `/hls.min.js` once (file lives in `apps/mobile/web/hls.min.js`).
Future<void> ensureHlsJsLoaded() async {
  if (_hlsGlobalPresent()) return;
  if (_hlsLoadCompleter != null) return _hlsLoadCompleter!.future;

  final completer = Completer<void>();
  _hlsLoadCompleter = completer;

  if (!_hlsScriptRequested) {
    _hlsScriptRequested = true;
    final script = web.HTMLScriptElement()
      ..async = true
      ..type = 'text/javascript'
      ..src = 'hls.min.js';
    script.addEventListener(
      'load',
      ((web.Event _) {
        if (!completer.isCompleted) completer.complete();
      }).toJS,
    );
    script.addEventListener(
      'error',
      ((web.Event _) {
        if (!completer.isCompleted) {
          completer.completeError(StateError('Failed to load hls.min.js'));
        }
      }).toJS,
    );
    final head = web.document.head;
    if (head != null) {
      head.append(script);
    } else {
      web.document.documentElement!.append(script);
    }
  }

  if (_hlsGlobalPresent() && !completer.isCompleted) {
    completer.complete();
  }

  return completer.future.timeout(
    const Duration(seconds: 12),
    onTimeout: () {
      throw TimeoutException('hls.js load timeout');
    },
  );
}
