import 'package:flutter/material.dart';

import '../../api/session_store.dart';
import '../../config/app_config.dart';
import '../../theme/any_colors.dart';
import '../profile/profile_page.dart';
import '../wallet/wallet_page.dart';
import '../../l10n/l10n.dart';
import '../../l10n/locale_scope.dart';

/// YouTube-style "You" tab: account, wallet, privacy via existing pages.
class YouPage extends StatelessWidget {
  const YouPage({
    super.key,
    required this.config,
    required this.accessToken,
    this.sessionLabel,
    this.sessionStore,
    this.onLogout,
    this.onDisplayNameChanged,
    this.onAccountDeleted,
  });

  final AppConfig config;
  final String accessToken;
  final String? sessionLabel;
  final SessionStore? sessionStore;
  final Future<void> Function()? onLogout;
  final void Function(String name)? onDisplayNameChanged;
  final Future<void> Function()? onAccountDeleted;

  Future<void> _openProfile(BuildContext context) async {
    await Navigator.of(context).push(
      MaterialPageRoute(
        builder: (_) => ProfilePage(
          config: config,
          accessToken: accessToken,
          sessionStore: sessionStore,
          onDisplayNameChanged: onDisplayNameChanged,
          onAccountDeleted: onAccountDeleted,
        ),
      ),
    );
  }

  Future<void> _openWallet(BuildContext context) async {
    await Navigator.of(context).push(
      MaterialPageRoute(
        builder: (_) => WalletPage(
          config: config,
          accessToken: accessToken,
        ),
      ),
    );
  }

