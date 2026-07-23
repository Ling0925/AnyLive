import 'dart:io' show Platform;

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:media_kit/media_kit.dart';
import 'package:media_kit_video/media_kit_video.dart';

import 'hls_player_logic.dart';

/// HLS live preview for a room.
///
/// Embeds [media_kit] when [enableEmbeddedPlayer] is true. Under `flutter test`
/// the default is false so CI never loads native player libs; pass
/// `enableEmbeddedPlayer: false` from widget tests explicitly as a belt-and-braces.
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

  /// Global default: disable native player under `flutter test`.
  static bool get embeddedPlayerEnabled {
    if (kIsWeb) return false;
    // flutter_test injects FLUTTER_TEST=true into the process environment.
    if (Platform.environment['FLUTTER_TEST'] == 'true') return false;
    // Compile-time define also works when tests set --dart-define.
    const fromDefine = bool.fromEnvironment('FLUTTER_TEST', defaultValue: false);
    if (fromDefine) return false;
    return true;
  }

  @override
  State<StreamPreview> createState() => _StreamPreviewState();
}

class _StreamPreviewState extends State<StreamPreview> {
  Player? _player;
  VideoController? _controller;
  String? _openedUrl;
  String? _playerError;
  bool _playerReady = false;
  static bool _mediaKitReady = false;

  /// Permanent end only (closed/ended). Idle is offline — host can re-start.
  bool get _terminal => isRoomTerminalStatus(widget.status);

  bool get _useEmbedded {
    return widget.enableEmbeddedPlayer ?? StreamPreview.embeddedPlayerEnabled;
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
    if (url != null && url.isNotEmpty && url != _openedUrl) {
      _ensurePlayer(url);
    }
  }

  void _ensurePlayer(String url) {
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
    _player ??= Player();
    _controller ??= VideoController(_player!);
    _openedUrl = url;
    _playerReady = false;
    _player!
        .open(Media(url), play: true)
        .then((_) {
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
            });
          }
        });
  }

  void _disposePlayer() {
    final p = _player;
    _player = null;
    _controller = null;
    _openedUrl = null;
    _playerReady = false;
    p?.dispose();
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
    Clipboard.setData(ClipboardData(text: url));
    final messenger = ScaffoldMessenger.maybeOf(context);
    messenger?.showSnackBar(
      const SnackBar(
        key: Key('stream-url-copied-snackbar'),
        content: Text('Copied stream URL'),
        duration: Duration(seconds: 4),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    if (_terminal) {
      return Material(
        color: theme.colorScheme.errorContainer,
        child: const Padding(
          padding: EdgeInsets.symmetric(horizontal: 16, vertical: 12),
          child: Text('Room ended'),
        ),
      );
    }

    if (!shouldShowPlayer(widget.status, widget.hlsUrl)) {
      final msg = playerPlaceholderMessage(widget.status, widget.hlsUrl);
      final offline = isRoomOfflineStatus(widget.status);
      return Material(
        color: offline
            ? theme.colorScheme.surfaceContainerHighest
            : Colors.transparent,
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
          child: Text(
            msg,
            key: Key(offline ? 'stream-offline' : 'stream-placeholder'),
            style: theme.textTheme.bodyMedium,
          ),
        ),
      );
    }

    final url = widget.hlsUrl!.trim();

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        SizedBox(
          width: double.infinity,
          height: 200,
          child: DecoratedBox(
            decoration: const BoxDecoration(color: Colors.black),
            child: _buildStage(theme),
          ),
        ),
        if (_playerError != null) ...[
          const SizedBox(height: 8),
          Text(
            _playerError!,
            style: theme.textTheme.bodySmall?.copyWith(
              color: theme.colorScheme.error,
            ),
          ),
        ],
        const SizedBox(height: 8),
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
                _useEmbedded ? 'HLS (in-app)' : 'HLS stream',
                style: theme.textTheme.labelLarge,
              ),
              const SizedBox(height: 4),
              SelectableText(url, style: theme.textTheme.bodySmall),
              if (!_useEmbedded) ...[
                const SizedBox(height: 4),
                Text(
                  'Open externally (in-app player disabled in tests)',
                  style: theme.textTheme.bodySmall?.copyWith(
                    color: theme.colorScheme.onSurfaceVariant,
                  ),
                ),
              ],
              const SizedBox(height: 8),
              OutlinedButton.icon(
                key: const Key('copy-stream-url'),
                onPressed: () => _copyUrl(context),
                icon: const Icon(Icons.copy, size: 16),
                label: const Text('Copy stream URL'),
              ),
            ],
          ),
        ),
      ],
    );
  }

  Widget _buildStage(ThemeData theme) {
    if (_useEmbedded && _controller != null && _playerError == null) {
      return Stack(
        fit: StackFit.expand,
        children: [
          Video(
            key: const Key('stream-preview-video'),
            controller: _controller!,
            controls: NoVideoControls,
            fill: Colors.black,
          ),
          if (!_playerReady)
            const Center(
              child: CircularProgressIndicator(
                key: Key('stream-preview-loading'),
                color: Colors.white54,
              ),
            ),
        ],
      );
    }
    // Scaffold / fallback when embedded off or init failed.
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
