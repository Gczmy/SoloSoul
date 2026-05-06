import 'package:flutter/material.dart';
import 'package:liquid_glass_widgets/liquid_glass_widgets.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart';
import 'package:solosoul_flutter/presentation/widgets/app_sidebar.dart';
import 'package:solosoul_flutter/presentation/widgets/scan_progress_banner.dart';

/// Persistent shell layout with sidebar + main content area.
/// Used as the builder for GoRouter ShellRoute.
///
/// Wrapped in [SoloGlassLayer] so all glass widgets inside the protected
/// pages share a single rendering context for optimal performance.
class ScaffoldWithSidebar extends StatelessWidget {
  final Widget child;

  const ScaffoldWithSidebar({super.key, required this.child});

  @override
  Widget build(BuildContext context) {
    final brightness = MediaQuery.platformBrightnessOf(context);
    final isDark = brightness == Brightness.dark;

    return Scaffold(
      backgroundColor:
          isDark ? AppTheme.darkBackground : AppTheme.lightBackground,
      body: Column(
        children: [
          const ScanProgressBanner(),
          Expanded(
            child: AdaptiveLiquidGlassLayer(
              settings: isDark
                  ? const LiquidGlassSettings(
                      thickness: 28,
                      blur: 10,
                      glassColor: Color(0x33FFFFFF),
                      chromaticAberration: 0.25,
                      refractiveIndex: 1.2,
                      lightIntensity: 1.2,
                      ambientStrength: 0.2,
                      saturation: 1.05,
                    )
                  : const LiquidGlassSettings(
                      thickness: 16,
                      blur: 8,
                      glassColor: Color(0x2DD2DCF0),
                      chromaticAberration: 0.2,
                      refractiveIndex: 1.15,
                      lightIntensity: 1.0,
                      ambientStrength: 0.15,
                      saturation: 1.0,
                    ),
              quality: GlassQuality.standard,
              child: Row(
                children: [
                  const AppSidebar(),
                  VerticalDivider(
                    width: 1,
                    color: isDark
                        ? const Color(0x1AFFFFFF)
                        : const Color(0x1A1F1F1F),
                  ),
                  Expanded(
                    child: GlassBackdropScope(
                      child: child,
                    ),
                  ),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }
}
