import 'package:flutter/material.dart';
import 'package:liquid_glass_widgets/liquid_glass_widgets.dart';

export 'package:solosoul_flutter/core/constants/sensitivity_enums.dart'
    show SensitivityLevel;

/// SnackBar type for toast notifications
enum SnackBarType {
  info,
  success,
  warning,
  error,
}

/// Shows a SnackBar using Overlay so it appears above dialogs
/// Positioned at top to unify with OperationNotification
///
/// [forOverlay] bypasses [context.mounted] check and uses the given overlay
/// directly — use when [context] may become invalid after an async gap
/// (e.g. the calling widget is removed from the tree by the state change).
/// When provided, theme/MediaQuery values are resolved from the overlay's own
/// context inside the entry builder, so the original [context] is not needed.
void showOverlaySnackBar(
  BuildContext? context, {
  OverlayState? forOverlay,
  required String content,
  Duration duration = const Duration(seconds: 2),
  SnackBarType type = SnackBarType.info,
  String? actionLabel,
  VoidCallback? onAction,
}) {
  final ctx = context;
  if (forOverlay == null) {
    if (ctx == null || !ctx.mounted) return;
  }
  final overlay = forOverlay ?? Overlay.of(ctx!);
  OverlayEntry? entry;

  entry = OverlayEntry(
    builder: (entryContext) {
      // Resolve visual values from the overlay's own context — this is
      // always valid even when the original [context] is stale (forOverlay).
      final BuildContext effectiveContext =
          forOverlay != null ? entryContext : context!;

      final (bgColor, icon, iconColor) = switch (type) {
        SnackBarType.info => (
            Theme.of(effectiveContext).colorScheme.inverseSurface,
            Icons.info_outline,
            Theme.of(effectiveContext).colorScheme.primary,
          ),
        SnackBarType.success => (
            AppTheme.successColor.withValues(alpha: 0.95),
            Icons.check_circle_outline,
            Colors.white,
          ),
        SnackBarType.warning => (
            Colors.orange.shade700,
            Icons.warning_amber_outlined,
            Colors.white,
          ),
        SnackBarType.error => (
            AppTheme.errorColor.withValues(alpha: 0.95),
            Icons.error_outline,
            Colors.white,
          ),
      };

      final topOffset =
          (MediaQuery.maybeOf(effectiveContext)?.padding.top ?? 0) + kToolbarHeight + 8;

      return Positioned(
        top: topOffset,
        left: 16.0,
        right: 16.0,
        child: SafeArea(
          child: Material(
            color: Colors.transparent,
            child: Container(
              padding:
                  const EdgeInsets.symmetric(horizontal: 16.0, vertical: 14.0),
              decoration: BoxDecoration(
                color: bgColor,
                borderRadius: BorderRadius.circular(12.0),
                boxShadow: [
                  BoxShadow(
                    color: Colors.black.withValues(alpha: 0.15),
                    blurRadius: 10,
                    offset: const Offset(0, 4),
                  ),
                ],
              ),
              child: Row(
                children: [
                  Icon(icon, color: iconColor, size: 22),
                  const SizedBox(width: 12),
                  Expanded(
                    child: Text(
                      content,
                      style: const TextStyle(
                        color: Colors.white,
                        fontSize: 14.0,
                        fontWeight: FontWeight.w500,
                      ),
                      textAlign: TextAlign.left,
                    ),
                  ),
                  if (actionLabel != null && onAction != null) ...[
                    const SizedBox(width: 8),
                    TextButton(
                      onPressed: () {
                        entry?.remove();
                        entry = null;
                        onAction();
                      },
                      style: TextButton.styleFrom(
                        foregroundColor: Colors.white,
                        padding: const EdgeInsets.symmetric(horizontal: 8),
                        minimumSize: Size.zero,
                        tapTargetSize: MaterialTapTargetSize.shrinkWrap,
                      ),
                      child: Text(
                        actionLabel,
                        style: const TextStyle(
                          fontWeight: FontWeight.w600,
                          fontSize: 14,
                        ),
                      ),
                    ),
                  ],
                ],
              ),
            ),
          ),
        ),
      );
    },
  );

  if (entry case final e?) overlay.insert(e);
  Future.delayed(duration, () {
    entry?.remove();
    entry = null;
  });
}

