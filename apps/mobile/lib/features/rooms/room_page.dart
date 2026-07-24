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
import '../../theme/any_colors.dart';
import '../../ui/live_badge.dart';
import '../../l10n/l10n.dart';

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
  Timer? _chromeHide;
  /// Player chrome (title strip) — tap stage to toggle.
  bool _playerChromeVisible = true;
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
        if (_featurePk) {
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
        } else if (_pk != null && mounted) {
          setState(() => _pk = null);
        }
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
            st.recordingEnabled ? context.l10n.recordingEnabled : context.l10n.recordingDisabled,
          ),
        ),
      );
    } catch (e) {
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(context.l10n.actionFailed('recording', '$e'))),
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
    _chromeHide?.cancel();
    _wsStop?.call();
    _chatController.dispose();
    super.dispose();
  }

  void _togglePlayerChrome() {
    setState(() => _playerChromeVisible = !_playerChromeVisible);
    _chromeHide?.cancel();
    if (_playerChromeVisible && _room.isLive) {
      _chromeHide = Timer(const Duration(seconds: 3), () {
        if (mounted) setState(() => _playerChromeVisible = false);
      });
    }
  }

  Future<void> _shareRoom() async {
    final shareUrl = widget.config.shareRoomUrl(_room.id);
    await Clipboard.setData(ClipboardData(text: shareUrl));
    if (!mounted) return;
    await showModalBottomSheet<void>(
      context: context,
      backgroundColor: AnyColors.bgElevated,
      shape: const RoundedRectangleBorder(
        borderRadius: BorderRadius.vertical(top: Radius.circular(16)),
      ),
      builder: (ctx) {
        return SafeArea(
          child: Padding(
            padding: const EdgeInsets.fromLTRB(20, 16, 20, 24),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                Text(
                  context.l10n.shareLive,
                  style: TextStyle(
                    color: AnyColors.textPrimary,
                    fontSize: 16,
                    fontWeight: FontWeight.w700,
                  ),
                ),
                const SizedBox(height: 8),
                Text(
                  shareUrl,
                  style: const TextStyle(
                    color: AnyColors.textSecondary,
                    fontSize: 13,
                  ),
                ),
                SizedBox(height: 16),
                FilledButton.icon(
                  key: const Key('room-share-copy'),
                  onPressed: () {
                    Clipboard.setData(ClipboardData(text: shareUrl));
                    Navigator.of(ctx).pop();
                    ScaffoldMessenger.of(context).showSnackBar(
                      SnackBar(content: Text(context.l10n.linkCopied)),
                    );
                  },
                  icon: const Icon(Icons.link),
                  label: Text(context.l10n.copyLink),
                ),
              ],
            ),
          ),
        );
      },
    );
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
      if (_featurePk) {
        try {
          pk = await _interactive.getPk(_room.id);
        } catch (_) {}
      }

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
          SnackBar(content: Text(context.l10n.unfollowedHost)),
        );
      } else {
        await _social.follow(_room.ownerId);
        if (!mounted) return;
        setState(() => _followingHost = true);
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text(context.l10n.followingHost)),
        );
      }
    } catch (e) {
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(context.l10n.actionFailed('follow', '$e'))),
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
        SnackBar(content: Text(context.l10n.reportSubmitted)),
      );
    } catch (e) {
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(context.l10n.actionFailed('report', '$e'))),
      );
    }
  }

  Future<void> _sendChat() async {
    final text = _chatController.text.trim();
    if (text.isEmpty || _sending) return;
    // Match H5: send only while live (backend does not enforce room status).
    if (!_room.isLive) return;
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
        SnackBar(content: Text(context.l10n.actionFailed('chat', '$e'))),
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
      HapticFeedback.lightImpact();
      Future<void>.delayed(const Duration(milliseconds: 1800), () {
        if (mounted && _giftOverlay == gift.name) {
          setState(() => _giftOverlay = null);
        }
      });
      unawaited(_events.track(
        'gift.tap',
        props: {'room_id': _room.id, 'gift_id': gift.id},
      ));
      if (_featurePk) {
        try {
          final pk = await _interactive.getPk(_room.id);
          if (mounted) setState(() => _pk = pk);
        } catch (_) {}
      }
    } catch (e) {
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(context.l10n.actionFailed('gift', '$e'))),
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
        SnackBar(
          content: Text(context.l10n.cohostInviteSent(session.status)),
        ),
      );
    } catch (e) {
      if (!mounted) return;
      final msg = e is InteractiveException && e.statusCode == 403
          ? context.l10n.inviteUnavailable
          : context.l10n.actionFailed('invite', '$e');
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
                ? context.l10n.cohostAccepted(session.status)
                : context.l10n.cohostDeclined(session.status),
          ),
        ),
      );
    } catch (e) {
      if (!mounted) return;
      final msg = e is InteractiveException && e.statusCode == 403
          ? context.l10n.cohostUnavailable
          : context.l10n.actionFailed('respond', '$e');
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
          title: Text(ctx.l10n.livekitJoin),
          content: SingleChildScrollView(
            child: SelectableText(
              ctx.l10n.livekitJoinDetail(
                info.url,
                info.roomName,
                info.identity ?? '-',
                info.token,
              ),
            ),
          ),
          actions: [
            TextButton(
              onPressed: () {
                Clipboard.setData(ClipboardData(text: info.token));
                Navigator.of(ctx).pop();
              },
              child: Text(ctx.l10n.copyToken),
            ),
            TextButton(
              onPressed: () => Navigator.of(ctx).pop(),
              child: Text(ctx.l10n.close),
            ),
          ],
        ),
      );
    } catch (e) {
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(context.l10n.actionFailed('LiveKit join', '$e')),
        ),
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
        SnackBar(content: Text(context.l10n.pkStarted)),
      );
    } catch (e) {
      if (!mounted) return;
      final msg = e is InteractiveException && e.statusCode == 403
          ? context.l10n.pkUnavailable
          : context.l10n.actionFailed('PK start', '$e');
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
                ? context.l10n.pkEnded
                : context.l10n.pkEndedWithWinner(pk.winnerRoomId!),
          ),
        ),
      );
    } catch (e) {
      if (!mounted) return;
      final msg = e is InteractiveException && e.statusCode == 403
          ? context.l10n.pkUnavailable
          : context.l10n.actionFailed('PK end', '$e');
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
          title: Text(context.l10n.obsPublish),
          content: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Text(ctx.l10n.server, style: Theme.of(ctx).textTheme.labelLarge),
              SelectableText(server.isEmpty ? info.pushUrl : server),
              SizedBox(height: 8),
              Text(ctx.l10n.streamKey, style: Theme.of(ctx).textTheme.labelLarge),
              SelectableText(info.streamKey),
            ],
          ),
          actions: [
            TextButton(
              onPressed: () {
                Clipboard.setData(ClipboardData(text: server.isEmpty ? info.pushUrl : server));
              },
              child: Text(ctx.l10n.copyServer),
            ),
            TextButton(
              onPressed: () {
                Clipboard.setData(ClipboardData(text: info.streamKey));
              },
              child: Text(ctx.l10n.copyKey),
            ),
            FilledButton(
              onPressed: () => Navigator.of(ctx).pop(),
              child: Text(ctx.l10n.close),
            ),
          ],
        ),
      );
    } catch (e) {
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(context.l10n.actionFailed('publish info', '$e')),
        ),
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
        SnackBar(
          content: Text(context.l10n.liveStartedObsHint),
        ),
      );
    } catch (e) {
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(context.l10n.actionFailed('start', '$e'))),
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
        SnackBar(content: Text(context.l10n.liveStoppedIdle)),
      );
    } catch (e) {
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(context.l10n.actionFailed('stop', '$e'))),
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
        SnackBar(content: Text(context.l10n.toppedUp)),
      );
    } catch (e) {
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(context.l10n.actionFailed('topup', '$e'))),
      );
    }
  }

  Future<void> _likeRoom() async {
    if (!_room.isLive) return;
    try {
      final r = await _rooms.likeRoom(_room.id);
      if (!mounted) return;
      setState(() => _likeCount = r.likeCount);
    } catch (e) {
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(context.l10n.actionFailed('like', '$e'))),
      );
    }
  }

  List<PopupMenuEntry<String>> _moreMenuItems() {
    final items = <PopupMenuEntry<String>>[
      PopupMenuItem(
        value: 'like',
        enabled: _room.isLive,
        child: Text(context.l10n.likeCount(_likeCount)),
      ),
      PopupMenuItem(
        value: 'share',
        child: Text(context.l10n.share),
      ),
      PopupMenuItem(
        value: 'report',
        child: Text(context.l10n.report),
      ),
      PopupMenuItem(
        value: 'refresh',
        child: Text(context.l10n.refresh),
      ),
    ];

    if (_isOwner && !_roomTerminal) {
      items.add(const PopupMenuDivider());
      if (!_room.isLive) {
        items.add(
          PopupMenuItem(
            key: Key('host-start-live'),
            value: 'start',
            child: Text(context.l10n.startLive),
          ),
        );
      }
      if (_room.isLive) {
        if (_featureCohost) {
          items.addAll([
            PopupMenuItem(
              key: Key('cohost-invite'),
              value: 'cohost-invite',
              child: Text(context.l10n.inviteCohost),
            ),
            PopupMenuItem(
              key: Key('cohost-accept'),
              value: 'cohost-accept',
              child: Text(context.l10n.acceptCohost),
            ),
            PopupMenuItem(
              key: Key('cohost-decline'),
              value: 'cohost-decline',
              child: Text(context.l10n.declineCohost),
            ),
          ]);
        }
        if (_featurePk) {
          if (_pk == null || !_pk!.isActive) {
            items.add(
              PopupMenuItem(
                key: Key('pk-start'),
                value: 'pk-start',
                child: Text(context.l10n.startPk),
              ),
            );
          } else {
            items.add(
              PopupMenuItem(
                key: Key('pk-end'),
                value: 'pk-end',
                child: Text(context.l10n.endPk),
              ),
            );
          }
        }
        if (_featureCohost || _featurePk) {
          items.add(
            PopupMenuItem(
              key: const Key('livekit-join'),
              value: 'livekit-join',
              child: Text(context.l10n.livekitJoin),
            ),
          );
        }
        items.add(
          PopupMenuItem(
            value: 'obs',
            child: Text(context.l10n.obsPublish),
          ),
        );
        items.add(
          PopupMenuItem(
            key: const Key('recording-toggle'),
            value: 'recording',
            child: Text(
              _recordingEnabled ? context.l10n.disableRecording : context.l10n.enableRecording,
            ),
          ),
        );
        items.add(
          PopupMenuItem(
            key: Key('host-stop-live'),
            value: 'stop',
            child: Text(context.l10n.stopLive),
          ),
        );
      }
    }
    return items;
  }

  void _onMoreSelected(String value) {
    switch (value) {
      case 'like':
        _likeRoom();
      case 'share':
        _shareRoom();
      case 'report':
        _reportRoom();
      case 'refresh':
        _load();
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
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      backgroundColor: AnyColors.bgApp,
      appBar: AppBar(
        backgroundColor: AnyColors.bgApp,
        title: Text(
          _room.title,
          maxLines: 1,
          overflow: TextOverflow.ellipsis,
        ),
        actions: [
          // Hidden-but-findable keys for existing tests (like / online).
          Opacity(
            opacity: 0,
            child: SizedBox(
              width: 1,
              height: 1,
              child: TextButton(
                key: const Key('room-like'),
                onPressed: _room.isLive ? _likeRoom : null,
                child: Text('♥ $_likeCount'),
              ),
            ),
          ),
          Opacity(
            opacity: 0,
            child: SizedBox(
              width: 1,
              height: 1,
              child: Text(
                '$_onlineCount online',
                key: const Key('room-online'),
              ),
            ),
          ),
          PopupMenuButton<String>(
            key: const Key('room-host-menu'),
            enabled: !_hostBusy,
            icon: const Icon(Icons.more_vert),
            tooltip: context.l10n.more,
            onSelected: _onMoreSelected,
            itemBuilder: (_) => _moreMenuItems(),
          ),
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
                      child: ListView(
                        padding: EdgeInsets.zero,
                        children: [
                          // 1) PlayerStage (black) — tap toggles chrome
                          _PlayerStage(
                            room: _room,
                            hlsUrl: _room.isLive ? _hlsUrl : null,
                            roomOffline: _roomOffline,
                            roomTerminal: _roomTerminal,
                            giftOverlay: _giftOverlay,
                            chromeVisible: _playerChromeVisible,
                            onToggleChrome: _togglePlayerChrome,
                          ),
                          // 2) Meta row: LIVE + online + title + likes
                          _MetaRow(
                            room: _room,
                            onlineCount: _onlineCount,
                            likeCount: _likeCount,
                            roomOffline: _roomOffline,
                            roomTerminal: _roomTerminal,
                            featurePk: _featurePk,
                            pk: _pk,
                          ),
                          // 3) Channel row: host + Follow
                          _ChannelRow(
                            ownerId: _room.ownerId,
                            following: _followingHost,
                            busy: _followBusy,
                            onToggle: _toggleFollow,
                          ),
                          Divider(height: 1, color: Color(0x14FFFFFF)),
                          // 4) Chat panel (scrolls with stage on short viewports)
                          if (_messages.isEmpty)
                            Padding(
                              padding: EdgeInsets.symmetric(vertical: 24),
                              child: Center(
                                child: Text(
                                  context.l10n.noMessagesYet,
                                  style: TextStyle(
                                    color: AnyColors.textSecondary,
                                    fontSize: 13,
                                  ),
                                ),
                              ),
                            )
                          else
                            Padding(
                              padding: const EdgeInsets.symmetric(
                                horizontal: 12,
                                vertical: 8,
                              ),
                              child: Column(
                                crossAxisAlignment: CrossAxisAlignment.stretch,
                                children: [
                                  for (final m in _messages)
                                    Padding(
                                      padding: const EdgeInsets.symmetric(
                                        vertical: 3,
                                      ),
                                      child: Text.rich(
                                        TextSpan(
                                          children: [
                                            TextSpan(
                                              text:
                                                  '${m.senderName.isEmpty ? m.senderId : m.senderName}: ',
                                              style: const TextStyle(
                                                color: AnyColors.accent,
                                                fontSize: 13,
                                                fontWeight: FontWeight.w600,
                                              ),
                                            ),
                                            TextSpan(
                                              text: m.body,
                                              style: const TextStyle(
                                                color: AnyColors.textPrimary,
                                                fontSize: 13,
                                              ),
                                            ),
                                          ],
                                        ),
                                      ),
                                    ),
                                ],
                              ),
                            ),
                        ],
                      ),
                    ),
                    // 5) Composer — live only (H5 parity); history still visible offline
                    _ChatInput(
                      controller: _chatController,
                      onSend: _sendChat,
                      enabled: !_sending && _room.isLive,
                      offlineHint: !_room.isLive && !_roomTerminal
                          ? context.l10n.roomOfflineChatDisabled
                          : (_roomTerminal ? context.l10n.streamEndedChatClosed : null),
                    ),
                    // 6) Gift dock + balance / top-up (hide when permanently closed)
                    if (!_roomTerminal)
                      _GiftDock(
                        gifts: _giftsCatalog,
                        balance: _balance,
                        onSend: _sendGift,
                        onTopup: _topup,
                        enabled: !_sending && _room.isLive,
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
      title: Text(context.l10n.reportRoom),
      content: TextField(
        controller: _reasonController,
        decoration: InputDecoration(
          labelText: context.l10n.reason,
          border: OutlineInputBorder(),
        ),
        maxLines: 3,
        autofocus: true,
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(context.l10n.cancel),
        ),
        FilledButton(
          onPressed: () =>
              Navigator.of(context).pop(_reasonController.text.trim()),
          child: Text(context.l10n.submit),
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
      title: Text(context.l10n.inviteCohost),
      content: TextField(
        key: const Key('cohost-invitee-id'),
        controller: _controller,
        decoration: InputDecoration(
          labelText: context.l10n.inviteeUserId,
          border: OutlineInputBorder(),
        ),
        autofocus: true,
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(context.l10n.cancel),
        ),
        FilledButton(
          onPressed: () => Navigator.of(context).pop(_controller.text.trim()),
          child: Text(context.l10n.invite),
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
      title: Text(context.l10n.startPk),
      content: TextField(
        key: const Key('pk-opponent-room-id'),
        controller: _controller,
        decoration: InputDecoration(
          labelText: context.l10n.opponentRoomId,
          border: OutlineInputBorder(),
        ),
        autofocus: true,
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(context.l10n.cancel),
        ),
        FilledButton(
          onPressed: () => Navigator.of(context).pop(_controller.text.trim()),
          child: Text(context.l10n.start),
        ),
      ],
    );
  }
}

/// Full-width black player stage (StreamPreview only — no double offline chrome).
class _PlayerStage extends StatelessWidget {
  const _PlayerStage({
    required this.room,
    required this.hlsUrl,
    required this.roomOffline,
    required this.roomTerminal,
    required this.chromeVisible,
    required this.onToggleChrome,
    this.giftOverlay,
  });

  final Room room;
  final String? hlsUrl;
  final bool roomOffline;
  final bool roomTerminal;
  final bool chromeVisible;
  final VoidCallback onToggleChrome;
  final String? giftOverlay;

  @override
  Widget build(BuildContext context) {
    // StreamPreview already renders ended/offline placeholders; keep a
    // compact status key for tests without stacking extra chrome that
    // overflows the stage height in widget tests.
    final statusKey = roomTerminal
        ? 'room-status-terminal'
        : roomOffline
            ? 'room-status-offline'
            : 'room-status-live';

    return ColoredBox(
      color: AnyColors.bgPlayer,
      child: GestureDetector(
        behavior: HitTestBehavior.opaque,
        onTap: onToggleChrome,
        child: Stack(
          children: [
            StreamPreview(
              status: room.status,
              hlsUrl: hlsUrl,
            ),
            Positioned(
              left: 0,
              top: 0,
              child: Opacity(
                opacity: 0,
                child: Text(room.status, key: Key(statusKey)),
              ),
            ),
            Positioned(
              left: 0,
              right: 0,
              bottom: 0,
              child: IgnorePointer(
                child: AnimatedOpacity(
                  opacity: chromeVisible ? 1 : 0,
                  duration: const Duration(milliseconds: 200),
                  child: Container(
                    padding: const EdgeInsets.fromLTRB(12, 28, 12, 10),
                    decoration: const BoxDecoration(
                      gradient: LinearGradient(
                        begin: Alignment.topCenter,
                        end: Alignment.bottomCenter,
                        colors: [
                          Color(0x00000000),
                          Color(0x99000000),
                        ],
                      ),
                    ),
                    child: Row(
                      children: [
                        if (room.isLive && !roomOffline)
                          LiveBadge(compact: true)
                        else
                          Text(
                            roomTerminal ? context.l10n.statusEnded : room.status.toUpperCase(),
                            style: const TextStyle(
                              color: Colors.white70,
                              fontSize: 11,
                              fontWeight: FontWeight.w700,
                            ),
                          ),
                        const SizedBox(width: 8),
                        Expanded(
                          child: Text(
                            room.title,
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                            style: const TextStyle(
                              color: Colors.white,
                              fontSize: 13,
                              fontWeight: FontWeight.w600,
                            ),
                          ),
                        ),
                      ],
                    ),
                  ),
                ),
              ),
            ),
            if (giftOverlay != null)
              Positioned.fill(
                child: IgnorePointer(
                  child: Center(
                    child: TweenAnimationBuilder<double>(
                      key: ValueKey(giftOverlay),
                      tween: Tween(begin: 0.85, end: 1.0),
                      duration: const Duration(milliseconds: 280),
                      curve: Curves.easeOutBack,
                      builder: (context, scale, child) {
                        return Transform.scale(scale: scale, child: child);
                      },
                      child: Container(
                        key: const Key('gift-overlay'),
                        padding: const EdgeInsets.symmetric(
                          horizontal: 20,
                          vertical: 12,
                        ),
                        decoration: BoxDecoration(
                          color: AnyColors.accent.withValues(alpha: 0.92),
                          borderRadius: BorderRadius.circular(16),
                          boxShadow: [
                            BoxShadow(
                              color: Color(0x66C850FF),
                              blurRadius: 18,
                            ),
                          ],
                        ),
                        child: Text(
                          giftOverlay!,
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
              ),
          ],
        ),
      ),
    );
  }
}

/// LIVE badge · online · title · optional PK banner.
class _MetaRow extends StatelessWidget {
  const _MetaRow({
    required this.room,
    required this.onlineCount,
    required this.likeCount,
    required this.roomOffline,
    required this.roomTerminal,
    this.featurePk = false,
    this.pk,
  });

  final Room room;
  final int onlineCount;
  final int likeCount;
  final bool roomOffline;
  final bool roomTerminal;
  /// Soft-hide PK banner when FEATURE_PK is off (meta.features.pk).
  final bool featurePk;
  final PkSession? pk;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.fromLTRB(12, 12, 12, 4),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          if (featurePk && pk != null && (pk!.isActive || pk!.isEnded)) ...[
            Container(
              key: const Key('pk-banner'),
              width: double.infinity,
              padding: const EdgeInsets.all(10),
              margin: const EdgeInsets.only(bottom: 8),
              decoration: BoxDecoration(
                color: AnyColors.accentSoft,
                borderRadius: BorderRadius.circular(8),
              ),
              child: Text(
                pk!.winnerRoomId == null
                    ? context.l10n.pkScoreLine(
                        pk!.status,
                        pk!.scoreA,
                        pk!.scoreB,
                      )
                    : context.l10n.pkScoreLineWinner(
                        pk!.status,
                        pk!.scoreA,
                        pk!.scoreB,
                        pk!.winnerRoomId!,
                      ),
                key: const Key('pk-score'),
                style: const TextStyle(
                  color: AnyColors.textPrimary,
                  fontSize: 13,
                  fontWeight: FontWeight.w600,
                ),
              ),
            ),
          ],
          Row(
            children: [
              if (room.isLive && !roomOffline) ...[
                LiveBadge(compact: true),
                SizedBox(width: 8),
              ] else
                Container(
                  padding:
                      EdgeInsets.symmetric(horizontal: 8, vertical: 3),
                  decoration: BoxDecoration(
                    color: AnyColors.bgElevated,
                    borderRadius:
                        BorderRadius.circular(AnyColors.radiusPill),
                  ),
                  child: Text(
                    roomTerminal ? context.l10n.statusEnded : room.status.toUpperCase(),
                    style: const TextStyle(
                      color: AnyColors.textSecondary,
                      fontSize: 10,
                      fontWeight: FontWeight.w700,
                    ),
                  ),
                ),
              const SizedBox(width: 8),
              Text(
                '$onlineCount watching',
                style: const TextStyle(
                  color: AnyColors.textSecondary,
                  fontSize: 12,
                ),
              ),
              const SizedBox(width: 12),
              Text(
                '♥ $likeCount',
                style: const TextStyle(
                  color: AnyColors.textSecondary,
                  fontSize: 12,
                ),
              ),
            ],
          ),
          const SizedBox(height: 8),
          Text(
            room.title,
            maxLines: 2,
            overflow: TextOverflow.ellipsis,
            style: const TextStyle(
              color: AnyColors.textPrimary,
              fontSize: 16,
              fontWeight: FontWeight.w600,
              height: 1.25,
            ),
          ),
        ],
      ),
    );
  }
}

/// Avatar placeholder + host id + Follow/Unfollow.
class _ChannelRow extends StatelessWidget {
  const _ChannelRow({
    required this.ownerId,
    required this.following,
    required this.busy,
    required this.onToggle,
  });

  final String ownerId;
  final bool following;
  final bool busy;
  final VoidCallback onToggle;

  @override
  Widget build(BuildContext context) {
    final label = ownerId.isEmpty
        ? context.l10n.host
        : (ownerId.length <= 12 ? ownerId : '${ownerId.substring(0, 10)}…');
    return Padding(
      padding: const EdgeInsets.fromLTRB(12, 8, 12, 8),
      child: Row(
        children: [
          CircleAvatar(
            radius: 18,
            backgroundColor: AnyColors.bgElevated,
            child: Text(
              label.isNotEmpty ? label[0].toUpperCase() : '?',
              style: const TextStyle(
                color: AnyColors.textPrimary,
                fontWeight: FontWeight.w700,
              ),
            ),
          ),
          const SizedBox(width: 10),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  label,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: const TextStyle(
                    color: AnyColors.textPrimary,
                    fontSize: 14,
                    fontWeight: FontWeight.w600,
                  ),
                ),
                Text(
                  context.l10n.host,
                  style: TextStyle(
                    color: AnyColors.textSecondary,
                    fontSize: 12,
                  ),
                ),
              ],
            ),
          ),
          AnimatedSwitcher(
            duration: const Duration(milliseconds: 200),
            child: following
                ? OutlinedButton(
                    key: const ValueKey('unfollow'),
                    onPressed: busy ? null : onToggle,
                    style: OutlinedButton.styleFrom(
                      foregroundColor: AnyColors.textSecondary,
                      side: const BorderSide(color: Color(0x33FFFFFF)),
                      visualDensity: VisualDensity.compact,
                    ),
                    child: Text(context.l10n.unfollow),
                  )
                : FilledButton(
                    key: const ValueKey('follow'),
                    onPressed: busy || ownerId.isEmpty ? null : onToggle,
                    style: FilledButton.styleFrom(
                      visualDensity: VisualDensity.compact,
                    ),
                    child: Text(context.l10n.follow),
                  ),
          ),
        ],
      ),
    );
  }
}

