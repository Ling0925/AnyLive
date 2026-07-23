import 'dart:async';
import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:http/http.dart' as http;

import '../../api/api_client.dart';
import '../../api/events_repository.dart';
import '../../api/gifts_repository.dart';
import '../../api/interactive_repository.dart';
import '../../api/reports_repository.dart';
import '../../api/rooms_repository.dart';
import '../../api/realtime_repository.dart';
import '../../api/social_repository.dart';
import '../../realtime/centrifugo_chat.dart';
import '../../config/app_config.dart';
import '../../player/hls_player_logic.dart';
import '../../player/stream_preview.dart';

/// Live room detail: title/status, HLS preview, chat, gifts, wallet, follow, report.
///
/// [StreamPreview] embeds media_kit for HLS on device; under `flutter test` the
/// native player stays off and a copyable URL + play placeholder remains.
class RoomPage extends StatefulWidget {
  const RoomPage({
    super.key,
    required this.config,
    required this.accessToken,
    required this.room,
    this.userId,
    this.roomsRepository,
    this.giftsRepository,
    this.socialRepository,
    this.reportsRepository,
    this.interactiveRepository,
    this.eventsRepository,
  });

  final AppConfig config;
  final String accessToken;
  final Room room;

  /// Current viewer id (for host-only start/stop/OBS). Optional for tests.
  final String? userId;

  /// Optional injectable repos for tests.
  final RoomsRepository? roomsRepository;
  final GiftsRepository? giftsRepository;
  final SocialRepository? socialRepository;
  final ReportsRepository? reportsRepository;
  final InteractiveRepository? interactiveRepository;
  final EventsRepository? eventsRepository;

  @override
  State<RoomPage> createState() => _RoomPageState();
}

class _RoomPageState extends State<RoomPage> {
  late RoomsRepository _rooms;
  late GiftsRepository _gifts;
  late SocialRepository _social;
  late ReportsRepository _reports;
  late InteractiveRepository _interactive;
  late EventsRepository _events;
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
  /// Soft-hide P3 menu entries when server feature flags are off (default).
  bool _featurePk = false;
  bool _featureCohost = false;
  PkSession? _pk;
  /// Durable gift idempotency keys keyed by gift id (reused until success).
  final Map<String, String> _giftRequestIds = {};
  Timer? _statusPoll;
  Timer? _chatPoll;
  Timer? _presencePoll;
  int _onlineCount = 0;
  int _likeCount = 0;
  String? _giftOverlay;
  bool _recordingEnabled = false;
  void Function()? _wsStop;
  late RealtimeRepository _realtime;

  /// Not watchable (idle after host stop, or terminal closed/ended).
  bool get _roomOffline => isRoomOfflineStatus(_room.status);

  /// Permanent end — force-close / closed. Host stop is idle (not terminal).
  bool get _roomTerminal => isRoomTerminalStatus(_room.status);

