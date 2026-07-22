import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'hls_player_logic.dart';

/// Scaffolding preview for an HLS live stream.
///
/// Does **not** embed a real video player (no `video_player` plugin) so CI and
/// unit tests stay free of native deps. When live + URL: black aspect-ratio box
/// with play icon, selectable URL, copy button, and external-open note.
/// When ended: a compact ended banner.
class StreamPreview extends StatelessWidget {
  const StreamPreview({
    super.key,
    required this.status,
    this.hlsUrl,
  });

  final String status;
  final String? hlsUrl;

  bool get _ended =>
      status == 'closed' || status == 'idle' || status == 'ended';

  void _copyUrl(BuildContext context) {
    final url = hlsUrl?.trim() ?? '';
    if (url.isEmpty) return;
    // Fire-and-forget clipboard; do not await (plugin may hang under flutter_test).
    // ignore: unawaited_futures
    Clipboard.setData(ClipboardData(text: url));
    final messenger = ScaffoldMessenger.maybeOf(context);
    messenger?.showSnackBar(
      const SnackBar(
        key: Key('stream-url-copied-snackbar'),
        content: Text('Copied - open in VLC / browser player'),
        duration: Duration(seconds: 4),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    if (_ended) {
      return Material(
        color: theme.colorScheme.errorContainer,
        child: const Padding(
          padding: EdgeInsets.symmetric(horizontal: 16, vertical: 12),
          child: Text('Room ended'),
        ),
      );
    }

    if (!shouldShowPlayer(status, hlsUrl)) {
      return Padding(
        padding: const EdgeInsets.symmetric(vertical: 4),
        child: Text(
          playerPlaceholderMessage(status, hlsUrl),
          style: theme.textTheme.bodyMedium,
        ),
      );
    }

    final url = hlsUrl!.trim();
    // Fixed preview height keeps room chrome usable under short viewports
    // (flutter_test default 800x600) while still reading as a 16:9 stage.
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        const SizedBox(
          width: double.infinity,
          height: 160,
          child: DecoratedBox(
            decoration: BoxDecoration(color: Colors.black),
            child: Center(
              child: Icon(
                Icons.play_circle_outline,
                size: 56,
                color: Colors.white70,
                key: Key('stream-preview-play-icon'),
              ),
            ),
          ),
        ),
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
              Text('HLS stream', style: theme.textTheme.labelLarge),
              const SizedBox(height: 4),
              SelectableText(
                url,
                style: theme.textTheme.bodySmall,
              ),
              const SizedBox(height: 4),
              Text(
                'Open externally (in-app player not embedded)',
                style: theme.textTheme.bodySmall?.copyWith(
                  color: theme.colorScheme.onSurfaceVariant,
                ),
              ),
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
}
