import 'package:flutter/material.dart';
import 'package:flutter_animate/flutter_animate.dart';
import 'package:liquid_glass_widgets/liquid_glass_widgets.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';

/// Header section for the login page.
/// Displays the logo, title, and subtitle.
class LoginHeader extends StatelessWidget {
  const LoginHeader({super.key});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Column(
      children: [
        // Logo — Liquid Glass orb
        Center(
          child: GlassButton(
            icon: const Icon(
              Icons.lock_outline,
              color: Colors.white,
            ),
            onTap: () {},
            width: 80,
            height: 80,
            iconSize: 36,
            shape: const LiquidRoundedSuperellipse(
              borderRadius: 20,
            ),
            useOwnLayer: true,
            settings: const LiquidGlassSettings(
              thickness: 30,
              blur: 10,
              glassColor: Color(0x4D487CA5),
              refractiveIndex: 1.3,
              lightIntensity: 1.2,
            ),
          ),
        )
            .animate()
            .scale(
              begin: const Offset(0.8, 0.8),
              end: const Offset(1, 1),
              duration: 500.ms,
              curve: Curves.easeOutBack,
            )
            .fadeIn(),

        const SizedBox(height: 32),

        // Title
        Text(
          'SoloSoul',
          style: theme.textTheme.headlineMedium?.copyWith(
            fontWeight: FontWeight.w700,
          ),
          textAlign: TextAlign.center,
        ).animate().fadeIn(
          delay: 100.ms,
          duration: 400.ms,
        ),

        const SizedBox(height: 8),

        // Subtitle
        Text(
          AppLocalizations.of(context).loginDataYourControl,
          style: theme.textTheme.bodyLarge?.copyWith(
            color: theme.colorScheme.onSurfaceVariant,
            fontWeight: FontWeight.w400,
          ),
          textAlign: TextAlign.center,
        ).animate().fadeIn(
          delay: 150.ms,
          duration: 400.ms,
        ),
      ],
    );
  }
}
