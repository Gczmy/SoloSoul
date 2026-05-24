import 'dart:ui';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:go_router/go_router.dart';
import 'package:liquid_glass_widgets/liquid_glass_widgets.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';

// =============================================================================
// Glass Adapters — SoloSoul-specific wrappers for liquid_glass_widgets
//
// These adapters bridge the gap between the library's iOS-centric defaults
// (white text, dark backgrounds) and SoloSoul's Notion-inspired bright theme
// (#FFFFFF canvas, dark text).
// =============================================================================

/// Light-theme text style for glass text fields.
/// Overrides the library's default white text.
const TextStyle kGlassTextFieldLightStyle = TextStyle(
  color: Color(0xFF1F1F1F),
  fontSize: 16,
);

/// Light-theme placeholder style for glass text fields.
const TextStyle kGlassTextFieldLightPlaceholderStyle = TextStyle(
  color: Color(0xFF9CA3AF),
  fontSize: 16,
);

/// Dark-theme text style for glass text fields.
const TextStyle kGlassTextFieldDarkStyle = TextStyle(
  color: Color.fromRGBO(255, 255, 255, 0.9),
  fontSize: 16,
);

/// Dark-theme placeholder style for glass text fields.
const TextStyle kGlassTextFieldDarkPlaceholderStyle = TextStyle(
  color: Color.fromRGBO(255, 255, 255, 0.5),
  fontSize: 16,
);

/// Resolves the appropriate text style for the current brightness.
TextStyle glassTextFieldStyle(BuildContext context) {
  final brightness = MediaQuery.platformBrightnessOf(context);
  return brightness == Brightness.dark
      ? kGlassTextFieldDarkStyle
      : kGlassTextFieldLightStyle;
}

/// Resolves the appropriate placeholder style for the current brightness.
TextStyle glassPlaceholderStyle(BuildContext context) {
  final brightness = MediaQuery.platformBrightnessOf(context);
  return brightness == Brightness.dark
      ? kGlassTextFieldDarkPlaceholderStyle
      : kGlassTextFieldLightPlaceholderStyle;
}

/// A convenience wrapper around [AdaptiveLiquidGlassLayer] that applies
/// SoloSoul's theme-aware glass settings.
///
/// Use this to wrap any subtree that contains multiple glass widgets
/// (GlassCard, GlassButton, GlassTextField, etc.) so they can share
/// a single rendering context for better performance.
class SoloGlassLayer extends StatelessWidget {
  final Widget child;
  final GlassQuality? quality;

  const SoloGlassLayer({
    super.key,
    required this.child,
    this.quality,
  });

  @override
  Widget build(BuildContext context) {
    final brightness = MediaQuery.platformBrightnessOf(context);
    final isDark = brightness == Brightness.dark;

    return AdaptiveLiquidGlassLayer(
      settings: isDark
          ? const LiquidGlassSettings(
              thickness: 30,
              blur: 10,
              glassColor: Color(0x33FFFFFF),
              chromaticAberration: 0.25,
              refractiveIndex: 1.2,
              lightIntensity: 1.3,
              ambientStrength: 0.2,
              saturation: 1.05,
            )
          : const LiquidGlassSettings(
              thickness: 18,
              blur: 8,
              glassColor: Color(0x2DD2DCF0),
              chromaticAberration: 0.2,
              refractiveIndex: 1.15,
              lightIntensity: 1.0,
              ambientStrength: 0.15,
              saturation: 1.0,
            ),
      quality: quality ?? GlassQuality.standard,
      child: child,
    );
  }
}

/// A SoloSoul-styled [GlassTextField] that automatically uses the correct
/// text color for bright/dark themes.
///
/// The underlying library defaults to white text (designed for dark iOS
/// backgrounds). This adapter flips to dark text when in light mode.
class SoloGlassTextField extends StatelessWidget {
  final TextEditingController? controller;
  final FocusNode? focusNode;
  final String? placeholder;
  final Widget? prefixIcon;
  final Widget? suffixIcon;
  final VoidCallback? onSuffixTap;
  final bool obscureText;
  final TextInputType? keyboardType;
  final TextInputAction? textInputAction;
  final int maxLines;
  final int? minLines;
  final int? maxLength;
  final bool enabled;
  final bool readOnly;
  final bool autofocus;
  final ValueChanged<String>? onChanged;
  final ValueChanged<String>? onSubmitted;
  final List<TextInputFormatter>? inputFormatters;
  final EdgeInsetsGeometry padding;
  final LiquidShape shape;
  final LiquidGlassSettings? settings;
  final bool useOwnLayer;
  final GlassQuality? quality;