  /// Host-only controls when we know the viewer is the owner.
  bool get _isOwner {
    final uid = widget.userId?.trim() ?? '';
    return uid.isNotEmpty && uid == _room.ownerId;
  }

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
    _interactive =
        widget.interactiveRepository ?? InteractiveRepository(client: api);
    _events = widget.eventsRepository ?? EventsRepository(client: api);
    _realtime = RealtimeRepository(client: api);
    unawaited(_loadFeatureFlags(api));
    _load();
    _startStatusPoll();
    _startChatPoll();
    _startPresencePoll();
    unawaited(_tryConnectCentrifugo());
  }

  /// Best-effort: GET /api/v1/meta features.pk / features.cohost.
  /// Fail closed (menus stay hidden) so P1 dogfood never requires P3 flags.
  Future<void> _loadFeatureFlags(ApiClient api) async {
    try {
      final client = http.Client();
      try {
        final res = await client.get(
          api.uri('/api/v1/meta'),
          headers: api.jsonHeaders(auth: false),
        );
        if (res.statusCode != 200 || !mounted) return;
        final body = jsonDecode(res.body);
        if (body is! Map<String, dynamic>) return;
        final features = body['features'];
        if (features is! Map<String, dynamic>) return;
        setState(() {
          _featurePk = features['pk'] == true;
          _featureCohost = features['cohost'] == true;
        });
      } finally {
        client.close();
      }
    } catch (_) {
      // Keep P3 menus hidden when meta is unavailable.
    }
  }

  void _startPresencePoll() {
    _presencePoll?.cancel();
    _presencePoll = Timer.periodic(const Duration(seconds: 20), (_) async {
      // Presence only while live; idle keeps status poll for re-go-live.
      if (!mounted || _roomTerminal || !_room.isLive) {
        _presencePoll?.cancel();
        return;
      }
      try {
        final online = await _rooms.presenceHeartbeat(_room.id);
        final stats = await _rooms.roomStats(_room.id);
        if (!mounted) return;
        setState(() {
          _onlineCount = online;
          _likeCount = stats.likeCount;
          _recordingEnabled = stats.recordingEnabled;
        });
      } catch (_) {
        // ignore transient presence errors
      }
    });
  }

  void _startStatusPoll() {
    _statusPoll?.cancel();
    // Lightweight poll so stop/force-close/webhook flips ended UI without manual refresh.
    // Idle rooms keep polling so host go-live can attach HLS without leaving the page.
    _statusPoll = Timer.periodic(const Duration(seconds: 8), (_) async {
      if (!mounted || _roomTerminal) {
        _statusPoll?.cancel();
        return;
      }
      try {
        final room = await _rooms.getRoom(_room.id);
        if (!mounted) return;
        final statusChanged = room.status != _room.status;
        final becameLive = statusChanged && room.isLive && !_room.isLive;
        final leftLive = statusChanged && !room.isLive && _room.isLive;
        if (statusChanged) {
          setState(() {
            _room = room;
            if (leftLive) {
              _hlsUrl = null;
              _presencePoll?.cancel();
              if (isRoomTerminalStatus(room.status)) {
                _statusPoll?.cancel();
                _chatPoll?.cancel();
                _wsStop?.call();
                _wsStop = null;
              }
            }
          });
        }
        // Room may open idle then go live (host start / OBS publish): fetch HLS.
        if (becameLive || (room.isLive && (_hlsUrl == null || _hlsUrl!.isEmpty))) {
          if (becameLive) {
            _startPresencePoll();
          }
          try {
            final play = await _rooms.playUrls(room.id);
            final hls = play['hls'] as String?;
            if (mounted && hls != null && hls.isNotEmpty && hls != _hlsUrl) {
              setState(() => _hlsUrl = hls);
            }
          } catch (_) {
            // play URL may lag until stream is active
          }
        }
        try {
          final pk = await _interactive.getPk(_room.id);
          if (!mounted) return;
          final changed = pk?.id != _pk?.id ||
              pk?.scoreA != _pk?.scoreA ||
              pk?.scoreB != _pk?.scoreB ||
              pk?.status != _pk?.status;
          if (changed) {
            setState(() => _pk = pk);
          }
        } catch (_) {}
      } catch (_) {
        // ignore transient poll errors
      }
    });
  }


  Future<void> _tryConnectCentrifugo() async {
    final wsUrl = widget.config.normalizedCentrifugoWsUrl;
    if (wsUrl == null) return;
    try {
      final tok = await _realtime.connectionToken(_room.id);
      if (!mounted || tok.token.isEmpty) return;
      final channel = tok.channels.isNotEmpty
          ? tok.channels.first
          : 'room:${_room.id}';
      _wsStop?.call();
      _wsStop = connectCentrifugoChat(
        wsUrl: wsUrl,
        token: tok.token,
        channel: channel,
        onMessage: (msg) {
          if (!mounted || _roomTerminal) return;
          final id = msg['id'] ?? '';
          if (_messages.any((m) => m.id == id)) return;
          setState(() {
            _messages = [
              ..._messages,
              ChatMessage(
                id: id,
                roomId: _room.id,
                senderId: msg['senderId'] ?? '',
                senderName: msg['senderName'] ?? '',
                body: msg['body'] ?? '',
                createdAt: DateTime.now().toUtc().toIso8601String(),
              ),
            ];
          });
        },
      );
    } catch (_) {
      // Fall back to HTTP poll only.
    }
  }

  Future<void> _toggleRecording() async {
    setState(() => _hostBusy = true);
    try {
      final next = !_recordingEnabled;
      final st = await _rooms.setRecording(_room.id, next);
      if (!mounted) return;
      setState(() => _recordingEnabled = st.recordingEnabled);
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(
            st.recordingEnabled ? 'Recording enabled' : 'Recording disabled',
          ),
        ),
      );
    } catch (e) {
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('recording failed: $e')),
      );
    } finally {
      if (mounted) setState(() => _hostBusy = false);
    }
  }

  void _startChatPoll() {
    _chatPoll?.cancel();
    // HTTP history poll (fallback when Centrifugo WS is unavailable).
    _chatPoll = Timer.periodic(const Duration(seconds: 3), (_) async {
      // Keep history while idle (host stop); only terminal ends the poll.
      if (!mounted || _roomTerminal) {
        _chatPoll?.cancel();
        return;
      }
      try {
        final messages = await _rooms.listMessages(_room.id);
        if (!mounted) return;
        if (messages.length != _messages.length ||
            (messages.isNotEmpty &&
                _messages.isNotEmpty &&
                messages.last.id != _messages.last.id)) {
          setState(() => _messages = messages);
        }
      } catch (_) {
        // ignore transient poll errors
      }
    });
  }

  @override
  void dispose() {
    _statusPoll?.cancel();
    _chatPoll?.cancel();
    _presencePoll?.cancel();
    _wsStop?.call();
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
      if (room.isLive) {
        try {
          final play = await _rooms.playUrls(_room.id);
          hls = play['hls'] as String?;
        } catch (_) {
          // Room may flip status; show without HLS.
        }
      }

      PkSession? pk;
      try {
        pk = await _interactive.getPk(_room.id);
      } catch (_) {}

      if (!mounted) return;
      setState(() {
        _room = room;
        _messages = messages;
        _giftsCatalog = gifts;
        _balance = balance;
        _hlsUrl = hls;
        _followingHost = following;
        _pk = pk;
        _loading = false;
      });
      unawaited(_events.track(
        'room.view',
        props: {'room_id': room.id, 'status': room.status},
      ));
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
      unawaited(_events.track(
        'chat.send',
        props: {'room_id': _room.id},
      ));
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
    if (_sending || !_room.isLive) return;
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
      setState(() {
        _balance = balance;
        _giftOverlay = gift.name;
      });
      Future<void>.delayed(const Duration(milliseconds: 1800), () {
        if (mounted && _giftOverlay == gift.name) {
          setState(() => _giftOverlay = null);
        }
      });
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('Sent ${gift.name}')),
      );
      unawaited(_events.track(
        'gift.tap',
        props: {'room_id': _room.id, 'gift_id': gift.id},
      ));
      try {
        final pk = await _interactive.getPk(_room.id);
        if (mounted) setState(() => _pk = pk);
      } catch (_) {}
    } catch (e) {
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('gift failed: $e')),
      );
    } finally {
      if (mounted) setState(() => _sending = false);
    }
  }

  Future<void> _inviteCohost() async {
    final inviteeId = await showDialog<String>(
      context: context,
      builder: (ctx) => const _InviteCohostDialog(),
    );
    if (inviteeId == null || inviteeId.isEmpty || !mounted) return;
    try {
      final session = await _interactive.invite(
        roomId: _room.id,
        inviteeId: inviteeId,
      );
      unawaited(_events.track(
        'cohost.invite',
        props: {'room_id': _room.id, 'invitee_id': inviteeId},
      ));
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('Co-host invite sent (${session.status})')),
      );
    } catch (e) {
      if (!mounted) return;
      final msg = e is InteractiveException && e.statusCode == 403
          ? 'invite unavailable (feature off)'
          : 'invite failed: $e';
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(msg)),
      );
    }
  }

  Future<void> _respondCohost(bool accept) async {
    setState(() => _hostBusy = true);
    try {
      final session = await _interactive.respond(
        roomId: _room.id,
        accept: accept,
      );
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(
            accept
                ? 'Co-host accepted (${session.status})'
                : 'Co-host declined (${session.status})',
          ),
        ),
      );
    } catch (e) {
      if (!mounted) return;
      final msg = e is InteractiveException && e.statusCode == 403
          ? 'co-host unavailable (feature off)'
          : 'respond failed: $e';
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(msg)),
      );
    } finally {
      if (mounted) setState(() => _hostBusy = false);
    }
  }

  Future<void> _showLivekitJoin() async {
    try {
      final info = await _interactive.livekitJoin(_room.id, role: 'viewer');
      if (!mounted) return;
      await showDialog<void>(
        context: context,
        builder: (ctx) => AlertDialog(
          title: const Text('LiveKit join'),
          content: SingleChildScrollView(
            child: SelectableText(
              'url: ${info.url}\n'
              'room: ${info.roomName}\n'
              'identity: ${info.identity ?? "-"}\n'
              'token: ${info.token}',
            ),
          ),
          actions: [
            TextButton(
              onPressed: () {
                Clipboard.setData(ClipboardData(text: info.token));
                Navigator.of(ctx).pop();
              },
              child: const Text('Copy token'),
            ),
            TextButton(
              onPressed: () => Navigator.of(ctx).pop(),
              child: const Text('Close'),
            ),
          ],
        ),
      );
    } catch (e) {
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('LiveKit join failed: $e')),
      );
    }
  }

  Future<void> _startPkDialog() async {
    final opponent = await showDialog<String>(
      context: context,
      builder: (ctx) => const _StartPkDialog(),
    );
    if (opponent == null || opponent.isEmpty || !mounted) return;
    try {
      final pk = await _interactive.startPk(
        roomId: _room.id,
        opponentRoomId: opponent,
      );
      unawaited(_events.track(
        'pk.start',
        props: {'room_id': _room.id, 'opponent_room_id': opponent},
      ));
      if (!mounted) return;
      setState(() => _pk = pk);
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('PK started')),
      );
    } catch (e) {
      if (!mounted) return;
      final msg = e is InteractiveException && e.statusCode == 403
          ? 'PK unavailable (feature off)'
          : 'PK start failed: $e';
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(msg)),
      );
    }
  }

  Future<void> _endPk() async {
    try {
      final pk = await _interactive.endPk(_room.id);
      unawaited(_events.track(
        'pk.end',
        props: {
          'room_id': _room.id,
          if (pk.winnerRoomId != null) 'winner_room_id': pk.winnerRoomId!,
        },
      ));
      if (!mounted) return;
      setState(() => _pk = pk);
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(
            pk.winnerRoomId == null
                ? 'PK ended'
                : 'PK ended · winner ${pk.winnerRoomId}',
          ),
        ),
      );
    } catch (e) {
      if (!mounted) return;
      final msg = e is InteractiveException && e.statusCode == 403
          ? 'PK unavailable (feature off)'
          : 'PK end failed: $e';
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(msg)),
      );
    }
  }

  Future<void> _showPublishInfo() async {
    if (_hostBusy) return;
    setState(() => _hostBusy = true);
    try {
      final info = await _rooms.publishInfo(_room.id);
      if (!mounted) return;
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

  Future<void> _startLive() async {
    if (_hostBusy || _roomTerminal || !_isOwner || _room.isLive) return;
    setState(() => _hostBusy = true);
    try {
      final room = await _rooms.startRoom(_room.id);
      if (!mounted) return;
      setState(() => _room = room);
      _startPresencePoll();
      try {
        final play = await _rooms.playUrls(_room.id);
        final hls = play['hls'] as String?;
        if (mounted && hls != null && hls.isNotEmpty) {
          setState(() => _hlsUrl = hls);
        }
      } catch (_) {
        // HLS appears after OBS publish; host can open OBS dialog next.
      }
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(
          content: Text('Live started — use OBS publish for stream key'),
        ),
      );
    } catch (e) {
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('start failed: $e')),
      );
    } finally {
      if (mounted) setState(() => _hostBusy = false);
    }
  }

  Future<void> _stopLive() async {
    if (_hostBusy || !_room.isLive || !_isOwner) return;
    setState(() => _hostBusy = true);
    try {
      final room = await _rooms.stopRoom(_room.id);
      if (!mounted) return;
      setState(() {
        _room = room;
        _hlsUrl = null;
      });
      _presencePoll?.cancel();
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('Live stopped — room idle (not closed)')),
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
          TextButton(
            key: const Key('room-like'),
            onPressed: !_room.isLive
                ? null
                : () async {
                    try {
                      final r = await _rooms.likeRoom(_room.id);
                      if (!mounted) return;
                      setState(() => _likeCount = r.likeCount);
                    } catch (e) {
                      if (!mounted) return;
                      ScaffoldMessenger.of(context).showSnackBar(
                        SnackBar(content: Text('like failed: $e')),
                      );
                    }
                  },
            child: Text('♥ $_likeCount'),
          ),
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 4),
            child: Center(
              child: Text(
                '$_onlineCount online',
                key: const Key('room-online'),
                style: Theme.of(context).textTheme.bodySmall,
              ),
            ),
          ),
          TextButton(
            onPressed: _followBusy ? null : _toggleFollow,
            child: Text(_followingHost ? 'Unfollow' : 'Follow'),
          ),
          if (_isOwner && !_roomTerminal)
            PopupMenuButton<String>(
              key: const Key('room-host-menu'),
              enabled: !_hostBusy,
              onSelected: (value) {
                switch (value) {
                  case 'start':
                    _startLive();
                  case 'cohost-invite':
                    _inviteCohost();
                  case 'cohost-accept':
                    _respondCohost(true);
                  case 'cohost-decline':
                    _respondCohost(false);
                  case 'pk-start':
                    _startPkDialog();
                  case 'pk-end':
                    _endPk();
                  case 'livekit-join':
                    _showLivekitJoin();
                  case 'obs':
                    _showPublishInfo();
                  case 'stop':
                    _stopLive();
                  case 'recording':
                    _toggleRecording();
                }
              },
              itemBuilder: (ctx) {
                final items = <PopupMenuEntry<String>>[];
                if (!_room.isLive) {
                  items.add(
                    const PopupMenuItem(
                      key: Key('host-start-live'),
                      value: 'start',
                      child: Text('Start live'),
                    ),
                  );
                }
                if (_room.isLive) {
                  if (_featureCohost) {
                    items.addAll(const [
                      PopupMenuItem(
                        key: Key('cohost-invite'),
                        value: 'cohost-invite',
                        child: Text('Invite co-host'),
                      ),
                      PopupMenuItem(
                        key: Key('cohost-accept'),
                        value: 'cohost-accept',
                        child: Text('Accept co-host'),
                      ),
                      PopupMenuItem(
                        key: Key('cohost-decline'),
                        value: 'cohost-decline',
                        child: Text('Decline co-host'),
                      ),
                    ]);
                  }
                  if (_featurePk) {
                    if (_pk == null || !_pk!.isActive) {
                      items.add(
                        const PopupMenuItem(
                          key: Key('pk-start'),
                          value: 'pk-start',
                          child: Text('Start PK'),
                        ),
                      );
                    } else {
                      items.add(
                        const PopupMenuItem(
                          key: Key('pk-end'),
                          value: 'pk-end',
                          child: Text('End PK'),
                        ),
                      );
                    }
                  }
                  if (_featureCohost || _featurePk) {
                    items.add(
                      const PopupMenuItem(
                        key: Key('livekit-join'),
                        value: 'livekit-join',
                        child: Text('LiveKit join'),
                      ),
                    );
                  }
                  items.add(
                    const PopupMenuItem(
                      value: 'obs',
                      child: Text('OBS publish'),
                    ),
                  );
                  items.add(
                    PopupMenuItem(
                      key: const Key('recording-toggle'),
                      value: 'recording',
                      child: Text(
                        _recordingEnabled
                            ? 'Disable recording'
                            : 'Enable recording',
                      ),
                    ),
                  );
                  items.add(
                    const PopupMenuItem(
                      key: Key('host-stop-live'),
                      value: 'stop',
                      child: Text('Stop live'),
                    ),
                  );
                }
                return items;
              },
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
                              hlsUrl: _room.isLive ? _hlsUrl : null,
                              balance: _balance,
                              onTopup: _topup,
                              roomOffline: _roomOffline,
                              roomTerminal: _roomTerminal,
                              pk: _pk,
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
                      enabled: !_sending && _room.isLive,
                    ),
                    if (_giftOverlay != null)
                      IgnorePointer(
                        child: Align(
                          alignment: Alignment.center,
                          child: AnimatedOpacity(
                            opacity: _giftOverlay == null ? 0 : 1,
                            duration: const Duration(milliseconds: 200),
                            child: Container(
                              key: const Key('gift-overlay'),
                              padding: const EdgeInsets.symmetric(
                                horizontal: 20,
                                vertical: 12,
                              ),
                              decoration: BoxDecoration(
                                color: Colors.pink.withValues(alpha: 0.85),
                                borderRadius: BorderRadius.circular(16),
                              ),
                              child: Text(
                                '🎁 $_giftOverlay',
                                style: const TextStyle(
                                  color: Colors.white,
                                  fontSize: 22,
                                  fontWeight: FontWeight.bold,
                                ),
                              ),
                            ),
                          ),
                        ),
                      ),
                    _ChatInput(
                      controller: _chatController,
                      onSend: _sendChat,
                      // Chat remains available while idle; gifts/like stay live-only.
                      enabled: !_sending && !_roomTerminal,
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

class _InviteCohostDialog extends StatefulWidget {
  const _InviteCohostDialog();

  @override
  State<_InviteCohostDialog> createState() => _InviteCohostDialogState();
}

class _InviteCohostDialogState extends State<_InviteCohostDialog> {
  final _controller = TextEditingController();

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: const Text('Invite co-host'),
      content: TextField(
        key: const Key('cohost-invitee-id'),
        controller: _controller,
        decoration: const InputDecoration(
          labelText: 'Invitee user id (UUID)',
          border: OutlineInputBorder(),
        ),
        autofocus: true,
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: const Text('Cancel'),
        ),
        FilledButton(
          onPressed: () => Navigator.of(context).pop(_controller.text.trim()),
          child: const Text('Invite'),
        ),
      ],
    );
  }
}

