import 'dart:async';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../../api/api_client.dart';
import '../../api/gifts_repository.dart';
import '../../api/reports_repository.dart';
import '../../api/rooms_repository.dart';
import '../../api/social_repository.dart';
import '../../config/app_config.dart';
import '../../player/stream_preview.dart';

/// Live room detail: title/status, HLS preview, chat, gifts, wallet, follow, report.
///
/// In-app video player is scaffolded without `video_player` (keeps CI free of
/// native plugins). [StreamPreview] shows a black play placeholder + copyable
/// HLS URL for external players (VLC / browser).
class RoomPage extends StatefulWidget {
  const RoomPage({
    super.key,
    required this.config,
    required this.accessToken,
    required this.room,
    this.roomsRepository,
    this.giftsRepository,
    this.socialRepository,
    this.reportsRepository,
  });

  final AppConfig config;
  final String accessToken;
  final Room room;

  /// Optional injectable repos for tests.
  final RoomsRepository? roomsRepository;
  final GiftsRepository? giftsRepository;
  final SocialRepository? socialRepository;
  final ReportsRepository? reportsRepository;

  @override
  State<RoomPage> createState() => _RoomPageState();
}

class _RoomPageState extends State<RoomPage> {
  late RoomsRepository _rooms;
  late GiftsRepository _gifts;
  late SocialRepository _social;
  late ReportsRepository _reports;
  late Room _room;

  final _chatController = TextEditingController();
  List<ChatMessage> _messages = [];
  List<GiftItem> _giftsCatalog = [];
  int _balance = 0;
  String? _hlsUrl;
  String? _error;
  bool _loading = true;
  bool _sending = false;
  bool _followingHost = false;
  bool _followBusy = false;
  bool _hostBusy = false;
  PublishInfo? _publishInfo;
  /// Durable gift idempotency keys keyed by gift id (reused until success).
  final Map<String, String> _giftRequestIds = {};
  Timer? _statusPoll;

  bool get _roomEnded =>
      !_room.isLive &&
      (_room.status == 'closed' ||
          _room.status == 'idle' ||
          _room.status == 'ended');

  @override
  void initState() {
    super.initState();
    _room = widget.room;
    final api = ApiClient(
      baseUrl: widget.config.normalizedApiBaseUrl,
      accessToken: widget.accessToken,
    );
    _rooms = widget.roomsRepository ?? RoomsRepository(client: api);
    _gifts = widget.giftsRepository ?? GiftsRepository(client: api);
    _social = widget.socialRepository ?? SocialRepository(client: api);
    _reports = widget.reportsRepository ?? ReportsRepository(client: api);
    _load();
    _startStatusPoll();
  }

  void _startStatusPoll() {
    _statusPoll?.cancel();
    // Lightweight poll so stop/force-close/webhook flips ended UI without manual refresh.
    _statusPoll = Timer.periodic(const Duration(seconds: 8), (_) async {
      if (!mounted || _roomEnded) {
        _statusPoll?.cancel();
        return;
      }
      try {
        final room = await _rooms.getRoom(_room.id);
        if (!mounted) return;
        if (room.status != _room.status) {
          setState(() {
            _room = room;
            if (!room.isLive) {
              _hlsUrl = null;
              _statusPoll?.cancel();
            }
          });
        }
      } catch (_) {
        // ignore transient poll errors
      }
    });
  }

  @override
  void dispose() {
    _statusPoll?.cancel();
    _chatController.dispose();
    super.dispose();
  }