  const SoloGlassTextField({
    super.key,
    this.controller,
    this.focusNode,
    this.placeholder,
    this.prefixIcon,
    this.suffixIcon,
    this.onSuffixTap,
    this.obscureText = false,
    this.keyboardType,
    this.textInputAction,
    this.maxLines = 1,
    this.minLines,
    this.maxLength,
    this.enabled = true,
    this.readOnly = false,
    this.autofocus = false,
    this.onChanged,
    this.onSubmitted,
    this.inputFormatters,
    this.padding = const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
    this.shape = const LiquidRoundedSuperellipse(borderRadius: 10),
    this.settings,
    this.useOwnLayer = false,
    this.quality,
  });

  @override
  Widget build(BuildContext context) {
    final isDark = MediaQuery.platformBrightnessOf(context) == Brightness.dark;

    // Adapt prefix/suffix icon colors for light theme
    Widget? adaptedPrefix;
    Widget? adaptedSuffix;
    if (prefixIcon != null) {
      adaptedPrefix = IconTheme.merge(
        data: IconThemeData(
          color: isDark ? Colors.white70 : const Color(0xFF6B6B6B),
          size: 20,
        ),
        child: prefixIcon!,
      );
    }
    if (suffixIcon != null) {
      adaptedSuffix = IconTheme.merge(
        data: IconThemeData(
          color: isDark ? Colors.white70 : const Color(0xFF6B6B6B),
          size: 20,
        ),
        child: suffixIcon!,
      );
    }

    return GlassTextField(
      controller: controller,
      focusNode: focusNode,
      placeholder: placeholder,
      prefixIcon: adaptedPrefix,
      suffixIcon: adaptedSuffix,
      onSuffixTap: onSuffixTap,
      obscureText: obscureText,
      keyboardType: keyboardType,
      textInputAction: textInputAction,
      maxLines: maxLines,
      minLines: minLines,
      maxLength: maxLength,
      enabled: enabled,
      readOnly: readOnly,
      autofocus: autofocus,
      onChanged: onChanged,
      onSubmitted: onSubmitted,
      inputFormatters: inputFormatters,
      textStyle: glassTextFieldStyle(context),
      placeholderStyle: glassPlaceholderStyle(context),
      padding: padding,
      shape: shape,
      settings: settings,
      useOwnLayer: useOwnLayer,
      quality: quality,
    );
  }
}

/// A SoloSoul-styled [GlassButton] for text labels.
/// Automatically adapts colors for bright/dark themes.
class SoloGlassButton extends StatelessWidget {
  final String label;
  final VoidCallback? onTap;
  final double width;
  final double height;
  final bool enabled;
  final GlassButtonStyle style;

  const SoloGlassButton({
    super.key,
    required this.label,
    this.onTap,
    this.width = 120,
    this.height = 48,
    this.enabled = true,
    this.style = GlassButtonStyle.filled,
  });

  @override
  Widget build(BuildContext context) {
    final isDark = MediaQuery.platformBrightnessOf(context) == Brightness.dark;

    return GlassButton.custom(
      onTap: (enabled && onTap != null) ? onTap! : () {},
      width: width,
      height: height,
      style: style,
      child: Center(
        child: Text(
          label,
          style: TextStyle(
            color: isDark ? Colors.white : const Color(0xFF1F1F1F),
            fontSize: 16,
            fontWeight: FontWeight.w600,
          ),
        ),
      ),
    );
  }
}

/// A SoloSoul-styled [GlassCard] that adapts padding and colors for our design.
class SoloGlassCard extends StatelessWidget {
  final Widget? child;
  final EdgeInsetsGeometry padding;
  final EdgeInsetsGeometry? margin;
  final LiquidShape shape;
  final LiquidGlassSettings? settings;
  final bool useOwnLayer;
  final GlassQuality? quality;
  final double? width;
  final double? height;

