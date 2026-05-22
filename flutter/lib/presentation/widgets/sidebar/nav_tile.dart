import 'package:flutter/material.dart';

class NavTile extends StatelessWidget {
  final IconData icon;
  final String label;
  final bool expanded;
  final bool selected;
  final VoidCallback onTap;
  final VoidCallback? onIconTap;

  const NavTile({
    super.key,
    required this.icon,
    required this.label,
    required this.expanded,
    this.selected = false,
    required this.onTap,
    this.onIconTap,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final bgColor = selected
        ? theme.colorScheme.primary.withValues(alpha: 0.1)
        : Colors.transparent;
    final fgColor = selected
        ? theme.colorScheme.primary
        : theme.colorScheme.onSurface;

    final tile = Padding(
      padding: EdgeInsets.symmetric(
        horizontal: expanded ? 0 : 8,
        vertical: 2,
      ),
      child: SizedBox(
        width: double.infinity,
        child: Material(
          color: bgColor,
          borderRadius: BorderRadius.circular(8),
          child: InkWell(
            onTap: onTap,
            borderRadius: BorderRadius.circular(8),
            child: LayoutBuilder(
              builder: (context, constraints) {
                final showLabel = expanded && constraints.maxWidth >= 50;
                return Container(
                  height: 40,
                  alignment: showLabel ? Alignment.centerLeft : Alignment.center,
                  padding: showLabel
                      ? const EdgeInsets.symmetric(horizontal: 12)
                      : const EdgeInsets.all(0),
                  child: showLabel
                      ? Row(
                          children: [
                            // Consistent padding so alignment matches PageTreeTile.
                            Padding(
                              padding: const EdgeInsets.all(4),
                              child: onIconTap != null
                                  ? InkWell(
                                      onTap: onIconTap,
                                      borderRadius: BorderRadius.circular(6),
                                      child: Icon(icon, size: 20, color: fgColor),
                                    )
                                  : Icon(icon, size: 20, color: fgColor),
                            ),
                            const SizedBox(width: 12),
                            Expanded(
                              child: Text(
                                label,
                                style: theme.textTheme.bodyMedium?.copyWith(
                                  color: fgColor,
                                  fontWeight: selected ? FontWeight.w600 : null,
                                ),
                                overflow: TextOverflow.ellipsis,
                              ),
                            ),
                          ],
                        )
                      : Center(child: Icon(icon, size: 22, color: fgColor)),
                );
              },
            ),
          ),
        ),
      ),
    );

    if (expanded) return tile;
    return Tooltip(message: label, child: tile);
  }
}
