import 'package:flutter/material.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart';

class SidebarHeader extends StatelessWidget {
  final bool expanded;
  final VoidCallback onToggle;

  const SidebarHeader({
    super.key,
    required this.expanded,
    required this.onToggle,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return SizedBox(
      height: 64,
      child: expanded
          ? LayoutBuilder(
              builder: (context, constraints) {
                final showText = constraints.maxWidth >= 140;
                return Padding(
                  padding: const EdgeInsets.symmetric(horizontal: 16),
                  child: Row(
                    children: [
                      Container(
                        width: 36,
                        height: 36,
                        decoration: BoxDecoration(
                          color: AppTheme.primaryColor.withValues(alpha: 0.15),
                          borderRadius: BorderRadius.circular(10),
                        ),
                        child: const Icon(
                          Icons.auto_awesome,
                          color: AppTheme.primaryColor,
                          size: 20,
                        ),
                      ),
                      if (showText) ...[
                        const SizedBox(width: 12),
                        Expanded(
                          child: Text(
                            'SoloSoul',
                            style: theme.textTheme.titleMedium?.copyWith(
                              fontWeight: FontWeight.w700,
                            ),
                            overflow: TextOverflow.ellipsis,
                          ),
                        ),
                      ],
                      if (!showText) const Spacer(),
                      IconButton(
                        icon: const Icon(Icons.chevron_left),
                        onPressed: onToggle,
                        tooltip: 'Collapse',
                      ),
                    ],
                  ),
                );
              },
            )
          : Center(
              child: IconButton(
                icon: const Icon(Icons.auto_awesome),
                onPressed: onToggle,
                tooltip: 'Expand',
              ),
            ),
    );
  }
}
