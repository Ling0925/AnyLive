import 'package:flutter/material.dart';

import '../../api/api_client.dart';
import '../../api/rooms_repository.dart';
import '../../config/app_config.dart';

/// Lists live rooms from the control-plane API.
class RoomListPage extends StatefulWidget {
  const RoomListPage({
    super.key,
    required this.config,
    required this.accessToken,
  });

  final AppConfig config;
  final String accessToken;

  @override
  State<RoomListPage> createState() => _RoomListPageState();
}

class _RoomListPageState extends State<RoomListPage> {
  late final RoomsRepository _rooms;
  List<Room> _items = [];
  String? _error;
  bool _loading = true;

  @override
  void initState() {
    super.initState();
    final api = ApiClient(
      baseUrl: widget.config.normalizedApiBaseUrl,
      accessToken: widget.accessToken,
    );
    _rooms = RoomsRepository(client: api);
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
    try {
      final room = await _rooms.createRoom('My Live ${DateTime.now().minute}');
      await _rooms.startRoom(room.id);
      await _reload();
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('Live room ${room.id}')),
      );
    } catch (e) {
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('failed: $e')),
      );
    }
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
        onPressed: _createAndStart,
        label: const Text('Go live'),
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
                      separatorBuilder: (_, __) => const Divider(height: 1),
                      itemBuilder: (context, i) {
                        final r = _items[i];
                        return ListTile(
                          title: Text(r.title),
                          subtitle: Text('${r.status} · ${r.id}'),
                          trailing: r.isLive
                              ? const Icon(Icons.circle, color: Colors.red, size: 12)
                              : null,
                        );
                      },
                    ),
    );
  }
}
