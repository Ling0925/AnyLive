import 'package:flutter/material.dart';

import '../../api/api_client.dart';
import '../../api/gifts_repository.dart';
import '../../api/rooms_repository.dart';
import '../../config/app_config.dart';

/// Live room detail: title/status, HLS URL, chat, gifts, wallet.
class RoomPage extends StatefulWidget {
  const RoomPage({
    super.key,
    required this.config,
    required this.accessToken,
    required this.room,
    this.roomsRepository,
    this.giftsRepository,
  });

  final AppConfig config;
  final String accessToken;
  final Room room;

  /// Optional injectable repos for tests.
  final RoomsRepository? roomsRepository;
  final GiftsRepository? giftsRepository;

  @override
  State<RoomPage> createState() => _RoomPageState();
}

class _RoomPageState extends State<RoomPage> {
  late RoomsRepository _rooms;
  late GiftsRepository _gifts;
  late Room _room;

  final _chatController = TextEditingController();
  List<ChatMessage> _messages = [];
  List<GiftItem> _giftsCatalog = [];
  int _balance = 0;
  String? _hlsUrl;
  String? _error;
  bool _loading = true;
  bool _sending = false;

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
    _load();
  }

  @override
  void dispose() {
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

      String? hls;
      if (room.isLive) {
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
    if (_sending) return;
    setState(() => _sending = true);
    try {
      final clientRequestId =
          'gift-${DateTime.now().microsecondsSinceEpoch}-${gift.id}';
      await _gifts.sendGift(
        roomId: _room.id,
        giftId: gift.id,
        receiverId: _room.ownerId,
        clientRequestId: clientRequestId,
      );
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
                    _RoomHeader(
                      room: _room,
                      hlsUrl: _hlsUrl,
                      balance: _balance,
                      onTopup: _topup,
                    ),
                    const Divider(height: 1),
                    Expanded(child: _ChatList(messages: _messages)),
                    _GiftBar(
                      gifts: _giftsCatalog,
                      onSend: _sendGift,
                      enabled: !_sending,
                    ),
                    _ChatInput(
                      controller: _chatController,
                      onSend: _sendChat,
                      enabled: !_sending,
                    ),
                  ],
                ),
    );
  }
}

class _RoomHeader extends StatelessWidget {
  const _RoomHeader({
    required this.room,
    required this.hlsUrl,
    required this.balance,
    required this.onTopup,
  });

  final Room room;
  final String? hlsUrl;
  final int balance;
  final VoidCallback onTopup;

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
                  color: room.isLive ? Colors.red : Colors.grey,
                ),
                label: Text(room.status),
              ),
            ],
          ),
          const SizedBox(height: 8),
          Text('Owner: ${room.ownerId}', style: theme.textTheme.bodySmall),
          if (room.isLive && hlsUrl != null) ...[
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
                    hlsUrl!,
                    style: theme.textTheme.bodySmall,
                  ),
                  const SizedBox(height: 4),
                  Text(
                    '(player not embedded — open URL in external player)',
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: theme.colorScheme.onSurfaceVariant,
                    ),
                  ),
                ],
              ),
            ),
          ] else if (room.isLive) ...[
            const SizedBox(height: 8),
            Text(
              'Live — play URL unavailable',
              style: theme.textTheme.bodyMedium,
            ),
          ],
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