  const SoloGlassCard({
    super.key,
    this.child,
    this.padding = const EdgeInsets.all(20),
    this.margin,
    this.shape = const LiquidRoundedSuperellipse(borderRadius: 12),
    this.settings,
    this.useOwnLayer = false,
    this.quality,
    this.width,
    this.height,
  });

  @override
  Widget build(BuildContext context) {
    return GlassCard(
      width: width,
      height: height,
      padding: padding,
      margin: margin,
      shape: shape,
      settings: settings,
      useOwnLayer: useOwnLayer,
      quality: quality,
      child: child,
    );
  }
}

/// A SoloSoul-styled [GlassAppBar] that uses our color scheme.
///
/// When [leading] is null and [automaticallyImplyLeading] is true:
/// - If the navigator can pop, shows a back button that calls [context.pop()].
/// - If [backRoute] is provided and navigator cannot pop, shows a back button
///   that navigates to [backRoute] via [context.go()].
class SoloGlassAppBar extends StatelessWidget implements PreferredSizeWidget {
  final Widget? title;
  final Widget? leading;
  final List<Widget>? actions;
  final bool centerTitle;
  final bool automaticallyImplyLeading;

  /// Fallback route to navigate to when the navigator cannot pop.
  /// Typically [AppRoutes.home] for pages reached via [context.go()].
  final String? backRoute;

  /// Optional widget to place at the bottom of the app bar (e.g. a [TabBar]).
  final PreferredSizeWidget? bottom;

  final Size _basePreferredSize;

  @override
  Size get preferredSize => Size(
        _basePreferredSize.width,
        _basePreferredSize.height + (bottom?.preferredSize.height ?? 0),
      );

  const SoloGlassAppBar({
    super.key,
    this.title,
    this.leading,
    this.actions,
    this.centerTitle = true,
    this.automaticallyImplyLeading = true,
    this.backRoute,
    this.bottom,
    Size preferredSize = const Size.fromHeight(56.0),
  }) : _basePreferredSize = preferredSize;

  @override
  Widget build(BuildContext context) {
    final isDark = MediaQuery.platformBrightnessOf(context) == Brightness.dark;
    final iconColor = isDark ? Colors.white : const Color(0xFF1F1F1F);

    Widget? effectiveLeading = leading;
    if (effectiveLeading == null && automaticallyImplyLeading) {
      final navigator = Navigator.of(context);
      if (navigator.canPop()) {
        effectiveLeading = _BackButton(iconColor: iconColor);
      } else if (backRoute != null) {
        effectiveLeading = _BackButton(
          iconColor: iconColor,
          fallbackRoute: backRoute,
        );
      }
    }

    final bar = GlassAppBar(
      title: DefaultTextStyle.merge(
        style: TextStyle(
          color: iconColor,
          fontWeight: FontWeight.w600,
        ),
        child: title ?? const SizedBox.shrink(),
      ),
      leading: effectiveLeading,
      actions: actions,
      centerTitle: centerTitle,
      preferredSize: _basePreferredSize,
      useOwnLayer: true,
      settings: const LiquidGlassSettings(
        blur: 15,
        thickness: 20,
        glassColor: Color(0x1AFFFFFF),
      ),
    );

    if (bottom == null) return bar;

    return SizedBox.fromSize(
      size: preferredSize,
      child: Column(
        children: [
          bar,
          bottom!,
        ],
      ),
    );
  }
}

/// Back button used by [SoloGlassAppBar] when [automaticallyImplyLeading] is enabled.
class _BackButton extends StatelessWidget {
  final Color iconColor;
  final String? fallbackRoute;

  const _BackButton({required this.iconColor, this.fallbackRoute});

  @override
  Widget build(BuildContext context) {
    return IconButton(
      icon: Icon(Icons.arrow_back_ios_rounded, color: iconColor, size: 20),
      tooltip: AppLocalizations.of(context).commonBack,
      onPressed: () {
        if (Navigator.of(context).canPop()) {
          context.pop();
        } else if (fallbackRoute != null) {
          context.go(fallbackRoute!);
        }
      },
    );
  }
}

