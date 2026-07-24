import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:media_kit/media_kit.dart';
import 'package:media_kit_video/media_kit_video.dart';

import 'flutter_test_env.dart';
import 'hls_player_logic.dart';
import 'web_hls_player.dart';
import '../l10n/l10n.dart';

/// HLS live preview for a room.
///
/// Playback backends:
/// - **web**: HTML `<video>` + hls.js ([WebHlsPlayerController]) — avoids
///   media_kit's desktop-Chrome "no supported source" path for m3u8.
/// - **native**: [media_kit] + `media_kit_libs_video`.
///
/// Under `flutter test` the default is false so CI never loads player backends;
/// pass `enableEmbeddedPlayer: false` from widget tests explicitly as belt-and-braces.
///
/// Copy-URL remains available so dogfood can open VLC if decode fails.
class StreamPreview extends StatefulWidget {
  const StreamPreview({
    super.key,
    required this.status,
    this.hlsUrl,
    this.enableEmbeddedPlayer,
  });

  final String status;
  final String? hlsUrl;

  /// When null, uses [embeddedPlayerEnabled] (true unless under flutter_test).
  final bool? enableEmbeddedPlayer;

  /// Global default: enable player on device + web; disable under `flutter test`.
  static bool get embeddedPlayerEnabled {
    if (isFlutterTestProcess) return false;
    const fromDefine =
        bool.fromEnvironment('FLUTTER_TEST', defaultValue: false);
    if (fromDefine) return false;
    return true;
  }

  @override
  State<StreamPreview> createState() => _StreamPreviewState();
}

class _StreamPreviewState extends State<StreamPreview> {
  // --- media_kit (native) ---
  Player? _player;
  VideoController? _controller;
  static bool _mediaKitReady = false;
  final List<StreamSubscription<dynamic>> _subs = [];

  // --- web hls.js ---
  WebHlsPlayerController? _webPlayer;
  StreamSubscription<void>? _webSub;

  String? _openedUrl;
  String? _playerError;
  bool _playerReady = false;
  bool _buffering = false;
  bool _playing = false;
  int? _videoW;
  int? _videoH;

  bool get _terminal => isRoomTerminalStatus(widget.status);

  bool get _useEmbedded {
    return widget.enableEmbeddedPlayer ?? StreamPreview.embeddedPlayerEnabled;
  }

  /// Prefer dedicated web HLS path on browsers.
  bool get _useWebHls => kIsWeb && isWebHlsPlayerSupported;

  String _playableUrl(String raw) {
    // Keep loopback host consistent with Flutter web page (usually 127.0.0.1).
    return normalizePlaybackUrl(raw, preferHost: '127.0.0.1');
  }

  @override
  void initState() {
    super.initState();
    if (_useEmbedded && shouldShowPlayer(widget.status, widget.hlsUrl)) {
      _ensurePlayer(widget.hlsUrl!.trim());
    }
  }

  @override
  void didUpdateWidget(covariant StreamPreview oldWidget) {
    super.didUpdateWidget(oldWidget);
    final url = widget.hlsUrl?.trim();
    final show = shouldShowPlayer(widget.status, widget.hlsUrl);
    if (!_useEmbedded) {
      _disposePlayer();
      return;
    }
    if (!show) {
      _disposePlayer();
      return;
    }
    if (url != null && url.isNotEmpty) {
      final next = _playableUrl(url);
      if (next != _openedUrl) {
        _ensurePlayer(url);
      }
    }
  }

  void _clearSubs() {
    for (final s in _subs) {
      // ignore: discarded_futures
      s.cancel();
    }
    _subs.clear();
    // ignore: discarded_futures
    _webSub?.cancel();
    _webSub = null;
  }