  Future<void> _load() async {
    setState(() {
      _loading = true;
      _error = null;
    });
    try {
      final room = await _rooms.getRoom(_room.id);
      final messages = await _rooms.listMessages(_room.id);
      final gifts = await _gifts.listGifts();
      final balance = await _gifts.walletBalance();

      bool following = false;
      try {
        final ids = await _social.listFollowing();
        following = ids.contains(room.ownerId);
      } catch (_) {
        // Follow state is best-effort.
      }

      String? hls;
      final ended = !room.isLive &&
          (room.status == 'closed' ||
              room.status == 'idle' ||
              room.status == 'ended');
      if (room.isLive && !ended) {
        try {
          final play = await _rooms.playUrls(_room.id);
          hls = play['hls'] as String?;
        } catch (_) {
          // Room may flip status; show without HLS.
        }
      }

      if (!mounted) return;
      setState(() {
        _room = room;
        _messages = messages;
        _giftsCatalog = gifts;
        _balance = balance;
        _hlsUrl = hls;
        _followingHost = following;
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

  Future<void> _toggleFollow() async {
    if (_followBusy || _room.ownerId.isEmpty) return;
    setState(() => _followBusy = true);
    try {
      if (_followingHost) {
        await _social.unfollow(_room.ownerId);
        if (!mounted) return;
        setState(() => _followingHost = false);
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(content: Text('Unfollowed host')),
        );
      } else {
        await _social.follow(_room.ownerId);
        if (!mounted) return;
        setState(() => _followingHost = true);
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(content: Text('Following host')),
        );
      }
    } catch (e) {
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('follow failed: $e')),
      );
    } finally {
      if (mounted) setState(() => _followBusy = false);
    }
  }