/// Shows a SoloSoul-styled glass dialog.
/// Adapts text colors for bright/dark themes.
Future<T?> showSoloGlassDialog<T>({
  required BuildContext context,
  required List<SoloGlassDialogAction> actions,
  String? title,
  String? message,
  Widget? content,
  bool barrierDismissible = false,
}) {
  final isDark = MediaQuery.platformBrightnessOf(context) == Brightness.dark;
  final textColor = isDark ? Colors.white : const Color(0xFF1F1F1F);
  final messageColor =
      isDark ? const Color(0xB3FFFFFF) : const Color(0xB31F1F1F);

  return showDialog<T>(
    context: context,
    barrierDismissible: barrierDismissible,
    barrierColor: isDark
        ? const Color(0x66000000)
        : const Color(0x26000000),
    builder: (context) => Dialog(
      backgroundColor: Colors.transparent,
      elevation: 0,
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 380),
        child: ClipRRect(
          borderRadius: BorderRadius.circular(16),
          child: BackdropFilter(
            filter: ImageFilter.blur(sigmaX: 20, sigmaY: 20),
            child: Container(
              decoration: BoxDecoration(
                color: isDark
                    ? const Color(0xCC1C1C1E)
                    : const Color(0xCCFFFFFF),
                borderRadius: BorderRadius.circular(16),
              ),
              padding: const EdgeInsets.all(24),
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                  if (title != null) ...[
                    Text(
                      title,
                      style: TextStyle(
                        fontSize: 18,
                        fontWeight: FontWeight.bold,
                        color: textColor,
                      ),
                      textAlign: TextAlign.center,
                    ),
                    const SizedBox(height: 8),
                  ],
                  if (message != null) ...[
                    Text(
                      message,
                      style: TextStyle(
                        fontSize: 14,
                        color: messageColor,
                        height: 1.4,
                      ),
                      textAlign: TextAlign.center,
                    ),
                    const SizedBox(height: 8),
                  ],
                  if (content != null) ...[
                    content,
                    const SizedBox(height: 8),
                  ],
                  const SizedBox(height: 16),
                  _buildActions(context, actions, isDark),
                ],
              ),
            ),
          ),
        ),
      ),
    ),
  );
}

Widget _buildActions(
    BuildContext context, List<SoloGlassDialogAction> actions, bool isDark) {
  final textColor = isDark ? Colors.white : const Color(0xFF1F1F1F);

  if (actions.length <= 2) {
    return Row(
      children: actions.asMap().entries.map((entry) {
        final index = entry.key;
        final action = entry.value;
        return Expanded(
          child: Padding(
            padding: EdgeInsets.only(left: index > 0 ? 8 : 0),
            child: GlassButton.custom(
              onTap: action.onPressed,
              height: 44,
              shape: const LiquidRoundedSuperellipse(borderRadius: 10),
              child: Center(
                child: Text(
                  action.label,
                  maxLines: 1,
                  softWrap: false,
                  overflow: TextOverflow.ellipsis,
                  textAlign: TextAlign.center,
                  style: TextStyle(
                    fontSize: 15,
                    fontWeight:
                        action.isPrimary ? FontWeight.bold : FontWeight.w600,
                    color: action.isDestructive
                        ? const Color(0xFFC4554D)
                        : textColor,
                  ),
                ),
              ),
            ),
          ),
        );
      }).toList(),
    );
  }

  return Column(
    crossAxisAlignment: CrossAxisAlignment.stretch,
    children: actions.asMap().entries.map((entry) {
      final index = entry.key;
      final action = entry.value;
      return Padding(
        padding: EdgeInsets.only(top: index > 0 ? 8 : 0),
        child: GlassButton.custom(
          onTap: action.onPressed,
          height: 44,
          shape: const LiquidRoundedSuperellipse(borderRadius: 10),
          child: Center(
            child: Text(
              action.label,
              maxLines: 1,
              softWrap: false,
              overflow: TextOverflow.ellipsis,
              textAlign: TextAlign.center,
              style: TextStyle(
                fontSize: 15,
                fontWeight: action.isPrimary ? FontWeight.bold : FontWeight.w600,
                color: action.isDestructive
                    ? const Color(0xFFC4554D)
                    : textColor,
              ),
            ),
          ),
        ),
      );
    }).toList(),
  );
}

/// Data class for glass dialog actions.
class SoloGlassDialogAction {
  final String label;
  final VoidCallback onPressed;
  final bool isPrimary;
  final bool isDestructive;

  const SoloGlassDialogAction({
    required this.label,
    required this.onPressed,
    this.isPrimary = false,
    this.isDestructive = false,
  });
}
