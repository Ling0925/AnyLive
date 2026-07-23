import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'any_colors.dart';

/// Dark-only Material theme from explicit [AnyColors] tokens.
///
/// Does **not** use [ColorScheme.fromSeed] so seed purple never fights
/// brand magenta.
ThemeData anyDarkTheme() {
  const scheme = ColorScheme.dark(
    brightness: Brightness.dark,
    primary: AnyColors.accent,
    onPrimary: AnyColors.textPrimary,
    secondary: AnyColors.accent,
    onSecondary: AnyColors.textPrimary,
    surface: AnyColors.bgElevated,
    onSurface: AnyColors.textPrimary,
    error: AnyColors.danger,
    onError: AnyColors.textPrimary,
    surfaceContainerHighest: AnyColors.bgElevated,
    outline: Color(0x14FFFFFF),
  );

  return ThemeData(
    useMaterial3: true,
    brightness: Brightness.dark,
    colorScheme: scheme,
    scaffoldBackgroundColor: AnyColors.bgApp,
    canvasColor: AnyColors.bgApp,
    dividerColor: const Color(0x14FFFFFF),
    appBarTheme: const AppBarTheme(
      backgroundColor: AnyColors.bgApp,
      foregroundColor: AnyColors.textPrimary,
      elevation: 0,
      scrolledUnderElevation: 0,
      centerTitle: false,
      systemOverlayStyle: SystemUiOverlayStyle.light,
      titleTextStyle: TextStyle(
        color: AnyColors.textPrimary,
        fontSize: 18,
        fontWeight: FontWeight.w600,
      ),
    ),
    bottomNavigationBarTheme: const BottomNavigationBarThemeData(
      backgroundColor: AnyColors.bgElevated,
      selectedItemColor: AnyColors.accent,
      unselectedItemColor: AnyColors.textSecondary,
      type: BottomNavigationBarType.fixed,
      elevation: 0,
      selectedLabelStyle: TextStyle(fontSize: 12, fontWeight: FontWeight.w600),
      unselectedLabelStyle: TextStyle(fontSize: 12),
    ),
    cardTheme: CardThemeData(
      color: AnyColors.bgElevated,
      elevation: 0,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(AnyColors.radiusCard),
      ),
      margin: EdgeInsets.zero,
    ),
    inputDecorationTheme: InputDecorationTheme(
      filled: true,
      fillColor: AnyColors.bgInput,
      hintStyle: const TextStyle(color: AnyColors.textSecondary),
      labelStyle: const TextStyle(color: AnyColors.textSecondary),
      border: OutlineInputBorder(
        borderRadius: BorderRadius.circular(8),
        borderSide: const BorderSide(color: Color(0x14FFFFFF)),
      ),
      enabledBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(8),
        borderSide: const BorderSide(color: Color(0x14FFFFFF)),
      ),
      focusedBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(8),
        borderSide: const BorderSide(color: AnyColors.accent),
      ),
      contentPadding: const EdgeInsets.symmetric(horizontal: 12, vertical: 12),
    ),
    filledButtonTheme: FilledButtonThemeData(
      style: FilledButton.styleFrom(
        backgroundColor: AnyColors.accent,
        foregroundColor: AnyColors.textPrimary,
        disabledBackgroundColor: AnyColors.accent.withValues(alpha: 0.35),
        disabledForegroundColor: AnyColors.textPrimary.withValues(alpha: 0.6),
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(8),
        ),
      ),
    ),
    textButtonTheme: TextButtonThemeData(
      style: TextButton.styleFrom(
        foregroundColor: AnyColors.accent,
      ),
    ),
    floatingActionButtonTheme: const FloatingActionButtonThemeData(
      backgroundColor: AnyColors.accent,
      foregroundColor: AnyColors.textPrimary,
    ),
    snackBarTheme: const SnackBarThemeData(
      backgroundColor: AnyColors.bgElevated,
      contentTextStyle: TextStyle(color: AnyColors.textPrimary),
      behavior: SnackBarBehavior.floating,
    ),
    listTileTheme: const ListTileThemeData(
      iconColor: AnyColors.textSecondary,
      textColor: AnyColors.textPrimary,
    ),
    progressIndicatorTheme: const ProgressIndicatorThemeData(
      color: AnyColors.accent,
    ),
    dividerTheme: const DividerThemeData(
      color: Color(0x14FFFFFF),
      thickness: 1,
      space: 1,
    ),
    dialogTheme: DialogThemeData(
      backgroundColor: AnyColors.bgElevated,
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(16)),
      titleTextStyle: const TextStyle(
        color: AnyColors.textPrimary,
        fontSize: 18,
        fontWeight: FontWeight.w600,
      ),
      contentTextStyle: const TextStyle(color: AnyColors.textSecondary),
    ),
    textTheme: const TextTheme(
      titleLarge: TextStyle(
        color: AnyColors.textPrimary,
        fontSize: 18,
        fontWeight: FontWeight.w600,
      ),
      titleMedium: TextStyle(
        color: AnyColors.textPrimary,
        fontSize: 16,
        fontWeight: FontWeight.w600,
      ),
      bodyLarge: TextStyle(
        color: AnyColors.textPrimary,
        fontSize: 14,
        fontWeight: FontWeight.w400,
      ),
      bodyMedium: TextStyle(
        color: AnyColors.textPrimary,
        fontSize: 14,
        fontWeight: FontWeight.w400,
      ),
      bodySmall: TextStyle(
        color: AnyColors.textSecondary,
        fontSize: 12,
        fontWeight: FontWeight.w400,
      ),
      labelLarge: TextStyle(
        color: AnyColors.textPrimary,
        fontSize: 14,
        fontWeight: FontWeight.w600,
      ),
    ),
  );
}