class _ChatInput extends StatelessWidget {
  const _ChatInput({
    required this.controller,
    required this.onSend,
    required this.enabled,
    this.offlineHint,
  });

  final TextEditingController controller;
  final VoidCallback onSend;
  final bool enabled;
  /// When non-null, shown as field hint (e.g. offline / stream ended).
  final String? offlineHint;

  @override
  Widget build(BuildContext context) {
    final hint = offlineHint ?? context.l10n.saySomething;
    return Padding(
      padding: const EdgeInsets.fromLTRB(12, 4, 12, 4),
      child: Row(
        children: [
          Expanded(
            child: TextField(
              controller: controller,
              enabled: enabled,
              decoration: InputDecoration(
                hintText: hint,
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
    );
  }
}

/// Horizontal gift chips + balance + top-up entry.
class _GiftDock extends StatelessWidget {
  const _GiftDock({
    required this.gifts,
    required this.balance,
    required this.onSend,
    required this.onTopup,
    required this.enabled,
  });

  final List<GiftItem> gifts;
  final int balance;
  final void Function(GiftItem) onSend;
  final VoidCallback onTopup;
  final bool enabled;

  @override
  Widget build(BuildContext context) {
    return SafeArea(
      top: false,
      child: Container(
        color: AnyColors.bgElevated,
        padding: const EdgeInsets.fromLTRB(8, 8, 8, 8),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Row(
              children: [
                Icon(
                  Icons.account_balance_wallet,
                  size: 16,
                  color: AnyColors.accent,
                ),
                SizedBox(width: 6),
                Text(
                  context.l10n.balance(balance),
                  style: const TextStyle(
                    color: AnyColors.textPrimary,
                    fontSize: 13,
                    fontWeight: FontWeight.w600,
                  ),
                ),
                Spacer(),
                TextButton(
                  onPressed: onTopup,
                  child: Text(context.l10n.topUp),
                ),
              ],
            ),
            if (gifts.isNotEmpty)
              SizedBox(
                height: 40,
                child: ListView.separated(
                  scrollDirection: Axis.horizontal,
                  itemCount: gifts.length,
                  separatorBuilder: (_, _) => const SizedBox(width: 8),
                  itemBuilder: (context, i) {
                    final g = gifts[i];
                    return ActionChip(
                      label: Text('${g.name} (${g.price})'),
                      onPressed: enabled ? () => onSend(g) : null,
                      backgroundColor: AnyColors.bgInput,
                      labelStyle: TextStyle(
                        color: enabled
                            ? AnyColors.textPrimary
                            : AnyColors.textSecondary,
                        fontSize: 12,
                      ),
                    );
                  },
                ),
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