// =============================================================================
// AppTheme — Material + Liquid Glass unified theme system
// =============================================================================

/// App theme configuration following Notion + Anytype bright aesthetic
/// fused with iOS 26 Liquid Glass material.
///
/// Design principles:
/// - Clean, breathable whitespace
/// - Notion light mode palette (#FFFFFF canvas, subtle grays)
/// - Liquid Glass as accent/containment, not the protagonist
/// - Consistent spacing and typography hierarchy
class AppTheme {
  AppTheme._();

  // ── CJK Font Fallback ─────────────────────────────────────────────────────
  /// Cross-platform CJK font fallback list.
  /// Ensures Chinese characters render correctly on Windows/macOS/Linux.
  /// Windows: Microsoft YaHei (Simplified) / Microsoft JhengHei (Traditional)
  /// macOS: PingFang SC / Heiti SC
  /// Android/Linux: Noto Sans SC
  static const List<String> _cjkFontFallback = [
    'Microsoft YaHei',
    'Microsoft JhengHei',
    'PingFang SC',
    'Heiti SC',
    'Noto Sans SC',
    'sans-serif',
  ];

  // ── Notion-inspired Color Palette ─────────────────────────────────────────

  /// Primary brand — Notion Blue (desaturated, restrained)
  static const Color primaryColor = Color(0xFF487CA5);

  /// Secondary accent
  static const Color secondaryColor = Color(0xFF6B7C93);

  /// Accent / Cyan for highlights
  static const Color accentColor = Color(0xFF06B6D4);

  /// Error / Danger
  static const Color errorColor = Color(0xFFC4554D);

  /// Success / Positive
  static const Color successColor = Color(0xFF548164);

  /// Warning / Caution
  static const Color warningColor = Color(0xFFC29343);

  // ── Light Theme Colors (Notion Bright) ────────────────────────────────────

  /// Main canvas — pure white like Notion light mode
  static const Color lightBackground = Color(0xFFFFFFFF);

  /// Secondary surface — sidebar, card groupings
  static const Color lightSurface = Color(0xFFF8F9FA);

  /// Primary text — high contrast
  static const Color lightOnSurface = Color(0xFF1F1F1F);

  /// Secondary text — hints, captions
  static const Color lightOnSurfaceVariant = Color(0xFF6B6B6B);

  /// Borders and dividers — subtle
  static const Color lightBorder = Color(0xFFEBECEF);

  // ── Dark Theme Colors ─────────────────────────────────────────────────────

  static const Color darkBackground = Color(0xFF0F172A);
  static const Color darkSurface = Color(0xFF1E293B);
  static const Color darkOnSurface = Color(0xFFF1F5F9);
  static const Color darkOnSurfaceVariant = Color(0xFF94A3B8);

  // ── UI Constants ──────────────────────────────────────────────────────────

  static const Duration kNotificationDuration = Duration(seconds: 5);
  static const Duration kOverlayDuration = Duration(seconds: 2);
  static const Duration kPasswordHintDelay = Duration(seconds: 4);
  static const EdgeInsets kPagePadding = EdgeInsets.all(24);
  static const double kDefaultBorderRadius = 12.0;
  static const int kDefaultMaxVisibleItems = 3;

  // ── Liquid Glass Settings (Light) ─────────────────────────────────────────

  /// Light mode glass settings — subtle, airy, matches white canvas.
  /// Lower thickness and blur to keep the "breathing" feel.
  static const GlassThemeSettings lightGlassSettings = GlassThemeSettings(
    thickness: 16.0,
    blur: 8.0,
    glassColor: Color(0x2DD2DCF0), // ~18% cool blue-white tint
    chromaticAberration: 0.2,
    refractiveIndex: 1.15,
    lightIntensity: 1.0,
    ambientStrength: 0.15,
    saturation: 1.0,
  );

