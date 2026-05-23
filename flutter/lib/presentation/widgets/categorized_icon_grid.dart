import 'package:flutter/material.dart';
import 'package:solosoul_flutter/core/services/unified_object_service.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';

/// A scrollable categorized grid of icons for selection.
///
/// Icons are grouped by [IconCategory] with section headers.
/// Used by icon pickers in bottom sheets and inline dialogs.
class CategorizedIconGrid extends StatelessWidget {
  final String currentIcon;
  final ValueChanged<String> onSelected;
  final double iconSize;
  final double spacing;

  const CategorizedIconGrid({
    super.key,
    required this.currentIcon,
    required this.onSelected,
    this.iconSize = 48,
    this.spacing = 12,
  });

  String _categoryLabel(AppLocalizations l10n, String nameKey) {
    return switch (nameKey) {
      'iconCategoryWork' => l10n.iconCategoryWork,
      'iconCategoryPeople' => l10n.iconCategoryPeople,
      'iconCategoryTravel' => l10n.iconCategoryTravel,
      'iconCategoryFinance' => l10n.iconCategoryFinance,
      'iconCategoryLife' => l10n.iconCategoryLife,
      'iconCategoryTech' => l10n.iconCategoryTech,
      'iconCategoryCreative' => l10n.iconCategoryCreative,
      'iconCategoryGeneral' => l10n.iconCategoryGeneral,
      _ => nameKey,
    };
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);

    return SingleChildScrollView(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          for (final category in kIconCategories) ...[
            Padding(
              padding: const EdgeInsets.only(top: 16, bottom: 8),
              child: Text(
                _categoryLabel(l10n, category.nameKey),
                style: theme.textTheme.labelSmall?.copyWith(
                  color: theme.colorScheme.onSurfaceVariant,
                  fontWeight: FontWeight.w600,
                  letterSpacing: 0.8,
                ),
              ),
            ),
            Wrap(
              spacing: spacing,
              runSpacing: spacing,
              children: category.iconNames.map((name) {
                final isSelected = name == currentIcon;
                return Material(
                  color: isSelected
                      ? theme.colorScheme.primary.withValues(alpha: 0.15)
                      : theme.colorScheme.surfaceContainerHighest,
                  borderRadius: BorderRadius.circular(10),
                  child: InkWell(
                    borderRadius: BorderRadius.circular(10),
                    onTap: () => onSelected(name),
                    child: Container(
                      width: iconSize,
                      height: iconSize,
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
        ],
      ),
    );
  }
}
