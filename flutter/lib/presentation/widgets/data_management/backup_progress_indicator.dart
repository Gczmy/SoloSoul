import 'package:flutter/material.dart';

class BackupProgressIndicator extends StatelessWidget {
  final double progress;

  const BackupProgressIndicator({
    super.key,
    required this.progress,
  });

  String get _statusText {
    if (progress >= 1.0) return 'Finishing...';
    if (progress >= 0.9) return 'Writing file...';
    if (progress >= 0.5) return 'Encrypting...';
    if (progress >= 0.3) return 'Encoding...';
    return 'Reading data...';
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 20),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          LinearProgressIndicator(
            value: progress > 0 ? progress : null,
            borderRadius: BorderRadius.circular(4),
          ),
          const SizedBox(height: 4),
          Text(
            _statusText,
            style: theme.textTheme.bodySmall?.copyWith(
              color: theme.colorScheme.onSurfaceVariant,
            ),
          ),
        ],
      ),
    );
  }
}