  /// Dark mode glass settings — deeper, more luminous.
  static const GlassThemeSettings darkGlassSettings = GlassThemeSettings(
    thickness: 32.0,
    blur: 10.0,
    glassColor: Color(0x33FFFFFF), // ~20% white tint
    chromaticAberration: 0.25,
    refractiveIndex: 1.2,
    lightIntensity: 1.3,
    ambientStrength: 0.2,
    saturation: 1.05,
  );

  /// Glass glow colors matching our brand palette.
  static const GlassGlowColors glassGlowColors = GlassGlowColors(
    primary: Color(0x3DFFFFFF),
    secondary: Color(0xFF487CA5),
    success: Color(0xFF548164),
    warning: Color(0xFFC29343),
    danger: Color(0xFFC4554D),
    info: Color(0xFF487CA5),
    glowBlurRadius: 4.0,
    glowSpreadRadius: 0,
    glowOpacity: 1.0,
  );

  /// Glass theme data for the entire app.
  static GlassThemeData get glassThemeData => const GlassThemeData(
        light: GlassThemeVariant(
          settings: lightGlassSettings,
          quality: GlassQuality.standard,
          glowColors: glassGlowColors,
          borderRadius: 12.0,
        ),
        dark: GlassThemeVariant(
          settings: darkGlassSettings,
          quality: GlassQuality.standard,
          glowColors: glassGlowColors,
          borderRadius: 12.0,
        ),
      );

  // ── Material Light Theme ──────────────────────────────────────────────────

