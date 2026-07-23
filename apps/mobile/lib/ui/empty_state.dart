import 'package:flutter/material.dart';

import '../theme/any_colors.dart';

/// Centered empty-list placeholder with optional CTA.
class EmptyState extends StatelessWidget {
  const EmptyState({
    super.key,
    required this.message,
    this.ctaLabel,
    this.onCta,
    this.actionLabel,
    this.onAction,
    this.icon = Icons.inbox_outlined,
  });

  final String message;
  final String? ctaLabel;
  final VoidCallback? onCta;
  /// Alias for [ctaLabel] (shell call sites).
  final String? actionLabel;
  /// Alias for [onCta] (shell call sites).
  final VoidCallback? onAction;
  final IconData icon;

  @override
  Widget build(BuildContext context) {
    final label = ctaLabel ?? actionLabel;
    final action = onCta ?? onAction;
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(32),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(icon, size: 48, color: AnyColors.textSecondary),
            const SizedBox(height: 16),
            Text(
              message,
              textAlign: TextAlign.center,
              style: const TextStyle(
                color: AnyColors.textSecondary,
                fontSize: 15,
              ),
            ),
            if (label != null && action != null) ...[
              const SizedBox(height: 20),
              FilledButton(onPressed: action, child: Text(label)),
            ],
          ],
        ),
      ),
    );
  }
}