class _StartPkDialog extends StatefulWidget {
  const _StartPkDialog();

  @override
  State<_StartPkDialog> createState() => _StartPkDialogState();
}

class _StartPkDialogState extends State<_StartPkDialog> {
  final _controller = TextEditingController();

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: const Text('Start PK'),
      content: TextField(
        key: const Key('pk-opponent-room-id'),
        controller: _controller,
        decoration: const InputDecoration(
          labelText: 'Opponent room id (UUID)',
          border: OutlineInputBorder(),
        ),
        autofocus: true,
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: const Text('Cancel'),
        ),
        FilledButton(
          onPressed: () => Navigator.of(context).pop(_controller.text.trim()),
          child: const Text('Start'),
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
    required this.roomOffline,
    required this.roomTerminal,
    this.pk,
  });

  final Room room;
  final String? hlsUrl;
  final int balance;
  final VoidCallback onTopup;
  /// Not watchable (idle or closed/ended).
  final bool roomOffline;
  /// Permanent end (closed/ended). Idle host-stop is offline but not terminal.
  final bool roomTerminal;
  final PkSession? pk;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final statusLabel = roomTerminal
        ? '${room.status} · ended'
        : room.status == 'idle'
            ? 'idle · offline'
            : room.status;
    return Padding(
      padding: const EdgeInsets.all(16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          if (pk != null && (pk!.isActive || pk!.isEnded)) ...[
            Card(
              key: const Key('pk-banner'),
              color: theme.colorScheme.secondaryContainer,
              child: Padding(
                padding: const EdgeInsets.all(12),
                child: Row(
                  children: [
                    Icon(
                      Icons.flash_on,
                      color: theme.colorScheme.onSecondaryContainer,
                    ),
                    const SizedBox(width: 8),
                    Expanded(
                      child: Text(
                        'PK ${pk!.status}: ${pk!.scoreA} – ${pk!.scoreB}'
                        '${pk!.winnerRoomId != null ? ' · win ${pk!.winnerRoomId}' : ''}',
                        key: const Key('pk-score'),
                        style: theme.textTheme.titleSmall,
                      ),
                    ),
                  ],
                ),
              ),
            ),
            const SizedBox(height: 8),
          ],
          Row(
            children: [
              Expanded(
                child: Text(
                  room.title,
                  style: theme.textTheme.titleLarge,
                ),
              ),
              Chip(
                key: Key(roomTerminal
                    ? 'room-status-terminal'
                    : roomOffline
                        ? 'room-status-offline'
                        : 'room-status-live'),
                avatar: Icon(
                  Icons.circle,
                  size: 10,
                  color: room.isLive && !roomOffline ? Colors.red : Colors.grey,
                ),
                label: Text(statusLabel),
              ),
            ],
          ),
          const SizedBox(height: 8),
          Text('Owner: ${room.ownerId}', style: theme.textTheme.bodySmall),
          const SizedBox(height: 8),
          StreamPreview(
            status: room.status,
            hlsUrl: hlsUrl,
            // Widget tests inject enableEmbeddedPlayer:false via default env;
            // production/device leaves null → media_kit when not under flutter_test.
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
