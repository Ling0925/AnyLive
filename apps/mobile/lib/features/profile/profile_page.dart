import 'dart:async';
import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../../api/api_client.dart';
import '../../api/auth_repository.dart';
import '../../api/compliance_repository.dart';
import '../../api/profile_repository.dart';
import '../../api/session_store.dart';
import '../../config/app_config.dart';

/// Profile editor + GDPR export/delete hooks.
class ProfilePage extends StatefulWidget {
  const ProfilePage({
    super.key,
    required this.config,
    required this.accessToken,
    this.profileRepository,
    this.complianceRepository,
    this.sessionStore,
    this.onDisplayNameChanged,
    this.onAccountDeleted,
  });

  final AppConfig config;
  final String accessToken;
  final ProfileRepository? profileRepository;
  final ComplianceRepository? complianceRepository;
  final SessionStore? sessionStore;
  final ValueChanged<String>? onDisplayNameChanged;
  final Future<void> Function()? onAccountDeleted;

  @override
  State<ProfilePage> createState() => _ProfilePageState();
}

class _ProfilePageState extends State<ProfilePage> {
  late final ProfileRepository _repo;
  late final ComplianceRepository _compliance;
  late final AuthRepository _auth;
  final _name = TextEditingController();
  final _region = TextEditingController();
  String? _email;
  String? _error;
  String? _exportHint;
  String? _avatarUrl;
  bool _loading = true;
  bool _saving = false;
  bool _busyDsar = false;
  bool _busyAvatar = false;
  bool _ageConfirmed = false;
  bool _privacyAccepted = false;
  // ISO region code (WBS E2.5).
  CreatorStats? _creator;
  String? _creatorError;
  int _sessionCount = 0;
  List<RefreshSessionInfo> _sessions = const [];

  @override
  void initState() {
    super.initState();
    if (widget.profileRepository != null) {
      _repo = widget.profileRepository!;
    } else {
      final api = ApiClient(
        baseUrl: widget.config.normalizedApiBaseUrl,
        accessToken: widget.accessToken,
      );
      _repo = ProfileRepository(client: api);
    }
    if (widget.complianceRepository != null) {
      _compliance = widget.complianceRepository!;
    } else {
      final api = ApiClient(
        baseUrl: widget.config.normalizedApiBaseUrl,
        accessToken: widget.accessToken,
      );
      _compliance = ComplianceRepository(client: api);
    }
    final authApi = ApiClient(
      baseUrl: widget.config.normalizedApiBaseUrl,
      accessToken: widget.accessToken,
    );
    _auth = AuthRepository(client: authApi);
    _load();
  }

  @override
  void dispose() {
    _name.dispose();
    _region.dispose();
    super.dispose();
  }

  Future<void> _load() async {
    setState(() {
      _loading = true;
      _error = null;
      _creatorError = null;
    });
    try {
      final me = await _repo.getMe();
      if (!mounted) return;
      setState(() {
        _name.text = me.displayName;
        _email = me.email;
        _ageConfirmed = me.ageConfirmed;
        _privacyAccepted = me.privacyAccepted;
        _region.text = me.region ?? '';
        _avatarUrl = me.avatarUrl;
        _loading = false;
      });
      // Creator stats are best-effort — profile still usable if this fails.
      unawaited(_loadCreator());
      unawaited(_loadSessions());
    } on ProfileException catch (e) {
      if (!mounted) return;
      setState(() {
        _error = e.toString();
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

  Future<void> _loadCreator() async {
    try {
      final stats = await _repo.getCreatorStats();
      if (!mounted) return;
      setState(() {
        _creator = stats;
        _creatorError = null;
      });
    } on ProfileException catch (e) {
      if (!mounted) return;
      setState(() => _creatorError = e.toString());
    } catch (e) {
      if (!mounted) return;
      setState(() => _creatorError = e.toString());
    }
  }

  Future<void> _loadSessions() async {
    try {
      final sessions = await _auth.listSessions();
      if (!mounted) return;
      setState(() {
        _sessions = sessions;
        _sessionCount = sessions.length;
      });
    } catch (_) {
      // best-effort
    }
  }

  Future<void> _revokeSession(RefreshSessionInfo session) async {
    try {
      await _auth.revokeSession(session.jti);
      if (!mounted) return;
      setState(() {
        _sessions = _sessions.where((s) => s.jti != session.jti).toList();
        _sessionCount = _sessions.length;
      });
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('Session revoked')),
      );
    } catch (e) {
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('revoke failed: $e')),
      );
    }
  }