  void _wirePlayer(Player player) {
    _clearSubs();
    _subs.add(player.stream.error.listen((e) {
      final msg = e.toString().trim();
      if (msg.isEmpty || !mounted) return;
      setState(() {
        _playerError = msg;
        _playerReady = false;
      });
    }));
    _subs.add(player.stream.buffering.listen((b) {
      if (!mounted) return;
      setState(() => _buffering = b);
    }));
    _subs.add(player.stream.playing.listen((p) {
      if (!mounted) return;
      setState(() => _playing = p);
    }));
    _subs.add(player.stream.width.listen((w) {
      if (!mounted || w == null) return;
      setState(() => _videoW = w);
    }));
    _subs.add(player.stream.height.listen((h) {
      if (!mounted || h == null) return;
      setState(() => _videoH = h);
    }));
  }

  void _syncFromWeb() {
    final w = _webPlayer;
    if (w == null || !mounted) return;
    setState(() {
      _playerError = w.error;
      _playerReady = w.ready;
      _playing = w.playing;
      _buffering = !w.ready && w.error == null;
    });
  }

  void _ensurePlayer(String url) {
    final playUrl = _playableUrl(url);
    if (_useWebHls) {
      _ensureWebPlayer(playUrl);
      return;
    }
    _ensureMediaKitPlayer(playUrl);
  }

  void _ensureWebPlayer(String playUrl) {
    _webPlayer ??= WebHlsPlayerController();
    // ignore: discarded_futures
    _webSub?.cancel();
    _webSub = _webPlayer!.changes.listen((_) => _syncFromWeb());
    _openedUrl = playUrl;
    _playerReady = false;
    _playerError = null;
    _buffering = true;
    _playing = false;
    if (mounted) setState(() {});

    // ignore: discarded_futures
    _webPlayer!.open(playUrl, muted: true).then((_) {
      if (mounted) _syncFromWeb();
    }).catchError((Object e) {
      if (mounted) {
        setState(() {
          _playerError = 'Playback failed: $e';
          _playerReady = false;
          _buffering = false;
        });
      }
    });
  }

  void _ensureMediaKitPlayer(String playUrl) {
    try {
      if (!_mediaKitReady) {
        MediaKit.ensureInitialized();
        _mediaKitReady = true;
      }
    } catch (e) {
      if (mounted) {
        setState(() {
          _playerError = 'Player init failed: $e';
        });
      }
      return;
    }

    final muted = kIsWeb;
    _player ??= Player(
      configuration: PlayerConfiguration(
        muted: muted,
        title: 'AnyLive',
      ),
    );
    _controller ??= VideoController(_player!);
    _wirePlayer(_player!);
    _openedUrl = playUrl;
    _playerReady = false;
    _playerError = null;
    _buffering = true;
    _videoW = null;
    _videoH = null;
    if (mounted) setState(() {});

    // ignore: discarded_futures
    _player!
        .open(Media(playUrl), play: true)
        .then((_) async {
          try {
            await _player!.play();
            if (kIsWeb) {
              await _player!.setVolume(0);
            }
          } catch (_) {}
          if (mounted) {
            setState(() {
              _playerReady = true;
              _playerError = null;
            });
          }
        })
        .catchError((Object e) {
          if (mounted) {
            setState(() {
              _playerError = 'Playback failed: $e';
              _playerReady = false;
              _buffering = false;
            });
          }
        });
  }

  Future<void> _userPlay() async {
    if (_useWebHls) {
      final w = _webPlayer;
      if (w == null) return;
      try {
        await w.play();
        if (mounted) _syncFromWeb();
      } catch (e) {
        if (mounted) {
          setState(() => _playerError = 'Play failed: $e');
        }
      }
      return;
    }
    final p = _player;
    if (p == null) return;
    try {
      await p.play();
      if (mounted) setState(() => _playerError = null);
    } catch (e) {
      if (mounted) {
        setState(() => _playerError = 'Play failed: $e');
      }
    }
  }

  void _disposePlayer() {
    _clearSubs();
    final p = _player;
    _player = null;
    _controller = null;
    final w = _webPlayer;
    _webPlayer = null;
    _openedUrl = null;
    _playerReady = false;
    _buffering = false;
    _playing = false;
    _videoW = null;
    _videoH = null;
    _playerError = null;
    p?.dispose();
    w?.dispose();
  }

