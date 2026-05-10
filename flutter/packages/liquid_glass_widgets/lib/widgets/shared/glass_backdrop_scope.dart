import 'package:flutter/widgets.dart';

/// A scope widget that enables GPU blur-sharing across all glass layers in its
/// subtree.
///
/// Wrap your screen — or the portion that contains multiple glass surfaces — in
/// [GlassBackdropScope] to allow those surfaces to share a single GPU
/// framebuffer capture for their blur step. Without this wrapper each
/// [AdaptiveLiquidGlassLayer] (and therefore every glass widget) captures its
/// own backdrop independently. With it all layers inside the scope share one
/// capture on Impeller, roughly halving GPU blit cost when multiple glass
/// surfaces are visible simultaneously (e.g. [GlassAppBar] + [GlassBottomBar]
/// on the same screen).
///
/// On Skia and Web the lightweight shader path is used instead of
/// [AdaptiveLiquidGlassLayer], so this widget has no effect on those backends.
///
/// ## Usage
///
/// ```dart
/// GlassBackdropScope(
///   child: Scaffold(
///     appBar: GlassAppBar(...),
///     body: ...,
///     bottomNavigationBar: GlassBottomBar(...),
///   ),
/// )
/// ```
///
/// Place [GlassBackdropScope] as high in the tree as practical — typically
/// wrapping your [Scaffold] — so all glass surfaces on the screen are
/// descendants.
class GlassBackdropScope extends StatelessWidget {
  /// Creates a [GlassBackdropScope].
  const GlassBackdropScope({required this.child, super.key});

  /// The subtree in which glass layers will share a backdrop capture.
  final Widget child;

  @override
  Widget build(BuildContext context) {
    return BackdropGroup(child: child);
  }
}
