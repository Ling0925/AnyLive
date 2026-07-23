import 'dart:async';

import 'package:flutter/material.dart';

import '../../api/api_client.dart';
import '../../api/events_repository.dart';
import '../../api/rooms_repository.dart';
import '../../api/social_repository.dart';
import '../../config/app_config.dart';
import '../../ui/empty_state.dart';
import '../../ui/live_card.dart';
import '../rooms/room_page.dart';

/// Which feed list this page shows when embedded in [MainShell].
enum FeedMode {
  /// Hot rooms + search (Home tab).
  home,

  /// Following-only list (Following tab).
  following,

  /// Legacy combined Discover with Hot | Following tabs.
  discover,
}

/// Discover feed — Hot / Following / combined modes.
class FeedPage extends StatefulWidget {
  const FeedPage({
    super.key,
    required this.config,
    required this.accessToken,
    this.userId,
    this.mode = FeedMode.discover,
    this.onJumpHome,
    this.socialRepository,
    this.eventsRepository,
  });

  final AppConfig config;
  final String accessToken;

  /// Current user id for host-only room controls (optional).
  final String? userId;

  /// Feed mode for shell tabs; defaults to combined Discover for tests.
  final FeedMode mode;

  /// Following empty-state CTA → switch shell to Home.
  final VoidCallback? onJumpHome;

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
  TabController? _tabs;

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

  bool get _isDiscover => widget.mode == FeedMode.discover;
  bool get _showHot =>
      widget.mode == FeedMode.home || widget.mode == FeedMode.discover;
  bool get _showFollowing =>
      widget.mode == FeedMode.following || widget.mode == FeedMode.discover;
  bool get _showSearch =>
      widget.mode == FeedMode.home || widget.mode == FeedMode.discover;

  @override
  void initState() {
    super.initState();
    if (_isDiscover) {
      _tabs = TabController(length: 2, vsync: this);
    }
    final api = ApiClient(
      baseUrl: widget.config.normalizedApiBaseUrl,
      accessToken: widget.accessToken,
    );
    _social = widget.socialRepository ?? SocialRepository(client: api);
    _events = widget.eventsRepository ?? EventsRepository(client: api);
    _rooms = RoomsRepository(client: api);
    if (_showHot) _reloadHot();
    if (_showFollowing) _reloadFollowing();
  }

  @override
  void dispose() {
    _tabs?.dispose();
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
    String? emptyCta,
    VoidCallback? onEmptyCta,
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
      return EmptyState(
        message: emptyLabel,
        ctaLabel: emptyCta,
        onCta: onEmptyCta,
        icon: Icons.live_tv_outlined,
      );
    }
    return RefreshIndicator(
      onRefresh: () async => onRetry(),
      child: ListView.separated(
        padding: const EdgeInsets.fromLTRB(12, 8, 12, 16),
        itemCount: items.length,
        separatorBuilder: (_, _) => const SizedBox(height: 12),
        itemBuilder: (context, i) {
          final r = items[i];
          return LiveCard(
            room: r,
            onTap: () => _openRoom(r),
          );
        },
      ),
    );
  }

  String get _title {
    switch (widget.mode) {
      case FeedMode.home:
        return 'Home';
      case FeedMode.following:
        return 'Following';
      case FeedMode.discover:
        return 'Discover';
    }
  }

  PreferredSizeWidget? _appBarBottom() {
    if (!_showSearch && !_isDiscover) return null;

    if (_isDiscover) {
      return PreferredSize(
        preferredSize: const Size.fromHeight(96),
        child: Column(
          children: [
            if (_showSearch) _searchField(),
            TabBar(
              controller: _tabs,
              tabs: const [
                Tab(text: 'Hot'),
                Tab(text: 'Following'),
              ],
            ),
          ],
        ),
      );
    }

    // Home tab: search only.
    return PreferredSize(
      preferredSize: const Size.fromHeight(56),
      child: _searchField(),
    );
  }

  Widget _searchField() {
    return Padding(
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
    );
  }

  List<Widget> _searchResultsWidgets() {
    if (!_showSearch) return const [];
    if (_searchLoading) {
      return const [LinearProgressIndicator(minHeight: 2)];
    }
    if (_searchError != null) {
      return [
        Padding(
          padding: const EdgeInsets.all(8),
          child: Text(_searchError!, style: const TextStyle(color: Colors.red)),
        ),
      ];
    }
    if (_searchRooms.isEmpty && _searchUsers.isEmpty) return const [];
    return [
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
    ];
  }

  void _refreshAll() {
    if (_showHot) _reloadHot();
    if (_showFollowing) _reloadFollowing();
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: Text(_title),
        bottom: _appBarBottom(),
        actions: [
          IconButton(
            onPressed: _refreshAll,
            icon: const Icon(Icons.refresh),
          ),
        ],
      ),
      body: Column(
        children: [
          ..._searchResultsWidgets(),
          Expanded(child: _bodyContent()),
        ],
      ),
    );
  }

  Widget _bodyContent() {
    switch (widget.mode) {
      case FeedMode.home:
        return _roomList(
          loading: _hotLoading,
          error: _hotError,
          items: _hot,
          onRetry: _reloadHot,
          emptyLabel: 'No hot rooms',
        );
      case FeedMode.following:
        return _roomList(
          loading: _followingLoading,
          error: _followingError,
          items: _following,
          onRetry: _reloadFollowing,
          emptyLabel: 'No rooms from people you follow',
          emptyCta: widget.onJumpHome != null ? 'Browse Home' : null,
          onEmptyCta: widget.onJumpHome,
        );
      case FeedMode.discover:
        return TabBarView(
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
        );
    }
  }
}