  Future<void> _reportRoom() async {
    final reason = await showDialog<String>(
      context: context,
      builder: (ctx) => const _ReportRoomDialog(),
    );
    if (reason == null || reason.isEmpty || !mounted) return;

    try {
      await _reports.createReport(
        targetType: 'room',
        targetId: _room.id,
        reason: reason,
      );
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('Report submitted')),
      );
    } catch (e) {
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('report failed: $e')),
      );
    }
  }

  Future<void> _sendChat() async {
    final text = _chatController.text.trim();
    if (text.isEmpty || _sending) return;
    setState(() => _sending = true);
    try {
      final msg = await _rooms.postMessage(_room.id, text);
      if (!mounted) return;
      setState(() {
        _messages = [..._messages, msg];
        _chatController.clear();
      });
    } catch (e) {
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('chat failed: $e')),
      );
    } finally {
      if (mounted) setState(() => _sending = false);
    }
  }

  Future<void> _sendGift(GiftItem gift) async {
    if (_sending || _roomEnded) return;
    setState(() => _sending = true);
    // One durable key per gift intent; reuse on retry until success.
    final clientRequestId = _giftRequestIds.putIfAbsent(
      gift.id,
      () => 'gift-${DateTime.now().microsecondsSinceEpoch}-${gift.id}',
    );
    try {
      await _gifts.sendGift(
        roomId: _room.id,
        giftId: gift.id,
        receiverId: _room.ownerId,
        clientRequestId: clientRequestId,
      );
      _giftRequestIds.remove(gift.id);
      final balance = await _gifts.walletBalance();
      if (!mounted) return;
      setState(() => _balance = balance);
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('Sent ${gift.name}')),
      );
    } catch (e) {
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('gift failed: $e')),
      );
    } finally {
      if (mounted) setState(() => _sending = false);
    }
  }

  Future<void> _showPublishInfo() async {
    if (_hostBusy) return;
    setState(() => _hostBusy = true);
    try {
      final info = await _rooms.publishInfo(_room.id);
      if (!mounted) return;
      setState(() => _publishInfo = info);
      final server = _obsServerBase(info.pushUrl, info.streamKey);
      await showDialog<void>(
        context: context,
        builder: (ctx) => AlertDialog(
          title: const Text('OBS publish'),
          content: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Text('Server', style: Theme.of(ctx).textTheme.labelLarge),
              SelectableText(server.isEmpty ? info.pushUrl : server),
              const SizedBox(height: 8),
              Text('Stream key', style: Theme.of(ctx).textTheme.labelLarge),
              SelectableText(info.streamKey),
            ],
          ),
          actions: [
            TextButton(
              onPressed: () {
                Clipboard.setData(ClipboardData(text: server.isEmpty ? info.pushUrl : server));
              },
              child: const Text('Copy server'),
            ),
            TextButton(
              onPressed: () {
                Clipboard.setData(ClipboardData(text: info.streamKey));
              },
              child: const Text('Copy key'),
            ),
            FilledButton(
              onPressed: () => Navigator.of(ctx).pop(),
              child: const Text('Close'),
            ),
          ],
        ),
      );
    } catch (e) {
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('publish info failed: $e')),
      );
    } finally {
      if (mounted) setState(() => _hostBusy = false);
    }
  }

  Future<void> _stopLive() async {
    if (_hostBusy || _roomEnded) return;
    setState(() => _hostBusy = true);
    try {
      final room = await _rooms.stopRoom(_room.id);
      if (!mounted) return;
      setState(() {
        _room = room;
        _hlsUrl = null;
      });
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('Live stopped')),
      );
    } catch (e) {
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('stop failed: $e')),
      );
    } finally {
      if (mounted) setState(() => _hostBusy = false);
    }
  }

  Future<void> _topup() async {
    try {
      final balance = await _gifts.topup(100);
      if (!mounted) return;
      setState(() => _balance = balance);
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('Topped up +100')),
      );
    } catch (e) {
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('topup failed: $e')),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: Text(_room.title),
        actions: [
          if (_room.isLive && !_roomEnded) ...[
            TextButton(
              onPressed: _hostBusy ? null : _showPublishInfo,
              child: const Text('OBS'),
            ),
            TextButton(
              onPressed: _hostBusy ? null : _stopLive,
              child: const Text('Stop'),
            ),
          ],
          TextButton(
            onPressed: _followBusy ? null : _toggleFollow,
            child: Text(_followingHost ? 'Unfollow' : 'Follow'),
          ),
          IconButton(
            onPressed: _reportRoom,
            icon: const Icon(Icons.flag_outlined),
            tooltip: 'Report room',
          ),
          IconButton(onPressed: _load, icon: const Icon(Icons.refresh)),
        ],
      ),
      body: _loading
          ? const Center(child: CircularProgressIndicator())
          : _error != null
              ? Center(child: Text(_error!))
              : Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    Expanded(
                      child: CustomScrollView(
                        slivers: [
                          SliverToBoxAdapter(
                            child: _RoomHeader(
                              room: _room,
                              hlsUrl: _roomEnded ? null : _hlsUrl,
                              balance: _balance,
                              onTopup: _topup,
                              roomEnded: _roomEnded,
                            ),
                          ),
                          const SliverToBoxAdapter(child: Divider(height: 1)),
                          if (_messages.isEmpty)
                            const SliverFillRemaining(
                              hasScrollBody: false,
                              child: Center(child: Text('No messages yet')),
                            )
                          else
                            SliverPadding(
                              padding: const EdgeInsets.symmetric(
                                horizontal: 12,
                                vertical: 8,
                              ),
                              sliver: SliverList(
                                delegate: SliverChildBuilderDelegate(
                                  (context, i) {
                                    final m = _messages[i];
                                    final name = m.senderName.isEmpty
                                        ? m.senderId
                                        : m.senderName;
                                    return Padding(
                                      padding: const EdgeInsets.symmetric(
                                        vertical: 4,
                                      ),
                                      child: Text(
                                        '$name: ${m.body}',
                                        style: Theme.of(context)
                                            .textTheme
                                            .bodyMedium,
                                      ),
                                    );
                                  },
                                  childCount: _messages.length,
                                ),
                              ),
                            ),
                        ],
                      ),
                    ),
                    _GiftBar(
                      gifts: _giftsCatalog,
                      onSend: _sendGift,
                      enabled: !_sending && !_roomEnded,
                    ),
                    _ChatInput(
                      controller: _chatController,
                      onSend: _sendChat,
                      enabled: !_sending && !_roomEnded,
                    ),
                  ],
                ),
    );
  }
}

/// Owns its [TextEditingController] so dispose is tied to dialog lifecycle
/// (avoids use-after-dispose while the route animates closed).
class _ReportRoomDialog extends StatefulWidget {
  const _ReportRoomDialog();

  @override
  State<_ReportRoomDialog> createState() => _ReportRoomDialogState();
}

class _ReportRoomDialogState extends State<_ReportRoomDialog> {
  final _reasonController = TextEditingController();

  @override
  void dispose() {
    _reasonController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: const Text('Report room'),
      content: TextField(
        controller: _reasonController,
        decoration: const InputDecoration(
          labelText: 'Reason',
          border: OutlineInputBorder(),
        ),
        maxLines: 3,
        autofocus: true,
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: const Text('Cancel'),
        ),
        FilledButton(
          onPressed: () =>
              Navigator.of(context).pop(_reasonController.text.trim()),
          child: const Text('Submit'),
        ),
      ],
    );
  }
}

