import 'package:flutter/material.dart';
import 'package:solosoul_flutter/core/services/unified_object_service.dart';

/// Inline icon picker widget.
///
/// Shows a 48×48 trigger button that opens a bottom sheet grid of
/// 26 predefined Material-style icons.
class IconPicker extends StatelessWidget {
  final String iconName;
  final ValueChanged<String> onChanged;

  const IconPicker({super.key, required this.iconName, required this.onChanged});

  static const List<String> _iconNames = [
    'article', 'folder', 'note', 'person', 'flight', 'work',
    'school', 'account_balance', 'credit_card', 'home', 'language',
    'star', 'book', 'favorite', 'security', 'medical_services',
    'phone', 'email', 'link', 'description', 'check_circle',
    'restaurant', 'sports', 'music_note', 'movie', 'camera',
  ];

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return InkWell(
      onTap: () async {
        final result = await showModalBottomSheet<String>(
          context: context,
          builder: (ctx) => SafeArea(
            child: Padding(
              padding: const EdgeInsets.all(16),
              child: Wrap(
                spacing: 12,
                runSpacing: 12,
                children: _iconNames.map((name) {
                  final isSelected = name == iconName;
                  return Material(
                    color: isSelected
                        ? theme.colorScheme.primary.withValues(alpha: 0.15)
                        : theme.colorScheme.surfaceContainerHighest,
                    borderRadius: BorderRadius.circular(10),
                    child: InkWell(
                      borderRadius: BorderRadius.circular(10),
                      onTap: () => Navigator.pop(ctx, name),
                      child: Container(
                        width: 48,
                        height: 48,
                        decoration: BoxDecoration(
                          border: Border.all(
                            color: isSelected ? theme.colorScheme.primary : Colors.transparent,
                            width: 2,
                          ),
                          borderRadius: BorderRadius.circular(10),
                        ),
                        child: Icon(
                          UnifiedObjectService.getIconFromName(name),
                          color: isSelected ? theme.colorScheme.primary : theme.colorScheme.onSurfaceVariant,
                        ),
                      ),
                    ),
                  );
                }).toList(),
              ),
            ),
          ),
        );
        if (result != null) onChanged(result);
      },
      borderRadius: BorderRadius.circular(12),
      child: Container(
        width: 48,
        height: 48,
        decoration: BoxDecoration(
          color: theme.colorScheme.primary.withValues(alpha: 0.1),
          borderRadius: BorderRadius.circular(12),
        ),
        child: Icon(
          UnifiedObjectService.getIconFromName(iconName),
          color: theme.colorScheme.primary,
        ),
      ),
    );
  }
}
