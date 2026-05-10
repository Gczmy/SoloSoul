import 'package:flutter/material.dart';

// =============================================================================
// Section Title
// =============================================================================

class LLMSectionTitle extends StatelessWidget {
  final String title;
  final ThemeData theme;

  const LLMSectionTitle({super.key, required this.title, required this.theme});

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 8),
      child: Text(
        title,
        style: theme.textTheme.titleSmall?.copyWith(
          color: theme.colorScheme.primary,
          fontWeight: FontWeight.w600,
        ),
      ),
    );
  }
}