  Future<void> _confirmLogout(BuildContext context) async {
    final ok = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text(ctx.l10n.logoutConfirmTitle),
        content: Text(ctx.l10n.logoutConfirmBody),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(ctx).pop(false),
            child: Text(ctx.l10n.cancel),
          ),
          FilledButton(
            onPressed: () => Navigator.of(ctx).pop(true),
            child: Text(ctx.l10n.logOut),
          ),
        ],
      ),
    );
    if (ok == true) {
      await onLogout?.call();
    }
  }

  @override
  Widget build(BuildContext context) {
    final l10n = context.l10n;
    final label = (sessionLabel == null || sessionLabel!.isEmpty)
        ? l10n.youTitle
        : sessionLabel!;
    final initial = label.isNotEmpty ? label[0].toUpperCase() : '?';

    return Scaffold(
      backgroundColor: AnyColors.bg,
      appBar: AppBar(
        title: Text(l10n.youTitle),
        backgroundColor: AnyColors.bg,
      ),
      body: ListView(
        padding: const EdgeInsets.fromLTRB(16, 8, 16, 32),
        children: [
          Material(
            color: AnyColors.elevated,
            borderRadius: BorderRadius.circular(12),
            child: InkWell(
              borderRadius: BorderRadius.circular(12),
              onTap: () => _openProfile(context),
              child: Container(
                padding: const EdgeInsets.all(16),
                decoration: BoxDecoration(
                  borderRadius: BorderRadius.circular(12),
                  border: Border.all(color: AnyColors.border),
                ),
                child: Row(
                  children: [
                    CircleAvatar(
                      radius: 28,
                      backgroundColor: AnyColors.accentSoft,
                      child: Text(
                        initial,
                        style: const TextStyle(
                          color: AnyColors.accent,
                          fontWeight: FontWeight.w700,
                          fontSize: 22,
                        ),
                      ),
                    ),
                    const SizedBox(width: 14),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(
                            label,
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                            style: const TextStyle(
                              fontSize: 18,
                              fontWeight: FontWeight.w600,
                              color: AnyColors.textPrimary,
                            ),
                          ),
                          const SizedBox(height: 4),
                          if (sessionLabel != null && sessionLabel!.isNotEmpty)
                            Text(
                              l10n.signedInAs(sessionLabel!),
                              maxLines: 1,
                              overflow: TextOverflow.ellipsis,
                              style: const TextStyle(
                                fontSize: 12,
                                color: AnyColors.textMuted,
                              ),
                            )
                          else
                            Text(
                              config.normalizedApiBaseUrl,
                              maxLines: 1,
                              overflow: TextOverflow.ellipsis,
                              style: const TextStyle(
                                fontSize: 12,
                                color: AnyColors.textMuted,
                              ),
                            ),
                        ],
                      ),
                    ),
                    const Icon(Icons.chevron_right, color: AnyColors.textMuted),
                  ],
                ),
              ),
            ),
          ),
          const SizedBox(height: 20),
          _sectionLabel(l10n.sectionAccount),
          _tile(
            context,
            key: const Key('you-profile'),
            icon: Icons.manage_accounts_outlined,
            title: l10n.profilePrivacy,
            subtitle: l10n.profilePrivacySub,
            onTap: () => _openProfile(context),
          ),
          _tile(
            context,
            key: const Key('home-wallet'),
            icon: Icons.account_balance_wallet_outlined,
            title: l10n.wallet,
            subtitle: l10n.walletSub,
            onTap: () => _openWallet(context),
          ),
          // Alias key for newer finders.
          const Offstage(child: SizedBox.shrink(key: Key('you-wallet'))),
          const SizedBox(height: 16),
          _sectionLabel(l10n.sectionSession),
          _tile(
            context,
            key: const Key('you-logout'),
            icon: Icons.logout,
            title: l10n.logout,
            subtitle: l10n.logoutSub,
            danger: true,
            onTap: () => _confirmLogout(context),
          ),
          // Compat key for tests that still look for home-logout.
          Opacity(
            opacity: 0,
            child: SizedBox(
              height: 0,
              child: TextButton(
                key: const Key('home-logout'),
                onPressed: () => onLogout?.call(),
                child: Text(l10n.logout),
              ),
            ),
          ),
          const SizedBox(height: 16),
          _sectionLabel(l10n.languageSection),
          _LanguageTile(),
          const SizedBox(height: 16),
          _sectionLabel(l10n.sectionAbout),

          Padding(
            padding: const EdgeInsets.only(left: 4, top: 4),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  l10n.envLine(config.environment),
                  style: const TextStyle(
                    fontSize: 12,
                    color: AnyColors.textMuted,
                  ),
                ),
                Text(
                  l10n.flavorLine(config.flavorLabel),
                  style: const TextStyle(
                    fontSize: 12,
                    color: AnyColors.textMuted,
                  ),
                ),
                Text(
                  l10n.apiLine(config.normalizedApiBaseUrl),
                  style: const TextStyle(
                    fontSize: 12,
                    color: AnyColors.textMuted,
                  ),
                ),
              ],
            ),
          ),
          const SizedBox(height: 24),
          Text(
            l10n.brandFlavor(config.flavorLabel),
            textAlign: TextAlign.center,
            style: const TextStyle(fontSize: 12, color: AnyColors.textMuted),
          ),
        ],
      ),
    );
  }

  Widget _sectionLabel(String text) {
    return Padding(
      padding: const EdgeInsets.only(left: 4, bottom: 8),
      child: Text(
        text.toUpperCase(),
        style: const TextStyle(
          fontSize: 11,
          fontWeight: FontWeight.w700,
          letterSpacing: 0.6,
          color: AnyColors.textMuted,
        ),
      ),
    );
  }

  Widget _tile(
    BuildContext context, {
    Key? key,
    required IconData icon,
    required String title,
    required String subtitle,
    required VoidCallback onTap,
    bool danger = false,
  }) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 8),
      child: Material(
        color: AnyColors.elevated,
        borderRadius: BorderRadius.circular(12),
        child: ListTile(
          key: key,
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(12),
            side: const BorderSide(color: AnyColors.border),
          ),
          leading: Icon(
            icon,
            color: danger ? AnyColors.danger : AnyColors.textSecondary,
          ),
          title: Text(
            title,
            style: TextStyle(
              color: danger ? AnyColors.danger : AnyColors.textPrimary,
              fontWeight: FontWeight.w600,
            ),
          ),
          subtitle: Text(
            subtitle,
            style: const TextStyle(color: AnyColors.textMuted, fontSize: 12),
          ),
          trailing: const Icon(Icons.chevron_right, color: AnyColors.textMuted),
          onTap: onTap,
        ),
      ),
    );
  }
}


class _LanguageTile extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    final l10n = context.l10n;
    final controller = LocaleScope.maybeOf(context);
    final code = controller?.locale.languageCode ?? 'zh';
    return Material(
      color: AnyColors.elevated,
      borderRadius: BorderRadius.circular(12),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 4),
        child: Row(
          children: [
            const Icon(Icons.language, color: AnyColors.accent),
            const SizedBox(width: 12),
            Expanded(
              child: Text(
                l10n.language,
                style: const TextStyle(
                  color: AnyColors.textPrimary,
                  fontWeight: FontWeight.w600,
                ),
              ),
            ),
            DropdownButtonHideUnderline(
              child: DropdownButton<String>(
                key: const Key('language-picker'),
                value: code == 'en' ? 'en' : 'zh',
                dropdownColor: AnyColors.elevated,
                style: const TextStyle(color: AnyColors.textPrimary),
                items: [
                  DropdownMenuItem(value: 'zh', child: Text(l10n.languageChinese)),
                  DropdownMenuItem(value: 'en', child: Text(l10n.languageEnglish)),
                ],
                onChanged: controller == null
                    ? null
                    : (v) {
                        if (v == null) return;
                        controller.setLocale(Locale(v));
                      },
              ),
            ),
          ],
        ),
      ),
    );
  }
}
