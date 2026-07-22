/// Pure helpers for in-app HLS player scaffolding.
///
/// No Flutter widget / plugin deps so unit tests can run without native tooling.

/// Whether the player chrome should render (live status + non-empty URL).
bool shouldShowPlayer(String status, String? hlsUrl) {
  if (status != 'live') return false;
  final url = hlsUrl?.trim() ?? '';
  return url.isNotEmpty;
}

/// Human-readable placeholder when the player is not shown or not embedded.
String playerPlaceholderMessage(String status, String? hlsUrl) {
  final ended = status == 'closed' || status == 'idle' || status == 'ended';
  if (ended) return 'Room ended';
  if (status != 'live') return 'Stream offline';
  final url = hlsUrl?.trim() ?? '';
  if (url.isEmpty) return 'Live — play URL unavailable';
  return 'Open stream URL in external player';
}

/// Heuristic: URL looks like an HLS playlist (m3u8 or path segment).
bool isLikelyHlsUrl(String? url) {
  final raw = url?.trim() ?? '';
  if (raw.isEmpty) return false;
  final lower = raw.toLowerCase();
  if (lower.contains('.m3u8')) return true;
  // Query-stripped path ending or containing /hls/
  final path = Uri.tryParse(raw)?.path.toLowerCase() ?? lower;
  if (path.endsWith('.m3u8') || path.contains('/hls/')) return true;
  return false;
}
