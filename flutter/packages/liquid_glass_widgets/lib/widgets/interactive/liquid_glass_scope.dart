import 'package:flutter/widgets.dart';

/// A scope that provides infrastructure for glass refraction on Skia and Web.
///
/// Place [LiquidGlassScope] at the root of a stack or page.
/// Descendant [GlassEffect] widgets (used by [GlassSegmentedControl],
/// [GlassTabBar], [GlassBottomBar]) will automatically find and sample the
/// capture surface marked by [GlassRefractionSource].
///
/// On Impeller, [LiquidGlassScope] is not needed — `GlassQuality.premium`
/// uses the native scene graph for refraction.
///
/// Usage:
/// ```dart
/// LiquidGlassScope(
///   child: Stack(
///     children: [
///       // 1. Mark the capture surface
///       GlassRefractionSource(
///         child: Image.asset('wallpaper.jpg'),
///       ),
///
///       // 2. Glass widgets sample it automatically
///       Center(child: GlassSegmentedControl(...)),
///     ],
///   ),
/// )
/// ```
class LiquidGlassScope extends StatefulWidget {
  const LiquidGlassScope({
    required this.child,
    super.key,
  });

  /// Convenience constructor for the common pattern of a background behind content.
  ///
  /// This eliminates the boilerplate of manually creating a Stack with Positioned.fill
  /// widgets. It's equivalent to:
  ///
  /// ```dart
  /// LiquidGlassScope(
  ///   child: Stack(
  ///     children: [
  ///       Positioned.fill(
  ///         child: GlassRefractionSource(child: background),
  ///       ),
  ///       Positioned.fill(child: content),
  ///     ],
  ///   ),
  /// )
  /// ```
  ///
  /// Example:
  /// ```dart
  /// LiquidGlassScope.stack(
  ///   background: Image.asset('wallpaper.jpg', fit: BoxFit.cover),
  ///   content: Scaffold(
  ///     body: MyContent(),
  ///     bottomNavigationBar: GlassBottomBar(...),
  ///   ),
  /// )
  /// ```
  factory LiquidGlassScope.stack({
    Key? key,
    required Widget background,
    required Widget content,
  }) {
    return LiquidGlassScope(
      key: key,
      child: Stack(
        children: [
          Positioned.fill(
            child: GlassRefractionSource(child: background),
          ),
          content, // Don't wrap in Positioned - let it naturally fill
        ],
      ),
    );
  }

  final Widget child;

  /// Returns the background key from the nearest ancestor scope.
  static GlobalKey? of(BuildContext context) {
    return context
        .dependOnInheritedWidgetOfExactType<_InheritedLiquidGlassScope>()
        ?.backgroundKey;
  }

  @override
  State<LiquidGlassScope> createState() => _LiquidGlassScopeState();
}

class _LiquidGlassScopeState extends State<LiquidGlassScope> {
  // Create the key ONCE and keep it stable across rebuilds
  final GlobalKey _backgroundKey =
      GlobalKey(debugLabel: 'LiquidGlassBackground');

  @override
  Widget build(BuildContext context) {
    assert(() {
      // Warn if nesting scopes (usually unintentional)
      final parentScope = context
          .dependOnInheritedWidgetOfExactType<_InheritedLiquidGlassScope>();
      if (parentScope != null) {
        debugPrint(
          '⚠️ [LiquidGlassScope] Warning: Nested LiquidGlassScope detected.\n'
          '   Inner scope will override outer scope for descendant widgets.\n'
          '   This is usually intentional for isolated demos, but may be unexpected.\n'
          '   If you want a single shared background, use only one scope at the root.',
        );
      }
      return true;
    }());

    return _InheritedLiquidGlassScope(
      backgroundKey: _backgroundKey,
      child: widget.child,
    );
  }
}

/// Marks a widget as the refraction capture source for the nearest [LiquidGlassScope].
///
/// Wraps [child] in a [RepaintBoundary] tagged with the scope's [GlobalKey].
/// Descendant [GlassEffect] widgets (the liquid pill inside [GlassSegmentedControl],
/// [GlassTabBar], and [GlassBottomBar]) will sample this boundary every frame
/// to produce real background refraction on Skia and Web.
///
/// On Impeller with `GlassQuality.premium`, this is not needed — the native
/// scene graph handles refraction without a captured boundary.
///
/// Typically used inside a [LiquidGlassScope] via the `.stack()` factory:
///
/// ```dart
/// LiquidGlassScope.stack(
///   background: Image.asset('wallpaper.jpg', fit: BoxFit.cover),
///   content: Scaffold(...),
/// )
/// ```
///
/// Or manually, for granular control:
///
/// ```dart
/// GlassRefractionSource(
///   child: Image.asset('wallpaper.jpg'),
/// )
/// ```
class GlassRefractionSource extends StatelessWidget {
  const GlassRefractionSource({
    required this.child,
    super.key,
  });

  final Widget child;

  @override
  Widget build(BuildContext context) {
    final key = LiquidGlassScope.of(context);

    assert(() {
      if (key == null) {
        debugPrint(
          'ℹ️ [GlassRefractionSource] No LiquidGlassScope found in the widget tree.\n'
          '   The background will render normally but glass refraction will use\n'
          '   synthetic frost instead of real background sampling.\n'
          '   Wrap your widget tree with LiquidGlassScope to enable refraction.',
        );
      }
      return true;
    }());

    // If no scope is found, render the child normally — no silent failures.
    if (key == null) return child;

    return RepaintBoundary(
      key: key,
      child: child,
    );
  }
}

/// Deprecated: use [GlassRefractionSource] instead.
///
/// [LiquidGlassBackground] was renamed to [GlassRefractionSource] in 0.7.0
/// to better reflect its purpose: marking the capture surface for glass
/// refraction rather than acting as a generic background widget.
@Deprecated(
  'Use GlassRefractionSource instead. '
  'LiquidGlassBackground was renamed in 0.7.0 for clarity. '
  'This alias will be removed in 1.0.0.',
)
typedef LiquidGlassBackground = GlassRefractionSource;

class _InheritedLiquidGlassScope extends InheritedWidget {
  const _InheritedLiquidGlassScope({
    required this.backgroundKey,
    required super.child,
  });

  final GlobalKey backgroundKey;

  @override
  bool updateShouldNotify(_InheritedLiquidGlassScope oldWidget) {
    return backgroundKey != oldWidget.backgroundKey;
  }
}
