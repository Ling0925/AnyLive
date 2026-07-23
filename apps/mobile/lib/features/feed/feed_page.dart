import 'dart:async';

import 'package:flutter/material.dart';

import '../../api/api_client.dart';
import '../../api/events_repository.dart';
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
    this.userId,
    this.socialRepository,
    this.eventsRepository,
  });

  final AppConfig config;
  final String accessToken;

  /// Current user id for host-only room controls (optional).
  final String? userId;

  /// Injectable for tests; when null a real [SocialRepository] is created.
  final SocialRepository? socialRepository;
  final EventsRepository? eventsRepository;

  @override
  State<FeedPage> createState() => _FeedPageState();
}

class _FeedPageState extends State<FeedPage>
    with SingleTickerProviderStateMixin {
  late final SocialRepository _social;
  late final EventsRepository _events;
  late final TabController _tabs;

  List<Room> _hot = [];
  List<Room> _following = [];
  String? _hotError;
  String? _followingError;
  bool _hotLoading = true;
  bool _followingLoading = true;
  late final RoomsRepository _rooms;
  final _searchController = TextEditingController();
  List<Room> _searchRooms = [];
  List<SearchUserHit> _searchUsers = [];
  String? _searchError;
  bool _searchLoading = false;

  @override
  void initState() {
    super.initState();
    _tabs = TabController(length: 2, vsync: this);
    final api = ApiClient(
      baseUrl: widget.config.normalizedApiBaseUrl,
      accessToken: widget.accessToken,
    );
    _social = widget.socialRepository ?? SocialRepository(client: api);
    _events = widget.eventsRepository ?? EventsRepository(client: api);
    _rooms = RoomsRepository(client: api);
    _reloadHot();
    _reloadFollowing();
  }

  @override
  void dispose() {
    _tabs.dispose();
    _searchController.dispose();
    super.dispose();
  }

  Future<void> _runSearch(String q) async {
    final query = q.trim();
    if (query.isEmpty) {
      setState(() {
        _searchRooms = [];
        _searchUsers = [];
        _searchError = null;
      });
      return;
    }
    setState(() {
      _searchLoading = true;
      _searchError = null;
    });
    try {
      final result = await _rooms.search(query);
      if (!mounted) return;
      setState(() {
        _searchRooms = result.rooms;
        _searchUsers = result.users;
        _searchLoading = false;
      });
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _searchError = e.toString();
        _searchLoading = false;
      });
    }
  }

  void _trackImpressions(List<Room> rooms, String feed) {
    for (final r in rooms.take(20)) {
      unawaited(_events.track(
        'feed.impression',
        props: {'room_id': r.id, 'feed': feed},
      ));
    }
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
      _trackImpressions(list, 'hot');
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
      _trackImpressions(list, 'following');
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
          userId: widget.userId,
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
        bottom: PreferredSize(
          preferredSize: const Size.fromHeight(96),
          child: Column(
            children: [
              Padding(
                padding: const EdgeInsets.fromLTRB(12, 0, 12, 8),
                child: TextField(
                  key: const Key('feed-search'),
                  controller: _searchController,
                  decoration: InputDecoration(
                    hintText: 'Search rooms or users',
                    isDense: true,
                    border: const OutlineInputBorder(),
                    suffixIcon: IconButton(
                      icon: const Icon(Icons.search),
                      onPressed: () => _runSearch(_searchController.text),
                    ),
                  ),
                  textInputAction: TextInputAction.search,
                  onSubmitted: _runSearch,
                ),
              ),
              TabBar(
                controller: _tabs,
                tabs: const [
                  Tab(text: 'Hot'),
                  Tab(text: 'Following'),
                ],
              ),
            ],
          ),
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
      body: Column(
        children: [
          if (_searchLoading)
            const LinearProgressIndicator(minHeight: 2)
          else if (_searchError != null)
            Padding(
              padding: const EdgeInsets.all(8),
              child: Text(_searchError!, style: const TextStyle(color: Colors.red)),
            )
          else if (_searchRooms.isNotEmpty || _searchUsers.isNotEmpty)
            SizedBox(
              height: 120,
              child: ListView(
                padding: const EdgeInsets.symmetric(horizontal: 8),
                children: [
                  ..._searchRooms.map(
                    (r) => ListTile(
                      dense: true,
                      title: Text(r.title),
                      subtitle: Text('room · ${r.status}'),
                      onTap: () => _openRoom(r),
                    ),
                  ),
                  ..._searchUsers.map(
                    (u) => ListTile(
                      dense: true,
                      title: Text(u.displayName),
                      subtitle: Text('user · ${u.id}'),
                    ),
                  ),
                ],
              ),
            ),
          Expanded(
            child: TabBarView(
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
          ),
        ],
      ),
    );
  }
}