class _RoomHeader extends StatelessWidget {
  const _RoomHeader({
    required this.room,
    required this.hlsUrl,
    required this.balance,
    required this.onTopup,
    required this.roomEnded,
  });

  final Room room;
  final String? hlsUrl;
  final int balance;
  final VoidCallback onTopup;
  final bool roomEnded;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.all(16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Expanded(
                child: Text(
                  room.title,
                  style: theme.textTheme.titleLarge,
                ),
              ),
              Chip(
                avatar: Icon(
                  Icons.circle,
                  size: 10,
                  color: room.isLive && !roomEnded ? Colors.red : Colors.grey,
                ),
                label: Text(room.status),
              ),
            ],
          ),
          const SizedBox(height: 8),
          Text('Owner: ${room.ownerId}', style: theme.textTheme.bodySmall),
          const SizedBox(height: 8),
          StreamPreview(
            status: room.status,
            hlsUrl: hlsUrl,
          ),
          const SizedBox(height: 12),
          Row(
            children: [
              Icon(Icons.account_balance_wallet,
                  size: 18, color: theme.colorScheme.primary),
              const SizedBox(width: 6),
              Text('Balance: $balance', style: theme.textTheme.titleSmall),
              const Spacer(),
              FilledButton.tonal(
                onPressed: onTopup,
                child: const Text('Top up'),
              ),
            ],
          ),
        ],
      ),
    );
  }
}

class _ChatList extends StatelessWidget {
  const _ChatList({required this.messages});

  final List<ChatMessage> messages;

  @override
  Widget build(BuildContext context) {
    if (messages.isEmpty) {
      return const Center(child: Text('No messages yet'));
    }
    return ListView.builder(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
      itemCount: messages.length,
      itemBuilder: (context, i) {
        final m = messages[i];
        final name = m.senderName.isEmpty ? m.senderId : m.senderName;
        return Padding(
          padding: const EdgeInsets.symmetric(vertical: 4),
          child: Text(
            '$name: ${m.body}',
            style: Theme.of(context).textTheme.bodyMedium,
          ),
        );
      },
    );
  }
}

class _GiftBar extends StatelessWidget {
  const _GiftBar({
    required this.gifts,
    required this.onSend,
    required this.enabled,
  });

  final List<GiftItem> gifts;
  final void Function(GiftItem) onSend;
  final bool enabled;

  @override
  Widget build(BuildContext context) {
    if (gifts.isEmpty) {
      return const SizedBox.shrink();
    }
    return SizedBox(
      height: 56,
      child: ListView.separated(
        scrollDirection: Axis.horizontal,
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
        itemCount: gifts.length,
        separatorBuilder: (_, _) => const SizedBox(width: 8),
        itemBuilder: (context, i) {
          final g = gifts[i];
          return ActionChip(
            label: Text('${g.name} (${g.price})'),
            onPressed: enabled ? () => onSend(g) : null,
          );
        },
      ),
    );
  }
}

class _ChatInput extends StatelessWidget {
  const _ChatInput({
    required this.controller,
    required this.onSend,
    required this.enabled,
  });

  final TextEditingController controller;
  final VoidCallback onSend;
  final bool enabled;

  @override
  Widget build(BuildContext context) {
    return SafeArea(
      top: false,
      child: Padding(
        padding: const EdgeInsets.fromLTRB(12, 4, 12, 12),
        child: Row(
          children: [
            Expanded(
              child: TextField(
                controller: controller,
                enabled: enabled,
                decoration: const InputDecoration(
                  hintText: 'Say something…',
                  border: OutlineInputBorder(),
                  isDense: true,
                ),
                onSubmitted: (_) => onSend(),
              ),
            ),
            const SizedBox(width: 8),
            IconButton.filled(
              onPressed: enabled ? onSend : null,
              icon: const Icon(Icons.send),
            ),
          ],
        ),
      ),
    );
  }
}

String _obsServerBase(String pushUrl, String streamKey) {
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