  /// Light theme
  static ThemeData get lightTheme {
    return ThemeData(
      useMaterial3: true,
      brightness: Brightness.light,
      fontFamilyFallback: _cjkFontFallback,
      colorScheme: const ColorScheme.light(
        primary: primaryColor,
        secondary: secondaryColor,
        tertiary: accentColor,
        error: errorColor,
        surface: lightSurface,
        onSurface: lightOnSurface,
        onSurfaceVariant: lightOnSurfaceVariant,
        surfaceContainerLowest: Color(0xFFF8F9FA),
        surfaceContainerLow: Color(0xFFF1F1EF),
        surfaceContainer: Color(0xFFEBECEF),
        outline: lightBorder,
      ),
      scaffoldBackgroundColor: lightBackground,
      textTheme: _buildTextTheme(lightOnSurface),
      appBarTheme: const AppBarTheme(
        backgroundColor: Colors.transparent,
        foregroundColor: lightOnSurface,
        elevation: 0,
        centerTitle: true,
        titleTextStyle: TextStyle(
          fontSize: 18,
          fontWeight: FontWeight.w600,
          color: lightOnSurface,
        ),
      ),
      elevatedButtonTheme: ElevatedButtonThemeData(
        style: ElevatedButton.styleFrom(
          backgroundColor: primaryColor,
          foregroundColor: Colors.white,
          elevation: 0,
          padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 16),
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(12),
          ),
          textStyle: const TextStyle(
            fontSize: 16,
            fontWeight: FontWeight.w600,
          ),
        ),
      ),
      outlinedButtonTheme: OutlinedButtonThemeData(
        style: OutlinedButton.styleFrom(
          foregroundColor: primaryColor,
          padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 16),
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(12),
          ),
          side: const BorderSide(color: primaryColor, width: 1.5),
          textStyle: const TextStyle(
            fontSize: 16,
            fontWeight: FontWeight.w600,
          ),
        ),
      ),
      inputDecorationTheme: InputDecorationTheme(
        filled: true,
        fillColor: lightSurface,
        contentPadding:
            const EdgeInsets.symmetric(horizontal: 16, vertical: 16),
        border: OutlineInputBorder(
          borderRadius: BorderRadius.circular(12),
          borderSide:
              BorderSide(color: lightOnSurfaceVariant.withValues(alpha: 0.2)),
        ),
        enabledBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(12),
          borderSide:
              BorderSide(color: lightOnSurfaceVariant.withValues(alpha: 0.2)),
        ),
        focusedBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(12),
          borderSide: const BorderSide(color: primaryColor, width: 2),
        ),
        errorBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(12),
          borderSide: const BorderSide(color: errorColor),
        ),
        hintStyle: const TextStyle(
          color: lightOnSurfaceVariant,
          fontSize: 16,
        ),
      ),
      cardTheme: CardThemeData(
        color: lightSurface,
        elevation: 0,
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(16),
          side: BorderSide(color: lightOnSurfaceVariant.withValues(alpha: 0.1)),
        ),
      ),
      dividerTheme: DividerThemeData(
        color: lightOnSurfaceVariant.withValues(alpha: 0.1),
        thickness: 1,
      ),
    );
  }

  /// Dark theme
  static ThemeData get darkTheme {
    return ThemeData(
      useMaterial3: true,
      brightness: Brightness.dark,
      fontFamilyFallback: _cjkFontFallback,
      colorScheme: const ColorScheme.dark(
        primary: primaryColor,
        secondary: secondaryColor,
        tertiary: accentColor,
        error: errorColor,
        surface: darkSurface,
        onSurface: darkOnSurface,
        onSurfaceVariant: darkOnSurfaceVariant,
      ),
      scaffoldBackgroundColor: darkBackground,
      textTheme: _buildTextTheme(darkOnSurface),
      appBarTheme: const AppBarTheme(
        backgroundColor: Colors.transparent,
        foregroundColor: darkOnSurface,
        elevation: 0,
        centerTitle: true,
        titleTextStyle: TextStyle(
          fontSize: 18,
          fontWeight: FontWeight.w600,
          color: darkOnSurface,
        ),
      ),
      elevatedButtonTheme: ElevatedButtonThemeData(
        style: ElevatedButton.styleFrom(
          backgroundColor: primaryColor,
          foregroundColor: Colors.white,
          elevation: 0,
          padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 16),
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(12),
          ),
          textStyle: const TextStyle(
            fontSize: 16,
            fontWeight: FontWeight.w600,
          ),
        ),
      ),
      outlinedButtonTheme: OutlinedButtonThemeData(
        style: OutlinedButton.styleFrom(
          foregroundColor: primaryColor,
          padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 16),
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(12),
          ),
          side: const BorderSide(color: primaryColor, width: 1.5),
          textStyle: const TextStyle(
            fontSize: 16,
            fontWeight: FontWeight.w600,
          ),
        ),
      ),
      inputDecorationTheme: InputDecorationTheme(
        filled: true,
        fillColor: darkBackground,
        contentPadding:
            const EdgeInsets.symmetric(horizontal: 16, vertical: 16),
        border: OutlineInputBorder(
          borderRadius: BorderRadius.circular(12),
          borderSide:
              BorderSide(color: darkOnSurfaceVariant.withValues(alpha: 0.2)),
        ),
        enabledBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(12),
          borderSide:
              BorderSide(color: darkOnSurfaceVariant.withValues(alpha: 0.2)),
        ),
        focusedBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(12),
          borderSide: const BorderSide(color: primaryColor, width: 2),
        ),
        errorBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(12),
          borderSide: const BorderSide(color: errorColor),
        ),
        hintStyle: const TextStyle(
          color: darkOnSurfaceVariant,
          fontSize: 16,
        ),
      ),
      cardTheme: CardThemeData(
        color: darkSurface,
        elevation: 0,
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(16),
          side: BorderSide(color: darkOnSurfaceVariant.withValues(alpha: 0.1)),
        ),
      ),
      dividerTheme: DividerThemeData(
        color: darkOnSurfaceVariant.withValues(alpha: 0.1),
        thickness: 1,
      ),
    );
  }

  static TextTheme _buildTextTheme(Color color) {
    return TextTheme(
      displayLarge: TextStyle(
        fontSize: 57,
        fontWeight: FontWeight.w400,
        color: color,
        fontFamilyFallback: _cjkFontFallback,
      ),
      displayMedium: TextStyle(
        fontSize: 45,
        fontWeight: FontWeight.w400,
        color: color,
        fontFamilyFallback: _cjkFontFallback,
      ),
      displaySmall: TextStyle(
        fontSize: 36,
        fontWeight: FontWeight.w400,
        color: color,
        fontFamilyFallback: _cjkFontFallback,
      ),
      headlineLarge: TextStyle(
        fontSize: 32,
        fontWeight: FontWeight.w600,
        color: color,
        fontFamilyFallback: _cjkFontFallback,
      ),
      headlineMedium: TextStyle(
        fontSize: 28,
        fontWeight: FontWeight.w600,
        color: color,
        fontFamilyFallback: _cjkFontFallback,
      ),
      headlineSmall: TextStyle(
        fontSize: 24,
        fontWeight: FontWeight.w600,
        color: color,
        fontFamilyFallback: _cjkFontFallback,
      ),
      titleLarge: TextStyle(
        fontSize: 22,
        fontWeight: FontWeight.w600,
        color: color,
        fontFamilyFallback: _cjkFontFallback,
      ),
      titleMedium: TextStyle(
        fontSize: 16,
        fontWeight: FontWeight.w600,
        color: color,
        fontFamilyFallback: _cjkFontFallback,
      ),
      titleSmall: TextStyle(
        fontSize: 14,
        fontWeight: FontWeight.w600,
        color: color,
        fontFamilyFallback: _cjkFontFallback,
      ),
      bodyLarge: TextStyle(
        fontSize: 16,
        fontWeight: FontWeight.w400,
        color: color,
        fontFamilyFallback: _cjkFontFallback,
      ),
      bodyMedium: TextStyle(
        fontSize: 14,
        fontWeight: FontWeight.w400,
        color: color,
        fontFamilyFallback: _cjkFontFallback,
      ),
      bodySmall: TextStyle(
        fontSize: 12,
        fontWeight: FontWeight.w400,
        color: color,
        fontFamilyFallback: _cjkFontFallback,
      ),
      labelLarge: TextStyle(
        fontSize: 14,
        fontWeight: FontWeight.w600,
        color: color,
        fontFamilyFallback: _cjkFontFallback,
      ),
      labelMedium: TextStyle(
        fontSize: 12,
        fontWeight: FontWeight.w600,
        color: color,
        fontFamilyFallback: _cjkFontFallback,
      ),
      labelSmall: TextStyle(
        fontSize: 11,
        fontWeight: FontWeight.w600,
        color: color,
        fontFamilyFallback: _cjkFontFallback,
      ),
    );
  }

  /// Password field border styles
  static OutlineInputBorder get passwordFieldEnabledBorder => OutlineInputBorder(
        borderRadius: BorderRadius.circular(8),
        borderSide: BorderSide(color: Colors.grey.shade400),
      );

  static OutlineInputBorder get passwordFieldErrorBorder => OutlineInputBorder(
        borderRadius: BorderRadius.circular(8),
        borderSide: BorderSide(color: Colors.red.shade300),
      );

  static OutlineInputBorder get passwordFieldFocusedErrorBorder =>
      OutlineInputBorder(
        borderRadius: BorderRadius.circular(8),
        borderSide: BorderSide(color: Colors.red.shade500, width: 2),
      );

  /// Input decoration utilities for password fields
  static InputDecoration passwordFieldDecoration({
    String? labelText,
    String? hintText,
    String? errorText,
    Widget? prefixIcon,
    Widget? suffixIcon,
  }) {
    return InputDecoration(
      labelText: labelText,
      hintText: hintText,
      errorText: errorText,
      prefixIcon: prefixIcon,
      suffixIcon: suffixIcon,
      errorStyle: const TextStyle(
        color: errorColor,
        fontWeight: FontWeight.w500,
      ),
      enabledBorder: passwordFieldEnabledBorder,
      errorBorder: passwordFieldErrorBorder,
      focusedErrorBorder: passwordFieldFocusedErrorBorder,
    );
  }
}
