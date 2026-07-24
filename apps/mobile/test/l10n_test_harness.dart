import 'package:anylive_mobile/l10n/l10n.dart';
import 'package:anylive_mobile/l10n/locale_controller.dart';
import 'package:anylive_mobile/l10n/locale_scope.dart';
import 'package:flutter/material.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:flutter_test/flutter_test.dart';

/// Pump [child] under MaterialApp with English localizations (tests assert EN).
Future<void> pumpL10n(
  WidgetTester tester,
  Widget child, {
  Locale locale = const Locale('en'),
  ThemeData? theme,
}) async {
  final controller = LocaleController(initial: locale, loaded: true);
  await tester.pumpWidget(
    LocaleScope(
      controller: controller,
      child: MaterialApp(
        locale: locale,
        supportedLocales: AppLocalizations.supportedLocales,
        localizationsDelegates: const [
          AppLocalizations.delegate,
          GlobalMaterialLocalizations.delegate,
          GlobalWidgetsLocalizations.delegate,
          GlobalCupertinoLocalizations.delegate,
        ],
        theme: theme,
        home: child,
      ),
    ),
  );
  await tester.pump();
}
