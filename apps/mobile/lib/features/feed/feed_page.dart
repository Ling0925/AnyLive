import 'package:flutter/material.dart';

import '../../api/api_client.dart';
import '../../api/rooms_repository.dart';
import '../../api/social_repository.dart';
import '../../config/app_config.dart';
import '../rooms/room_page.dart';

/// Discover feed with Hot / Following tabs.
class FeedPage extends StatefulWidget {
  const FeedPage({
    super.key,
    required this.config,
    required this.accessToken,
    this.socialRepository,
  });

  final AppConfig config;
  final String accessToken;

  /// Injectable for tests; when null a real [SocialRepository] is created.
  final SocialRepository? socialRepository;

  @override
  State<FeedPage> createState() => _FeedPageState();
}

class _FeedPageState extends State<FeedPage>
    with SingleTickerProviderStateMixin {
  late final SocialRepository _social;
  late final TabController _tabs;

  List<Room> _hot = [];
  List<Room> _following = [];
  String? _hotError;
  String? _followingError;
  bool _hotLoading = true;
  bool _followingLoading = true;

  @override
  void initState() {
    super.initState();
    _tabs = TabController(length: 2, vsync: this);
    if (widget.socialRepository != null) {
      _social = widget.socialRepository!;
    } else {
      final api = ApiClient(
        baseUrl: widget.config.normalizedApiBaseUrl,
        accessToken: widget.accessToken,
      );
      _social = SocialRepository(client: api);
    }
    _reloadHot();
    _reloadFollowing();
  }

  @override
  void dispose() {
    _tabs.dispose();
    super.dispose();
  }

  Future<void> _reloadHot() async {
    setState(() {
      _hotLoading = true;
      _hotError = null;
    });
    try {
      final list = await _social.feedHot();
      if (!mounted) return;
      setState(() {
        _hot = list;
        _hotLoading = false;
      });
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _hotError = e.toString();
        _hotLoading = false;
      });
    }
  }

  Future<void> _reloadFollowing() async {
    setState(() {
      _followingLoading = true;
      _followingError = null;
    });
    try {
      final list = await _social.feedFollowing();
      if (!mounted) return;
      setState(() {
        _following = list;
        _followingLoading = false;
      });
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _followingError = e.toString();
        _followingLoading = false;
      });
    }
  }

  void _openRoom(Room room) {
    Navigator.of(context).push(
      MaterialPageRoute(
        builder: (_) => RoomPage(
          config: widget.config,
          accessToken: widget.accessToken,
          room: room,
        ),
      ),
    );
  }

  Widget _roomList({
    required bool loading,
    required String? error,
    required List<Room> items,
    required VoidCallback onRetry,
    required String emptyLabel,
  }) {
    if (loading) {
      return const Center(child: CircularProgressIndicator());
    }
    if (error != null) {
      return Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Text(error),
            const SizedBox(height: 8),
            TextButton(onPressed: onRetry, child: const Text('Retry')),
          ],
        ),
      );
    }
    if (items.isEmpty) {
      return Center(child: Text(emptyLabel));
    }
    return RefreshIndicator(
      onRefresh: () async => onRetry(),
      child: ListView.separated(
        itemCount: items.length,
        separatorBuilder: (_, _) => const Divider(height: 1),
        itemBuilder: (context, i) {
          final r = items[i];
          return ListTile(
            title: Text(r.title),
            subtitle: Text('${r.status} · ${r.id}'),
            trailing: r.isLive
                ? const Icon(Icons.circle, color: Colors.red, size: 12)
                : null,
            onTap: () => _openRoom(r),
          );
        },
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('Discover'),
        bottom: TabBar(
          controller: _tabs,
          tabs: const [
            Tab(text: 'Hot'),
            Tab(text: 'Following'),
          ],
        ),
        actions: [
          IconButton(
            onPressed: () {
              _reloadHot();
              _reloadFollowing();
            },
            icon: const Icon(Icons.refresh),
          ),
        ],
      ),
      body: TabBarView(
        controller: _tabs,
        children: [
          _roomList(
            loading: _hotLoading,
            error: _hotError,
            items: _hot,
            onRetry: _reloadHot,
            emptyLabel: 'No hot rooms',
          ),
          _roomList(
            loading: _followingLoading,
            error: _followingError,
            items: _following,
            onRetry: _reloadFollowing,
            emptyLabel: 'No rooms from people you follow',
          ),
        ],
      ),
    );
  }
}
