import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../../api/api_client.dart';
import '../../api/rooms_repository.dart';
import '../../config/app_config.dart';
import '../../theme/any_colors.dart';
import '../../ui/live_badge.dart';
import '../rooms/room_page.dart';
import '../../l10n/l10n.dart';

/// OBS-first Go Live tab (create + start + publish credentials).
class GoLivePage extends StatefulWidget {
  const GoLivePage({
    super.key,
    required this.config,
    required this.accessToken,
    this.userId,
    this.roomsRepository,
  });

  final AppConfig config;
  final String accessToken;
  final String? userId;
  final RoomsRepository? roomsRepository;

  @override
  State<GoLivePage> createState() => _GoLivePageState();
}

class _GoLivePageState extends State<GoLivePage> {
  late final RoomsRepository _rooms;
  final _titleController = TextEditingController();
  Room? _room;
  PublishInfo? _publish;
  String? _error;
  bool _busy = false;

  @override
  void initState() {
    super.initState();
    final api = ApiClient(
      baseUrl: widget.config.normalizedApiBaseUrl,
      accessToken: widget.accessToken,
    );
    _rooms = widget.roomsRepository ?? RoomsRepository(client: api);
  }

  @override
  void dispose() {
    _titleController.dispose();
    super.dispose();
  }

