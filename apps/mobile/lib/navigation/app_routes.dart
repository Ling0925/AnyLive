/// Named route constants and thin helpers (WBS E8.1 nav scaffold).
///
/// Full go_router / Riverpod migration is deferred; pages still use
/// [Navigator] + [MaterialPageRoute]. These paths document the intended
/// graph and power optional [MaterialApp.routes] entries.
class AppRoutes {
  AppRoutes._();

  static const home = '/';
  static const login = '/login';
  static const profile = '/profile';
  static const wallet = '/wallet';
  static const feed = '/feed';
  static const rooms = '/rooms';
  static const following = '/following';
  static const goLive = '/go-live';
  static const you = '/you';

  /// Room detail: `/rooms/:roomId`
  static String room(String roomId) => '/rooms/$roomId';

  /// Parse `/rooms/{id}` → id, or null when not a room path.
  static String? parseRoomId(String? path) {
    if (path == null || path.isEmpty) return null;
    final uri = Uri.tryParse(path);
    if (uri == null) return null;
    final segs = uri.pathSegments;
    if (segs.length >= 2 && segs[0] == 'rooms' && segs[1].isNotEmpty) {
      return segs[1];
    }
    // Also accept absolute paths without host.
    final m = RegExp(r'^/rooms/([^/]+)/?$').firstMatch(path);
    return m?.group(1);
  }
}
