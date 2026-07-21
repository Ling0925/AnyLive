/// Minimal HTTP client for AnyLive API (P1 auth surface).
class ApiClient {
  ApiClient({required this.baseUrl, this.accessToken});

  final String baseUrl;
  String? accessToken;

  String get _root => baseUrl.endsWith('/')
      ? baseUrl.substring(0, baseUrl.length - 1)
      : baseUrl;

  /// Build JSON headers with optional bearer token.
  Map<String, String> jsonHeaders({bool auth = false}) {
    final headers = <String, String>{
      'Content-Type': 'application/json',
      'Accept': 'application/json',
    };
    if (auth && accessToken != null && accessToken!.isNotEmpty) {
      headers['Authorization'] = 'Bearer $accessToken';
    }
    return headers;
  }

  Uri uri(String path) {
    final p = path.startsWith('/') ? path : '/$path';
    return Uri.parse('$_root$p');
  }
}
