import 'package:flutter/material.dart';

/// Shared dark watch-chrome tokens (aligned with H5 `--*` CSS vars).
///
/// Brand magenta is for primary CTA / selected tab only.
/// LIVE badge always uses [live] red, never magenta.
class AnyColors {
  AnyColors._();

  static const bgApp = Color(0xFF0F0F0F);
  static const bgElevated = Color(0xFF212121);
  static const bgPlayer = Color(0xFF000000);
  static const bgInput = Color(0xFF121212);

  static const textPrimary = Color(0xFFF1F1F1);
  static const textSecondary = Color(0xFFAAAAAA);

  /// Brand magenta — CTA / selected tab / Follow filled.
  static const accent = Color(0xFFC850FF);

  /// accent @ 15% — chip / soft fills.
  static const accentSoft = Color(0x26C850FF);

  /// LIVE badge only.
  static const live = Color(0xFFFF0033);

  static const success = Color(0xFF3DDC97);
  static const danger = Color(0xFFFF4D4F);

  static const radiusCard = 12.0;
  static const radiusPill = 999.0;

  // Backward-compatible aliases used by earlier draft widgets.
  static const bg = bgApp;
  static const surface = bgElevated;
  static const elevated = bgElevated;
  static const border = Color(0x14FFFFFF);
  static const textMuted = textSecondary;
  static const gradientStart = Color(0xFF2A1640);
  static const gradientEnd = bgApp;
}