  Future<void> _logoutAllDevices() async {
    final ok = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('Sign out all devices?'),
        content: const Text(
          'Revokes every refresh session. This device will need to log in again.',
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(ctx).pop(false),
            child: const Text('Cancel'),
          ),
          FilledButton(
            key: const Key('confirm-logout-all'),
            onPressed: () => Navigator.of(ctx).pop(true),
            child: const Text('Sign out all'),
          ),
        ],
      ),
    );
    if (ok != true || !mounted) return;
    try {
      final n = await _auth.logoutAllSessions();
      if (!mounted) return;
      setState(() {
        _sessionCount = 0;
        _sessions = const [];
      });
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('Revoked $n session(s)')),
      );
      final deleted = widget.onAccountDeleted;
      if (deleted != null) {
        await deleted();
      }
    } catch (e) {
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('logout-all failed: $e')),
      );
    }
  }

  Future<void> _save() async {
    final trimmed = _name.text.trim();
    if (trimmed.isEmpty) {
      setState(() => _error = 'Display name is required');
      return;
    }
    setState(() {
      _saving = true;
      _error = null;
    });
    try {
      final updated = await _repo.patchMe(
        displayName: trimmed,
        ageConfirmed: _ageConfirmed,
        privacyAccepted: _privacyAccepted,
        region: _region.text.trim(),
      );
      if (!mounted) return;
      widget.onDisplayNameChanged?.call(updated.displayName);
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('Profile saved')),
      );
      setState(() {
        _name.text = updated.displayName;
        _ageConfirmed = updated.ageConfirmed;
        _privacyAccepted = updated.privacyAccepted;
        _region.text = updated.region ?? '';
        _saving = false;
      });
    } on ProfileException catch (e) {
      if (!mounted) return;
      setState(() {
        _error = e.toString();
        _saving = false;
      });
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _error = e.toString();
        _saving = false;
      });
    }
  }

  Future<void> _setAvatarUrl() async {
    setState(() {
      _busyAvatar = true;
      _error = null;
    });
    try {
      final presign = await _repo.presignAvatar();
      // Control-plane dogfood: confirm without binary PUT when MinIO is off.
      final updated = await _repo.confirmAvatar(
        objectKey: presign.objectKey,
        publicUrl: presign.publicUrl,
      );
      if (!mounted) return;
      setState(() {
        _avatarUrl = updated.avatarUrl;
        _busyAvatar = false;
      });
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('Avatar URL set')),
      );
    } on ProfileException catch (e) {
      if (!mounted) return;
      setState(() {
        _error = e.toString();
        _busyAvatar = false;
      });
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _error = e.toString();
        _busyAvatar = false;
      });
    }
  }

  Future<void> _export() async {
    setState(() {
      _busyDsar = true;
      _error = null;
      _exportHint = null;
    });
    try {
      final payload = await _compliance.exportMe();
      final pretty = const JsonEncoder.withIndent('  ').convert(payload);
      // Clipboard may throw under flutter_test without a mock channel.
      try {
        await Clipboard.setData(ClipboardData(text: pretty));
      } catch (_) {}
      if (!mounted) return;
      setState(() {
        _busyDsar = false;
        _exportHint = 'Export copied (${pretty.length} chars)';
      });
    } on ComplianceException catch (e) {
      if (!mounted) return;
      setState(() {
        _error = e.toString();
        _busyDsar = false;
      });
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _error = e.toString();
        _busyDsar = false;
      });
    }
  }

  Future<void> _deleteAccount() async {
    final ok = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('Delete account?'),
        content: const Text(
          'This soft-deletes your account. You will be signed out.',
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(ctx).pop(false),
            child: const Text('Cancel'),
          ),
          FilledButton(
            key: const Key('confirm-delete-account'),
            onPressed: () => Navigator.of(ctx).pop(true),
            child: const Text('Delete'),
          ),
        ],
      ),
    );
    if (ok != true) return;
    setState(() {
      _busyDsar = true;
      _error = null;
    });
    try {
      await _compliance.deleteMe();
      await widget.sessionStore?.clear();
      await widget.onAccountDeleted?.call();
      if (!mounted) return;
      Navigator.of(context).pop();
    } on ComplianceException catch (e) {
      if (!mounted) return;
      setState(() => _error = e.toString());
    } catch (e) {
      if (!mounted) return;
      setState(() => _error = e.toString());
    } finally {
      if (mounted) setState(() => _busyDsar = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Edit profile')),
      body: _loading
          ? const Center(child: CircularProgressIndicator())
          : SingleChildScrollView(
              padding: const EdgeInsets.all(24),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  if (_email != null) ...[
                    Text(
                      'Email',
                      style: Theme.of(context).textTheme.labelLarge,
                    ),
                    const SizedBox(height: 4),
                    Text(_email!),
                    const SizedBox(height: 16),
                  ],
                  if (_avatarUrl != null && _avatarUrl!.isNotEmpty) ...[
                    Text(
                      'Avatar URL',
                      style: Theme.of(context).textTheme.labelLarge,
                    ),
                    const SizedBox(height: 4),
                    Text(
                      key: const Key('avatar-url'),
                      _avatarUrl!,
                      style: Theme.of(context).textTheme.bodySmall,
                    ),
                    const SizedBox(height: 8),
                  ],
                  OutlinedButton.icon(
                    key: const Key('avatar-presign-confirm'),
                    onPressed: (_saving || _busyAvatar) ? null : _setAvatarUrl,
                    icon: const Icon(Icons.account_circle_outlined),
                    label: Text(
                      _busyAvatar ? 'Setting avatar…' : 'Set avatar URL',
                    ),
                  ),
                  const SizedBox(height: 16),
                  TextField(
                    controller: _name,
                    decoration: const InputDecoration(
                      labelText: 'Display name',
                      border: OutlineInputBorder(),
                    ),
                    enabled: !_saving,
                  ),
                  const SizedBox(height: 8),
                  TextField(
                    key: const Key('profile-region'),
                    controller: _region,
                    decoration: const InputDecoration(
                      labelText: 'Region (e.g. US, SG)',
                      border: OutlineInputBorder(),
                    ),
                    enabled: !_saving,
                    textCapitalization: TextCapitalization.characters,
                  ),
                  const SizedBox(height: 8),
                  CheckboxListTile(
                    contentPadding: EdgeInsets.zero,
                    title: const Text('I confirm I am 18 or older'),
                    value: _ageConfirmed,
                    onChanged: _saving
                        ? null
                        : (v) => setState(() => _ageConfirmed = v ?? false),
                    controlAffinity: ListTileControlAffinity.leading,
                  ),
                  CheckboxListTile(
                    contentPadding: EdgeInsets.zero,
                    title: const Text('I accept the privacy policy'),
                    value: _privacyAccepted,
                    onChanged: _saving
                        ? null
                        : (v) => setState(() => _privacyAccepted = v ?? false),
                    controlAffinity: ListTileControlAffinity.leading,
                  ),
                  if (_error != null) ...[
                    const SizedBox(height: 12),
                    Text(
                      _error!,
                      style: TextStyle(
                        color: Theme.of(context).colorScheme.error,
                      ),
                    ),
                  ],
                  if (_exportHint != null) ...[
                    const SizedBox(height: 12),
                    Text(
                      key: const Key('export-copied-hint'),
                      _exportHint!,
                      style: Theme.of(context).textTheme.bodySmall?.copyWith(
                            color: Theme.of(context).colorScheme.primary,
                          ),
                    ),
                  ],
                  const SizedBox(height: 16),
                  FilledButton(
                    onPressed: _saving ? null : _save,
                    child: Text(_saving ? 'Saving…' : 'Save'),
                  ),
                  const SizedBox(height: 24),
                  Text(
                    key: const Key('creator-center-title'),
                    'Creator center',
                    style: Theme.of(context).textTheme.titleMedium,
                  ),
                  const SizedBox(height: 8),
                  if (_creatorError != null)
                    Text(
                      key: const Key('creator-center-error'),
                      _creatorError!,
                      style: TextStyle(
                        color: Theme.of(context).colorScheme.error,
                      ),
                    )
                  else if (_creator == null)
                    const Text(
                      key: Key('creator-center-loading'),
                      'Loading stats…',
                    )
                  else
                    Card(
                      key: const Key('creator-center-card'),
                      child: Padding(
                        padding: const EdgeInsets.all(12),
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Wrap(
                              spacing: 16,
                              runSpacing: 8,
                              children: [
                                _statChip(
                                  'Followers',
                                  '${_creator!.followerCount}',
                                ),
                                _statChip(
                                  'Following',
                                  '${_creator!.followingCount}',
                                ),
                                _statChip(
                                  'Live rooms',
                                  '${_creator!.liveRooms}',
                                ),
                                _statChip(
                                  'Total rooms',
                                  '${_creator!.totalRooms}',
                                ),
                                _statChip(
                                  'Gift coins',
                                  '${_creator!.giftCoinsReceived}',
                                ),
                                _statChip(
                                  'Gift credits',
                                  '${_creator!.giftCreditEntries}',
                                ),
                              ],
                            ),
                            if (_creator!.rooms.isNotEmpty) ...[
                              const SizedBox(height: 12),
                              Text(
                                'Your rooms',
                                style: Theme.of(context).textTheme.labelLarge,
                              ),
                              const SizedBox(height: 4),
                              ..._creator!.rooms.take(5).map(
                                    (r) => Text(
                                      '· ${r.title} (${r.status})',
                                      key: Key('creator-room-${r.id}'),
                                    ),
                                  ),
                            ],
                          ],
                        ),
                      ),
                    ),
                  const SizedBox(height: 24),
                  Text(
                    'Privacy & data',
                    style: Theme.of(context).textTheme.titleMedium,
                  ),
                  const SizedBox(height: 8),
                  OutlinedButton.icon(
                    key: const Key('export-account'),
                    onPressed: _busyDsar ? null : _export,
                    icon: const Icon(Icons.download_outlined),
                    label: const Text('Export my data'),
                  ),
                  const SizedBox(height: 8),
                  OutlinedButton.icon(
                    key: const Key('logout-all-sessions'),
                    onPressed: _logoutAllDevices,
                    icon: const Icon(Icons.devices),
                    label: Text(
                      _sessionCount > 0
                          ? 'Sign out all devices ($_sessionCount)'
                          : 'Sign out all devices',
                    ),
                  ),
                  if (_sessions.isNotEmpty) ...[
                    const SizedBox(height: 8),
                    ..._sessions.map(
                      (s) => ListTile(
                        key: Key('session-${s.jti}'),
                        contentPadding: EdgeInsets.zero,
                        dense: true,
                        title: Text(
                          s.jti.length > 12 ? '${s.jti.substring(0, 12)}…' : s.jti,
                          style: Theme.of(context).textTheme.bodySmall,
                        ),
                        subtitle: Text(
                          'expires ${s.expiresAt}',
                          style: Theme.of(context).textTheme.labelSmall,
                        ),
                        trailing: TextButton(
                          key: Key('revoke-session-${s.jti}'),
                          onPressed: () => _revokeSession(s),
                          child: const Text('Revoke'),
                        ),
                      ),
                    ),
                  ],
                  const SizedBox(height: 8),
                  OutlinedButton.icon(
                    key: const Key('delete-account'),
                    onPressed: _busyDsar ? null : _deleteAccount,
                    icon: Icon(
                      Icons.delete_outline,
                      color: Theme.of(context).colorScheme.error,
                    ),
                    label: Text(
                      'Delete account',
                      style: TextStyle(
                        color: Theme.of(context).colorScheme.error,
                      ),
                    ),
                  ),
                ],
              ),
            ),
    );
  }

  Widget _statChip(String label, String value) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          label,
          style: Theme.of(context).textTheme.labelSmall,
        ),
        Text(
          value,
          style: Theme.of(context).textTheme.titleSmall,
        ),
      ],
    );
  }
}