  @override
  void dispose() {
    _disposePlayer();
    super.dispose();
  }

  void _copyUrl(BuildContext context) {
    final url = widget.hlsUrl?.trim() ?? '';
    if (url.isEmpty) return;
    // ignore: unawaited_futures
    Clipboard.setData(ClipboardData(text: _playableUrl(url)));
    final messenger = ScaffoldMessenger.maybeOf(context);
    messenger?.showSnackBar(
      SnackBar(
        key: Key('stream-url-copied-snackbar'),
        content: Text(context.l10n.copiedStreamUrl),
        duration: Duration(seconds: 4),
      ),
    );
  }


  String _localizedPlaceholder(BuildContext context) {
    final l10n = context.l10n;
    final status = widget.status;
    if (isRoomTerminalStatus(status)) return l10n.streamEnded;
    if (status == 'idle') return l10n.hostOffline;
    if (status != 'live') return l10n.hostOffline;
    final url = widget.hlsUrl?.trim() ?? '';
    if (url.isEmpty) return l10n.livePlayUrlUnavailable;
    return l10n.openStreamExternal;
  }

  String? _localizedSubline(BuildContext context) {
    final l10n = context.l10n;
    final status = widget.status;
    if (isRoomTerminalStatus(status)) return l10n.roomForceClosed;
    if (status == 'idle') return l10n.hostStoppedMayReturn;
    return null;
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    if (_terminal) {
      final msg = _localizedPlaceholder(context);
      final sub = _localizedSubline(context);
      return Material(
        color: theme.colorScheme.errorContainer,
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 16),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                msg,
                key: const Key('stream-ended'),
                style: theme.textTheme.titleMedium?.copyWith(
                  fontWeight: FontWeight.w600,
                ),
              ),
              if (sub != null) ...[
                const SizedBox(height: 4),
                Text(sub, style: theme.textTheme.bodySmall),
              ],
            ],
          ),
        ),
      );
    }

    if (!shouldShowPlayer(widget.status, widget.hlsUrl)) {
      final msg = _localizedPlaceholder(context);
      final sub = _localizedSubline(context);
      final offline = isRoomOfflineStatus(widget.status);
      return Material(
        color: offline
            ? theme.colorScheme.surfaceContainerHighest
            : Colors.transparent,
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 16),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                msg,
                key: Key(offline ? 'stream-offline' : 'stream-placeholder'),
                style: theme.textTheme.titleMedium?.copyWith(
                  fontWeight: FontWeight.w600,
                ),
              ),
              if (sub != null) ...[
                const SizedBox(height: 4),
                Text(sub, style: theme.textTheme.bodySmall),
              ],
            ],
          ),
        ),
      );
    }

    final url = _playableUrl(widget.hlsUrl!.trim());
    final hasFrame = (_videoW ?? 0) > 0 && (_videoH ?? 0) > 0;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        SizedBox(
          width: double.infinity,
          height: 240,
          child: DecoratedBox(
            decoration: const BoxDecoration(color: Colors.black),
            child: _buildStage(theme, hasFrame: hasFrame),
          ),
        ),
        if (_playerError != null) ...[
          const SizedBox(height: 8),
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 8),
            child: Text(
              key: const Key('stream-player-error'),
              _playerError!,
              style: theme.textTheme.bodySmall?.copyWith(
                color: theme.colorScheme.error,
              ),
            ),
          ),
        ],
        SizedBox(height: 8),
        Container(
          width: double.infinity,
          padding: const EdgeInsets.all(12),
          decoration: BoxDecoration(
            color: theme.colorScheme.surfaceContainerHighest,
            borderRadius: BorderRadius.circular(8),
          ),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                _useEmbedded
                    ? (_useWebHls
                        ? context.l10n.hlsBrowserHlsJs
                        : (kIsWeb
                            ? context.l10n.hlsBrowserMuted
                            : context.l10n.hlsInApp))
                    : context.l10n.hlsStream,
                style: theme.textTheme.labelLarge,
              ),
              SizedBox(height: 4),
              SelectableText(
                url,
                key: const Key('stream-hls-url'),
                style: theme.textTheme.bodySmall,
              ),
              if (!_useEmbedded) ...[
                SizedBox(height: 4),
                Text(
                  context.l10n.playerDisabledCopyUrl,
                  style: theme.textTheme.bodySmall?.copyWith(
                    color: theme.colorScheme.onSurfaceVariant,
                  ),
                ),
              ],
              if (_useEmbedded && kIsWeb) ...[
                SizedBox(height: 4),
                Text(
                  context.l10n.browserAutoplayMuted,
                  style: theme.textTheme.bodySmall?.copyWith(
                    color: theme.colorScheme.onSurfaceVariant,
                  ),
                ),
              ],
              SizedBox(height: 8),
              Wrap(
                spacing: 8,
                runSpacing: 8,
                children: [
                  OutlinedButton.icon(
                    key: const Key('copy-stream-url'),
                    onPressed: () => _copyUrl(context),
                    icon: const Icon(Icons.copy, size: 16),
                    label: Text(context.l10n.copyStreamUrl),
                  ),
                  if (_useEmbedded)
                    OutlinedButton.icon(
                      key: const Key('stream-retry-play'),
                      onPressed: _userPlay,
                      icon: const Icon(Icons.play_arrow, size: 16),
                      label: Text(context.l10n.playRetry),
                    ),
                ],
              ),
            ],
          ),
        ),
      ],
    );
  }

  Widget _buildStage(ThemeData theme, {required bool hasFrame}) {
    if (_useEmbedded && _playerError == null) {
      final Widget? videoChild;
      if (_useWebHls && _webPlayer != null) {
        videoChild = buildWebHlsView(_webPlayer!);
      } else if (_controller != null) {
        videoChild = Video(
          key: const Key('stream-preview-video'),
          controller: _controller!,
          controls: AdaptiveVideoControls,
          fill: Colors.black,
        );
      } else {
        videoChild = null;
      }

      if (videoChild != null) {
        // Web backends sometimes never report width/height; once playing, drop
        // the overlay so the HTML <video> surface is not covered forever.
        final showOverlay = !_playerReady ||
            _buffering ||
            (!_playing && !hasFrame && !_useWebHls);
        // For web hls path: hide overlay once playing or ready.
        final webOverlay = _useWebHls && (!_playing && !_playerReady);
        final overlay = _useWebHls ? webOverlay : showOverlay;

        return Stack(
          fit: StackFit.expand,
          children: [
            KeyedSubtree(
              key: const Key('stream-preview-video'),
              child: videoChild,
            ),
            if (overlay)
              ColoredBox(
                color: Colors.black54,
                child: Center(
                  child: Column(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      CircularProgressIndicator(
                        key: Key('stream-preview-loading'),
                        color: Colors.white54,
                      ),
                      SizedBox(height: 10),
                      Text(
                        !_playerReady
                            ? context.l10n.openingStream
                            : _buffering
                                ? context.l10n.buffering
                                : context.l10n.waitingForVideo,
                        style:
                            TextStyle(color: Colors.white70, fontSize: 13),
                      ),
                      if (!_playing) ...[
                        SizedBox(height: 10),
                        FilledButton.tonal(
                          key: const Key('stream-tap-to-play'),
                          onPressed: _userPlay,
                          child: Text(context.l10n.tapToPlay),
                        ),
                      ],
                    ],
                  ),
                ),
              ),
          ],
        );
      }
    }
    return const Center(
      child: Icon(
        Icons.play_circle_outline,
        size: 56,
        color: Colors.white70,
        key: Key('stream-preview-play-icon'),
      ),
    );
  }
}
