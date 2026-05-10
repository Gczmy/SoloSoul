import 'package:flutter/material.dart';

/// Character counter widget displaying current/max characters.
///
/// Displays the count with a "/" separator, coloring red when at max.
class CharacterCounter extends StatelessWidget {
  final int currentLength;
  final int maxLength;
  final String maxLabel;

  const CharacterCounter({
    super.key,
    required this.currentLength,
    required this.maxLength,
    required this.maxLabel,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final isAtMax = currentLength >= maxLength;

    return Row(
      mainAxisAlignment: MainAxisAlignment.center,
      children: [
        SizedBox(
          width: 16,
          child: Text(
            '$currentLength',
            textAlign: TextAlign.right,
            style: theme.textTheme.bodySmall?.copyWith(
              color: isAtMax
                  ? theme.colorScheme.error
                  : theme.colorScheme.onSurfaceVariant,
            ),
          ),
        ),
        Text(
          '/',
          style: theme.textTheme.bodySmall?.copyWith(
            color: isAtMax
                ? theme.colorScheme.error
                : theme.colorScheme.onSurfaceVariant,
          ),
        ),
        SizedBox(
          width: 16,
          child: Text(
            maxLabel,
            textAlign: TextAlign.left,
            style: theme.textTheme.bodySmall?.copyWith(
              color: isAtMax
                  ? theme.colorScheme.error
                  : theme.colorScheme.onSurfaceVariant,
            ),
          ),
        ),
      ],
    );
  }
}
