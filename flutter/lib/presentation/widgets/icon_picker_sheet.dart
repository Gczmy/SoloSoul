import 'package:flutter/material.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/core/services/unified_object_service.dart';

/// Predefined icon names available for selection across the app.
const List<String> kIconNames = [
  'article',
  'folder',
  'note',
  'person',
  'flight',
  'work',
  'school',
  'account_balance',
  'credit_card',
  'home',
  'language',
  'star',
  'book',
  'favorite',
  'security',
  'medical_services',
  'phone',
  'email',
  'link',
  'description',
  'check_circle',
  'restaurant',
  'sports',
  'music_note',
  'movie',
  'camera',
];

/// Bottom sheet for picking an icon from a predefined grid.
class IconPickerSheet extends StatelessWidget {
  final String currentIcon;

  const IconPickerSheet({super.key, required this.currentIcon});

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);

    return SafeArea(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(l10n.iconPickerTitle, style: theme.textTheme.titleLarge),
            const SizedBox(height: 16),
            Wrap(
              spacing: 12,
              runSpacing: 12,
              children: kIconNames.map((name) {
                final isSelected = name == currentIcon;
                return Material(
                  color: isSelected
                      ? theme.colorScheme.primary.withValues(alpha: 0.15)
                      : theme.colorScheme.surfaceContainerHighest,
                  borderRadius: BorderRadius.circular(10),
                  child: InkWell(
                    borderRadius: BorderRadius.circular(10),
                    onTap: () => Navigator.pop(context, name),
                    child: Container(
                      width: 48,
                      height: 48,
                      decoration: BoxDecoration(
                        border: Border.all(
                          color: isSelected
                              ? theme.colorScheme.primary
                              : Colors.transparent,
                          width: 2,
                        ),
                        borderRadius: BorderRadius.circular(10),
                      ),
                      child: Icon(
                        UnifiedObjectService.getIconFromName(name),
                        color: isSelected
                            ? theme.colorScheme.primary
                            : theme.colorScheme.onSurfaceVariant,
                      ),
                    ),
                  ),
                );
              }).toList(),
            ),
          ],
        ),
      ),
    );
  }
}
