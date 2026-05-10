import 'package:flutter/material.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart';

/// Background decorations for the login page.
/// Displays decorative gradient orbs for visual depth.
class LoginBackground extends StatelessWidget {
  const LoginBackground({super.key});

  @override
  Widget build(BuildContext context) {
    final isDark = MediaQuery.platformBrightnessOf(context) == Brightness.dark;

    return Stack(
      children: [
        // Top-left orb
        Positioned(
          top: 80,
          left: -80,
          child: Container(
            width: 240,
            height: 240,
            decoration: BoxDecoration(
              shape: BoxShape.circle,
              color: AppTheme.primaryColor.withValues(
                alpha: isDark ? 0.12 : 0.07,
              ),
            ),
          ),
        ),
        // Bottom-right orb
        Positioned(
          bottom: 120,
          right: -100,
          child: Container(
            width: 300,
            height: 300,
            decoration: BoxDecoration(
              shape: BoxShape.circle,
              color: AppTheme.accentColor.withValues(
                alpha: isDark ? 0.1 : 0.05,
              ),
            ),
          ),
        ),
        // Top-right small orb
        Positioned(
          top: 300,
          right: 20,
          child: Container(
            width: 100,
            height: 100,
            decoration: BoxDecoration(
              shape: BoxShape.circle,
              color: AppTheme.secondaryColor.withValues(
                alpha: isDark ? 0.08 : 0.04,
              ),
            ),
          ),
        ),
      ],
    );
  }
}
