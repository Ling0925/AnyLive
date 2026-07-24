import 'package:flutter/material.dart';
import 'package:shared_preferences/shared_preferences.dart';

/// Persists and notifies the active app locale.
///
/// `null` [locale] means follow the platform; otherwise an explicit language
/// (e.g. `zh`, `en`). Default product preference is Chinese when the platform
/// is unsupported.
class LocaleController extends ChangeNotifier {
  LocaleController({
    Locale? initial,
    bool loaded = false,
  })  : _locale = initial ?? const Locale('zh'),
        _loaded = loaded;

  static const prefsKey = 'anylive_locale_code_v1';
  static const supported = <Locale>[
    Locale('zh'),
    Locale('en'),
  ];

  Locale _locale;
  bool _loaded;

  Locale get locale => _locale;
  bool get isLoaded => _loaded;

  /// Resolved locale for MaterialApp (never null). Default Chinese.
  Locale get effectiveLocale => _locale;

  Future<void> load() async {
    final prefs = await SharedPreferences.getInstance();
    final code = prefs.getString(prefsKey);
    if (code == null || code.isEmpty || code == 'system') {
      // Prefer Chinese as product default when nothing stored.
      _locale = const Locale('zh');
    } else {
      _locale = Locale(code);
    }
    _loaded = true;
    notifyListeners();
  }

  Future<void> setLocale(Locale next) async {
    if (_locale == next) return;
    _locale = next;
    final prefs = await SharedPreferences.getInstance();
    await prefs.setString(prefsKey, next.languageCode);
    notifyListeners();
  }
}
