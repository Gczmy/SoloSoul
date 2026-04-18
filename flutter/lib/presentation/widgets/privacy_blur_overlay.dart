import 'dart:ui';
import 'package:flutter/material.dart';

/// Privacy blur overlay that obscures the screen when vault is locked.
/// Uses BackdropFilter with blur and shows a lock icon with "Vault Protected" text.
class PrivacyBlurOverlay extends StatelessWidget {
  final bool visible;

  const PrivacyBlurOverlay({
    super.key,
    required this.visible,
  });

  @override
  Widget build(BuildContext context) {
    if (!visible) {
      return const SizedBox.shrink();
    }

    return Positioned.fill(
      child: BackdropFilter(
        filter: ImageFilter.blur(sigmaX: 20, sigmaY: 20),
        child: Container(
          color: Colors.black.withValues(alpha: 0.7),
          child: const Center(
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                Icon(
                  Icons.lock_outline,
                  size: 64,
                  color: Colors.white70,
                ),
                SizedBox(height: 16),
                Text(
                  'Vault Protected',
                  style: TextStyle(
                    color: Colors.white70,
                    fontSize: 20,
                    fontWeight: FontWeight.w500,
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}
