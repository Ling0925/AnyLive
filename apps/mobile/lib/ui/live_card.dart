import 'package:flutter/material.dart';

import '../api/rooms_repository.dart';
import '../theme/any_colors.dart';
import 'live_badge.dart';

/// 16:9 room card for Home / Following feeds.
class LiveCard extends StatelessWidget {
  const LiveCard({
    super.key,
    required this.room,
    required this.onTap,
  });

  final Room room;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return Material(
      color: AnyColors.bgElevated,
      borderRadius: BorderRadius.circular(AnyColors.radiusCard),
      clipBehavior: Clip.antiAlias,
      child: InkWell(
        onTap: onTap,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            AspectRatio(
              aspectRatio: 16 / 9,
              child: Stack(
                fit: StackFit.expand,
                children: [
                  // Color placeholder — no cover API (design Q5).
                  const DecoratedBox(
                    decoration: BoxDecoration(
                      gradient: LinearGradient(
                        begin: Alignment.topLeft,
                        end: Alignment.bottomRight,
                        colors: [
                          AnyColors.gradientStart,
                          AnyColors.gradientEnd,
                        ],
                      ),
                    ),
                  ),
                  const Center(
                    child: Icon(
                      Icons.live_tv_outlined,
                      color: AnyColors.textSecondary,
                      size: 40,
                    ),
                  ),
                  if (room.isLive)
                    const Positioned(
                      top: 10,
                      left: 10,
                      child: LiveBadge(compact: true),
                    ),
                ],
              ),
            ),
            Padding(
              padding: const EdgeInsets.fromLTRB(12, 10, 12, 12),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    room.title.isEmpty ? 'Untitled live' : room.title,
                    maxLines: 2,
                    overflow: TextOverflow.ellipsis,
                    style: const TextStyle(
                      color: AnyColors.textPrimary,
                      fontSize: 16,
                      fontWeight: FontWeight.w600,
                      height: 1.25,
                    ),
                  ),
                  const SizedBox(height: 4),
                  Text(
                    room.ownerId.isEmpty
                        ? room.status
                        : '${room.ownerId} · ${room.status}',
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: const TextStyle(
                      color: AnyColors.textSecondary,
                      fontSize: 12,
                    ),
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}
