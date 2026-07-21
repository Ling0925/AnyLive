import 'package:flutter/material.dart';

import '../../api/api_client.dart';
import '../../api/profile_repository.dart';
import '../../config/app_config.dart';

/// Simple profile editor: load GET /me, save display name via PATCH /me.
class ProfilePage extends StatefulWidget {
  const ProfilePage({
    super.key,
    required this.config,
    required this.accessToken,
    this.profileRepository,
    this.onDisplayNameChanged,
  });

  final AppConfig config;
  final String accessToken;

  /// Injectable for tests; when null a real [ProfileRepository] is created.
  final ProfileRepository? profileRepository;

  /// Called after a successful save so the parent can refresh session label.
  final ValueChanged<String>? onDisplayNameChanged;

  @override
  State<ProfilePage> createState() => _ProfilePageState();
}

class _ProfilePageState extends State<ProfilePage> {
  late final ProfileRepository _repo;
  final _name = TextEditingController();
  String? _email;
  String? _error;
  bool _loading = true;
  bool _saving = false;

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
    _load();
  }

  @override
  void dispose() {
    _name.dispose();
    super.dispose();
  }

  Future<void> _load() async {
    setState(() {
      _loading = true;
      _error = null;
    });
    try {
      final me = await _repo.getMe();
      if (!mounted) return;
      setState(() {
        _name.text = me.displayName;
        _email = me.email;
        _loading = false;
      });
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
      final updated = await _repo.patchMe(displayName: trimmed);
      if (!mounted) return;
      widget.onDisplayNameChanged?.call(updated.displayName);
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('Profile saved')),
      );
      setState(() {
        _name.text = updated.displayName;
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

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Edit profile')),
      body: _loading
          ? const Center(child: CircularProgressIndicator())
          : Padding(
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
                  TextField(
                    controller: _name,
                    decoration: const InputDecoration(
                      labelText: 'Display name',
                      border: OutlineInputBorder(),
                    ),
                    enabled: !_saving,
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
                  const SizedBox(height: 16),
                  FilledButton(
                    onPressed: _saving ? null : _save,
                    child: Text(_saving ? 'Saving…' : 'Save'),
                  ),
                ],
              ),
            ),
    );
  }
}
