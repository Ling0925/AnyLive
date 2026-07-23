import 'package:flutter/material.dart';

import '../theme/any_colors.dart';

/// Red pill `LIVE` badge — never uses brand magenta.
class LiveBadge extends StatelessWidget {
  const LiveBadge({
    super.key,
    this.label = 'LIVE',
    this.compact = false,
  });

  final String label;

  /// Smaller padding for dense card overlays.
  final bool compact;

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: EdgeInsets.symmetric(
        horizontal: compact ? 6 : 8,
        vertical: compact ? 2 : 3,
      ),
      decoration: BoxDecoration(
        color: AnyColors.live,
        borderRadius: BorderRadius.circular(AnyColors.radiusPill),
      ),
      child: Text(
        label,
        style: TextStyle(
          color: Colors.white,
          fontSize: compact ? 10 : 11,
          fontWeight: FontWeight.w700,
          letterSpacing: 0.6,
          height: 1.1,
        ),
      ),
    );
  }
}