  Future<void> _startLive() async {
    if (_busy) return;
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      final title = _titleController.text.trim().isEmpty
          ? context.l10n.defaultLiveTitle
          : _titleController.text.trim();
      final room = await _rooms.createRoom(title);
      final started = await _rooms.startRoom(room.id);
      PublishInfo? publish;
      try {
        publish = await _rooms.publishInfo(room.id);
      } catch (_) {}
      if (!mounted) return;
      setState(() {
        _room = started;
        _publish = publish;
        _busy = false;
      });
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _error = e.toString();
        _busy = false;
      });
    }
  }

  Future<void> _stopLive() async {
    final room = _room;
    if (room == null || _busy) return;
    setState(() => _busy = true);
    try {
      final stopped = await _rooms.stopRoom(room.id);
      if (!mounted) return;
      setState(() {
        _room = stopped;
        _publish = null;
        _busy = false;
      });
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(context.l10n.liveEnded)),
      );
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _error = e.toString();
        _busy = false;
      });
    }
  }

  Future<void> _refreshPublish() async {
    final room = _room;
    if (room == null || _busy) return;
    setState(() => _busy = true);
    try {
      if (!room.isLive) {
        final started = await _rooms.startRoom(room.id);
        _room = started;
      }
      final publish = await _rooms.publishInfo(room.id);
      if (!mounted) return;
      setState(() {
        _publish = publish;
        _busy = false;
      });
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _error = e.toString();
        _busy = false;
      });
    }
  }

  Future<void> _copy(String label, String value) async {
    await Clipboard.setData(ClipboardData(text: value));
    if (!mounted) return;
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(content: Text(context.l10n.labelCopied(label))),
    );
  }

  void _openRoom() {
    final room = _room;
    if (room == null) return;
    Navigator.of(context).push(
      MaterialPageRoute(
        builder: (_) => RoomPage(
          config: widget.config,
          accessToken: widget.accessToken,
          userId: widget.userId,
          room: room,
          roomsRepository: _rooms,
        ),
      ),
    );
  }

  String _obsServer(String pushUrl, String streamKey) {
    if (pushUrl.isEmpty) return 'rtmp://localhost:1935/live';
    // push_url is rtmp://host/app/stream?… — server is rtmp://host/app
    final withoutQuery = pushUrl.split('?').first;
    final key = streamKey.split('?').first;
    if (key.isNotEmpty && withoutQuery.endsWith('/$key')) {
      return withoutQuery.substring(
        0,
        withoutQuery.length - key.length - 1,
      );
    }
    // Fallback: drop last path segment
    final i = withoutQuery.lastIndexOf('/');
    if (i > 'rtmp://x'.length) return withoutQuery.substring(0, i);
    return withoutQuery;
  }

  @override
  Widget build(BuildContext context) {
    final room = _room;
    final publish = _publish;
    final push = publish?.pushUrl ?? '';
    final key = publish?.streamKey ?? room?.id ?? '';
    final server = _obsServer(push, key);
    final isLive = room != null && room.isLive;

    return Scaffold(
      backgroundColor: AnyColors.bg,
      appBar: AppBar(
        title: Text(context.l10n.goLiveTitle),
        backgroundColor: AnyColors.bg,
      ),
      body: ListView(
        padding: const EdgeInsets.fromLTRB(16, 8, 16, 32),
        children: [
          Text(
            isLive ? context.l10n.youAreLive : context.l10n.broadcastWithObs,
            style: const TextStyle(
              color: AnyColors.textPrimary,
              fontSize: 20,
              fontWeight: FontWeight.w700,
            ),
          ),
          SizedBox(height: 8),
          Text(
            'Stream with OBS (recommended for P1). Paste Server + full Stream Key including ?exp=&sig=.',
            style: TextStyle(color: AnyColors.textSecondary, height: 1.35),
          ),
          SizedBox(height: 16),
          TextField(
            key: const Key('go-live-title'),
            controller: _titleController,
            enabled: !isLive && !_busy,
            decoration: InputDecoration(
              labelText: context.l10n.roomTitle,
              hintText: context.l10n.defaultLiveTitle,
            ),
          ),
          SizedBox(height: 16),
          if (!isLive)
            FilledButton.icon(
              key: const Key('go-live-start'),
              onPressed: _busy ? null : _startLive,
              icon: const Icon(Icons.videocam),
              label: Text(_busy ? context.l10n.working : context.l10n.startLive),
            ),
          if (_error != null) ...[
            const SizedBox(height: 12),
            Text(_error!, style: const TextStyle(color: AnyColors.danger)),
          ],
          if (room != null) ...[
            const SizedBox(height: 24),
            Container(
              padding: const EdgeInsets.all(16),
              decoration: BoxDecoration(
                color: AnyColors.elevated,
                borderRadius: BorderRadius.circular(12),
                border: Border.all(color: AnyColors.border),
              ),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      if (room.isLive) const LiveBadge(),
                      if (room.isLive) const SizedBox(width: 8),
                      Expanded(
                        child: Text(
                          room.title,
                          style: const TextStyle(
                            fontWeight: FontWeight.w600,
                            fontSize: 16,
                            color: AnyColors.textPrimary,
                          ),
                        ),
                      ),
                    ],
                  ),
                  SizedBox(height: 6),
                  Text(
                    context.l10n.roomStatus(room.status),
                    style: const TextStyle(
                      color: AnyColors.textMuted,
                      fontSize: 12,
                    ),
                  ),
                  const SizedBox(height: 4),
                  SelectableText(
                    room.id,
                    style: const TextStyle(
                      color: AnyColors.textMuted,
                      fontSize: 11,
                      fontFamily: 'monospace',
                    ),
                  ),
                ],
              ),
            ),
            if (publish != null || room.isLive) ...[
              SizedBox(height: 16),
              _credTile(
                label: context.l10n.obsServer,
                value: server,
                onCopy: () => _copy(context.l10n.server, server),
              ),
              _credTile(
                label: 'Stream Key (full, with ?exp=&sig=)',
                value: key,
                onCopy: () => _copy(context.l10n.streamKey, key),
              ),
            ],
            SizedBox(height: 12),
            Wrap(
              spacing: 8,
              runSpacing: 8,
              children: [
                OutlinedButton(
                  onPressed: _busy ? null : _refreshPublish,
                  child: Text(context.l10n.refreshObsKeys),
                ),
                OutlinedButton(
                  key: const Key('go-live-open-room'),
                  onPressed: _openRoom,
                  child: Text(context.l10n.openMyRoom),
                ),
                if (room.isLive)
                  OutlinedButton(
                    key: const Key('go-live-end'),
                    onPressed: _busy ? null : _stopLive,
                    child: Text(
                      context.l10n.endLive,
                      style: TextStyle(color: AnyColors.danger),
                    ),
                  ),
              ],
            ),
          ],
          SizedBox(height: 24),
          Container(
            padding: const EdgeInsets.all(14),
            decoration: BoxDecoration(
              color: AnyColors.surface,
              borderRadius: BorderRadius.circular(10),
              border: Border.all(color: AnyColors.border),
            ),
            child: Text(
              '${context.l10n.obsInstructions}\n'
              '${context.l10n.obsKeySeparate}',
              style: TextStyle(
                color: AnyColors.textMuted,
                fontSize: 13,
                height: 1.45,
              ),
            ),
          ),
        ],
      ),
    );
  }

  Widget _credTile({
    required String label,
    required String value,
    required VoidCallback onCopy,
  }) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 10),
      child: Container(
        width: double.infinity,
        padding: const EdgeInsets.all(12),
        decoration: BoxDecoration(
          color: AnyColors.surface,
          borderRadius: BorderRadius.circular(10),
          border: Border.all(color: AnyColors.border),
        ),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              label,
              style: const TextStyle(
                fontSize: 12,
                color: AnyColors.textMuted,
                fontWeight: FontWeight.w600,
              ),
            ),
            SizedBox(height: 6),
            SelectableText(
              value.isEmpty ? '—' : value,
              style: const TextStyle(
                fontSize: 13,
                color: AnyColors.textPrimary,
                fontFamily: 'monospace',
              ),
            ),
            Align(
              alignment: Alignment.centerRight,
              child: TextButton.icon(
                onPressed: value.isEmpty ? null : onCopy,
                icon: const Icon(Icons.copy, size: 16),
                label: Text(context.l10n.copy),
              ),
            ),
          ],
        ),
      ),
    );
  }
}
