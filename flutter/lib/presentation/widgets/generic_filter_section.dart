import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';

/// Filter option configuration for [GenericFilterSection].
class FilterOption<T> {
  final T id;
  final String label;
  final IconData icon;
  final Color color;

  const FilterOption({
    required this.id,
    required this.label,
    required this.icon,
    required this.color,
  });
}

/// Filter group configuration for [GenericFilterSection].
class FilterGroup<T> {
  final String label;
  final List<FilterOption<T>> options;
  final Set<T> selectedIds;
  final ValueChanged<Set<T>> onSelectionChanged;
  final bool singleSelect;

  const FilterGroup({
    required this.label,
    required this.options,
    required this.selectedIds,
    required this.onSelectionChanged,
    this.singleSelect = false,
  });
}

/// Reusable horizontal filter section with consistent styling.
/// UI and behavior are identical to [OperationLogFilterSection].
/// Includes optional header row with icon, label, filter count badge, and collapse toggle.
class GenericFilterSection<T> extends ConsumerWidget {
  const GenericFilterSection({
    super.key,
    required this.filterGroups,
    this.showHeader = true,
    this.headerIcon = Icons.filter_list,
    this.headerLabel,
    this.collapsible = true,
    this.expanded = true,
    this.onToggle,
    this.showClearAll = false,
    this.onClearAll,
    required this.resultCount,
  });

  final List<FilterGroup<T>> filterGroups;
  final bool showHeader;
  final IconData headerIcon;
  final String? headerLabel;
  final bool collapsible;
  final bool expanded;
  final VoidCallback? onToggle;
  final bool showClearAll;
  final VoidCallback? onClearAll;
  final int resultCount;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);
    final l10n = AppLocalizations.of(context);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        // Header row
        InkWell(
          onTap: collapsible ? onToggle : null,
          child: Container(
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
            decoration: BoxDecoration(
              color: theme.colorScheme.surfaceContainerHighest.withValues(alpha: 0.3),
              border: Border(
                bottom: BorderSide(color: theme.colorScheme.outlineVariant),
              ),
            ),
            child: Row(
              children: [
                Icon(
                  headerIcon,
                  size: 20,
                  color: theme.colorScheme.onSurfaceVariant,
                ),
                const SizedBox(width: 8),
                Text(
                  headerLabel ?? l10n.operationLogFilters,
                  style: theme.textTheme.titleSmall?.copyWith(
                    color: theme.colorScheme.onSurfaceVariant,
                  ),
                ),
                const SizedBox(width: 8),
                Container(
                  padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                  decoration: BoxDecoration(
                    color: theme.colorScheme.primary.withValues(alpha: 0.1),
                    borderRadius: BorderRadius.circular(10),
                  ),
                  child: Text(
                    '$resultCount',
                    style: theme.textTheme.labelSmall?.copyWith(
                      color: theme.colorScheme.primary,
                      fontWeight: FontWeight.bold,
                    ),
                  ),
                ),
                const Spacer(),
                if (collapsible)
                  AnimatedRotation(
                    turns: expanded ? 0.5 : 0,
                    duration: const Duration(milliseconds: 300),
                    child: Icon(
                      Icons.keyboard_arrow_down,
                      color: theme.colorScheme.onSurfaceVariant,
                    ),
                  ),
              ],
            ),
          ),
        ),
        // Filter content
        if (expanded)
          Container(
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
            decoration: BoxDecoration(
              color: theme.colorScheme.surfaceContainerHighest.withValues(alpha: 0.3),
              border: Border(
                bottom: BorderSide(color: theme.colorScheme.outlineVariant),
              ),
            ),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                for (int groupIndex = 0; groupIndex < filterGroups.length; groupIndex++) ...[
                  if (groupIndex > 0) const SizedBox(height: 8),
                  _buildFilterRow(context, ref, filterGroups[groupIndex]),
                ],
                if (showClearAll && _hasActiveFilters)
                  TextButton.icon(
                    onPressed: onClearAll,
                    icon: const Icon(Icons.clear_all, size: 16),
                    label: Text(l10n.genericFilterClearAll),
                    style: TextButton.styleFrom(
                      padding: const EdgeInsets.symmetric(horizontal: 8),
                      minimumSize: Size.zero,
                    ),
                  ),
              ],
            ),
          ),
      ],
    );
  }

  bool get _hasActiveFilters {
    for (final group in filterGroups) {
      if (group.selectedIds.isNotEmpty) {
        // singleSelect group with 'all' selected means no active filter
        if (group.singleSelect && group.selectedIds.contains('all')) continue;
        return true;
      }
    }
    return false;
  }

  Widget _buildFilterRow(BuildContext context, WidgetRef ref, FilterGroup<T> group) {
    final theme = Theme.of(context);

    return Row(
      children: [
        if (group.label.isNotEmpty) ...[
          Text(
            group.label,
            style: theme.textTheme.labelMedium?.copyWith(
              color: theme.colorScheme.onSurfaceVariant,
            ),
          ),
          const SizedBox(width: 8),
        ],
        Expanded(
          child: _ScrollableChipRow(
            children: [
              for (int i = 0; i < group.options.length; i++) ...[
                _FilterChip(
                  label: group.options[i].label,
                  icon: group.options[i].icon,
                  isSelected: group.selectedIds.contains(group.options[i].id),
                  color: group.options[i].color,
                  onSelected: (_) {
                    final newSelection = Set<T>.from(group.selectedIds);
                    if (group.singleSelect) {
                      newSelection
                        ..clear()
                        ..add(group.options[i].id);
                    } else {
                      if (newSelection.contains(group.options[i].id)) {
                        newSelection.remove(group.options[i].id);
                      } else {
                        newSelection.add(group.options[i].id);
                      }
                    }
                    group.onSelectionChanged(newSelection);
                  },
                ),
                if (i < group.options.length - 1) const SizedBox(width: 4),
              ],
            ],
          ),
        ),
      ],
    );
  }
}

/// Horizontal scrollable row with overscroll support.
class _ScrollableChipRow extends StatelessWidget {
  final List<Widget> children;
  const _ScrollableChipRow({required this.children});

  @override
  Widget build(BuildContext context) {
    return SingleChildScrollView(
      scrollDirection: Axis.horizontal,
      physics: const ClampingScrollPhysics(),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: children,
      ),
    );
  }
}

/// Filter chip styled identically to [OperationFilterChip].
class _FilterChip extends StatelessWidget {
  const _FilterChip({
    required this.label,
    required this.icon,
    required this.isSelected,
    required this.color,
    required this.onSelected,
  });

  final String label;
  final IconData icon;
  final bool isSelected;
  final Color color;
  final ValueChanged<bool> onSelected;

  @override
  Widget build(BuildContext context) {
    return FilterChip(
      label: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(icon, size: 14, color: isSelected ? Colors.white : color),
          const SizedBox(width: 4),
          Text(label),
        ],
      ),
      selected: isSelected,
      onSelected: onSelected,
      backgroundColor: color.withValues(alpha: 0.1),
      selectedColor: color,
      checkmarkColor: Colors.white,
      labelStyle: TextStyle(
        color: isSelected ? Colors.white : color,
        fontSize: 12,
        fontWeight: FontWeight.w500,
      ),
      padding: const EdgeInsets.symmetric(horizontal: 4),
      visualDensity: VisualDensity.compact,
      side: BorderSide(color: color.withValues(alpha: 0.3)),
    );
  }
}
