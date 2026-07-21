import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../../api/api_client.dart';
import '../../api/rooms_repository.dart';
import '../../config/app_config.dart';
import 'room_page.dart';

/// Lists live rooms from the control-plane API and hosts go-live + OBS publish.
class RoomListPage extends StatefulWidget {
  const RoomListPage({
    super.key,
    required this.config,
    required this.accessToken,
    this.roomsRepository,
  });

  final AppConfig config;
  final String accessToken;

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
      await showDialog<void>(
        context: context,
        builder: (ctx) => _GoLiveDialog(room: started, publish: publish),
      );
    } catch (e) {
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('failed: $e')),
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
        title: const Text('Live rooms'),
        actions: [
          IconButton(onPressed: _reload, icon: const Icon(Icons.refresh)),
        ],
      ),
      floatingActionButton: FloatingActionButton.extended(
        onPressed: _goingLive ? null : _createAndStart,
        label: Text(_goingLive ? 'Starting…' : 'Go live'),
        icon: const Icon(Icons.videocam),
      ),
      body: _loading
          ? const Center(child: CircularProgressIndicator())
          : _error != null
              ? Center(child: Text(_error!))
              : _items.isEmpty
                  ? const Center(child: Text('No live rooms'))
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
      SnackBar(content: Text('$label copied')),
    );
  }

  @override
  Widget build(BuildContext context) {
    final push = publish?.pushUrl ?? '';
    final key = publish?.streamKey ?? room.id;
    return AlertDialog(
      title: const Text('You are live'),
      content: SingleChildScrollView(
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          mainAxisSize: MainAxisSize.min,
          children: [
            Text(room.title, style: Theme.of(context).textTheme.titleMedium),
            const SizedBox(height: 8),
            Text('Room: ${room.id}',
                style: Theme.of(context).textTheme.bodySmall),
            const SizedBox(height: 16),
            Text('OBS / RTMP publish',
                style: Theme.of(context).textTheme.labelLarge),
            const SizedBox(height: 4),
            SelectableText(push.isEmpty ? '(unavailable)' : push),
            const SizedBox(height: 8),
            Text('Stream key', style: Theme.of(context).textTheme.labelLarge),
            SelectableText(key),
            const SizedBox(height: 8),
            const Text(
              'P1 host path: paste push URL into OBS Custom RTMP. '
              'Stream key is the room UUID.',
            ),
          ],
        ),
      ),
      actions: [
        if (push.isNotEmpty)
          TextButton(
            onPressed: () => _copy(context, 'Push URL', push),
            child: const Text('Copy push URL'),
          ),
        TextButton(
          onPressed: () => _copy(context, 'Stream key', key),
          child: const Text('Copy stream key'),
        ),
        FilledButton(
          onPressed: () => Navigator.of(context).pop(),
          child: const Text('Open room'),
        ),
      ],
    );
  }
}
