import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../../api/api_client.dart';
import '../../api/rooms_repository.dart';
import '../../config/app_config.dart';
import 'room_page.dart';
import '../../l10n/l10n.dart';

/// Lists live rooms from the control-plane API and hosts go-live + OBS publish.
class RoomListPage extends StatefulWidget {
  const RoomListPage({
    super.key,
    required this.config,
    required this.accessToken,
    this.userId,
    this.roomsRepository,
  });

  final AppConfig config;
  final String accessToken;

  /// Current user id for host-only room controls (optional).
  final String? userId;

  /// Optional injectable repository for tests.
  final RoomsRepository? roomsRepository;

  @override
  State<RoomListPage> createState() => _RoomListPageState();
}

class _RoomListPageState extends State<RoomListPage> {
  late final RoomsRepository _rooms;
  List<Room> _items = [];
  String? _error;
  bool _loading = true;
  bool _goingLive = false;

  @override
  void initState() {
    super.initState();
    final api = ApiClient(
      baseUrl: widget.config.normalizedApiBaseUrl,
      accessToken: widget.accessToken,
    );
    _rooms = widget.roomsRepository ?? RoomsRepository(client: api);
    _reload();
  }

  Future<void> _reload() async {
    setState(() {
      _loading = true;
      _error = null;
    });
    try {
      final list = await _rooms.listRooms(status: 'live');
      if (!mounted) return;
      setState(() {
        _items = list;
        _loading = false;
      });
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _error = e.toString();
        _loading = false;
      });
    }
  }

  Future<void> _createAndStart() async {
    if (_goingLive) return;
    setState(() => _goingLive = true);
    try {
      final room = await _rooms.createRoom('My Live ${DateTime.now().minute}');
      final started = await _rooms.startRoom(room.id);
      PublishInfo? publish;
      try {
        publish = await _rooms.publishInfo(room.id);
      } catch (_) {
        // Publish credentials are best-effort for the dialog.
      }
      await _reload();
      if (!mounted) return;
      final open = await showDialog<bool>(
        context: context,
        builder: (ctx) => _GoLiveDialog(room: started, publish: publish),
      );
      if (open == true && mounted) {
        _openRoom(started);
      }
    } catch (e) {
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(context.l10n.actionFailed('action', '$e'))),
      );
    } finally {
      if (mounted) setState(() => _goingLive = false);
    }
  }

  void _openRoom(Room room) {
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

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: Text(context.l10n.liveRooms),
        actions: [
          IconButton(onPressed: _reload, icon: const Icon(Icons.refresh)),
        ],
      ),
      floatingActionButton: FloatingActionButton.extended(
        onPressed: _goingLive ? null : _createAndStart,
        label: Text(_goingLive ? context.l10n.starting : context.l10n.goLiveAction),
        icon: const Icon(Icons.videocam),
      ),
      body: _loading
          ? const Center(child: CircularProgressIndicator())
          : _error != null
              ? Center(child: Text(_error!))
              : _items.isEmpty
                  ? Center(child: Text(context.l10n.noLiveRooms))
                  : ListView.separated(
                      itemCount: _items.length,
                      separatorBuilder: (_, _) => const Divider(height: 1),
                      itemBuilder: (context, i) {
                        final r = _items[i];
                        return ListTile(
                          title: Text(r.title),
                          subtitle: Text('${r.status} · ${r.id}'),
                          trailing: r.isLive
                              ? const Icon(Icons.circle,
                                  color: Colors.red, size: 12)
                              : null,
                          onTap: () => _openRoom(r),
                        );
                      },
                    ),
    );
  }
}

/// Shows OBS RTMP push URL / stream key after go-live.
class _GoLiveDialog extends StatelessWidget {
  const _GoLiveDialog({required this.room, this.publish});

  final Room room;
  final PublishInfo? publish;

  Future<void> _copy(BuildContext context, String label, String value) async {
    await Clipboard.setData(ClipboardData(text: value));
    if (!context.mounted) return;
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(content: Text(context.l10n.labelCopied(label))),
    );
  }

  @override
  Widget build(BuildContext context) {
    final push = publish?.pushUrl ?? '';
    final key = publish?.streamKey ?? room.id;
    // push_url is rtmp://host/app/stream — OBS wants Server=rtmp://host/app and Key=stream.
    final server = _obsServerFromPushUrl(push, key);
    return AlertDialog(
      title: Text(context.l10n.youAreLive),
      content: SingleChildScrollView(
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          mainAxisSize: MainAxisSize.min,
          children: [
            Text(room.title, style: Theme.of(context).textTheme.titleMedium),
            SizedBox(height: 8),
            Text(context.l10n.roomIdLine(room.id),
                style: Theme.of(context).textTheme.bodySmall),
            SizedBox(height: 16),
            Text(context.l10n.obsServerCustom,
                style: Theme.of(context).textTheme.labelLarge),
            SizedBox(height: 4),
            SelectableText(server.isEmpty ? context.l10n.unavailable : server),
            SizedBox(height: 8),
            Text(context.l10n.obsStreamKey,
                style: Theme.of(context).textTheme.labelLarge),
            SelectableText(key),
            SizedBox(height: 8),
            Text(
              '${context.l10n.obsInstructions} '
              '${context.l10n.obsKeySeparate}',
            ),
          ],
        ),
      ),
      actions: [
        if (server.isNotEmpty)
          TextButton(
            onPressed: () => _copy(context, context.l10n.obsServer, server),
            child: Text(context.l10n.copyServer),
          ),
        TextButton(
          onPressed: () => _copy(context, 'Stream key', key),
          child: Text(context.l10n.copyStreamKey),
        ),
        FilledButton(
          onPressed: () => Navigator.of(context).pop(true),
          child: Text(context.l10n.openRoom),
        ),
        TextButton(
          onPressed: () => Navigator.of(context).pop(false),
          child: Text(context.l10n.close),
        ),
      ],
    );
  }
}

/// Strip trailing /{streamKey} from a full RTMP push URL for OBS Server field.
String _obsServerFromPushUrl(String pushUrl, String streamKey) {
  final push = pushUrl.trim();
  if (push.isEmpty) return '';
  final key = streamKey.trim();
  if (key.isNotEmpty) {
    final suffix = '/$key';
    if (push.endsWith(suffix)) {
      return push.substring(0, push.length - suffix.length);
    }
  }
  final i = push.lastIndexOf('/');
  if (i > 'rtmp://'.length) return push.substring(0, i);
  return push;
}

