import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/providers/trash_filter_provider.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart';
import 'package:solosoul_flutter/presentation/widgets/operation_filter_chip.dart';

class TrashFilterSection extends ConsumerWidget {
  final VoidCallback? onClearAll;

  const TrashFilterSection({
    super.key,
    this.onClearAll,
  });

  String _getTimeFilterLabel(BuildContext context, String id) {
    final l10n = AppLocalizations.of(context);
    switch (id) {
      case 'all':
        return l10n.trashTimeFilterAll;
      case '10days':
        return l10n.trashTimeFilter10Days;
      case '1day':
        return l10n.trashTimeFilter1Day;
      case '6hours':
        return l10n.trashTimeFilter6Hours;
      case '1hour':
        return l10n.trashTimeFilter1Hour;
      default:
        return id;
    }
  }

  String _getTypeFilterLabel(BuildContext context, String typeId) {
    final l10n = AppLocalizations.of(context);
    switch (typeId) {
      case 'page':
        return l10n.typePage;
      case 'collection':
        return l10n.typeCollection;
      case 'item':
        return l10n.typeItem;
      default:
        return typeId;
    }
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);
    final l10n = AppLocalizations.of(context);
    final timeFilter = ref.watch(trashTimeFilterProvider);
    final typeFilters = ref.watch(trashTypeFilterProvider);

    final hasActiveFilters =
        (timeFilter != null && timeFilter != 'all') || typeFilters.isNotEmpty;

    return Container(
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
          // Time range filter row
          Row(
            children: [
              Text(
                l10n.trashTimeFilterLabel,
                style: theme.textTheme.labelMedium?.copyWith(
                  color: theme.colorScheme.onSurfaceVariant,
                ),
              ),
              const SizedBox(width: 8),
              Expanded(
                child: SingleChildScrollView(
                  scrollDirection: Axis.horizontal,
                  child: Row(
                    children: [
                      for (final (id, _) in timeFilterOptions) ...[
                        OperationFilterChip(
                          label: _getTimeFilterLabel(context, id),
                          icon: Icons.schedule,
                          isSelected: timeFilter == id || (timeFilter == null && id == 'all'),
                          color: AppTheme.primaryColor,
                          onSelected: (_) => ref
                              .read(trashTimeFilterProvider.notifier)
                              .setFilter(id == 'all' ? null : id),
                        ),
                        if (id != timeFilterOptions.last.$1)
                          const SizedBox(width: 4),
                      ],
                    ],
                  ),
                ),
              ),
            ],
          ),
          const SizedBox(height: 8),
          // Type filter row
          Row(
            children: [
              Text(
                l10n.trashTypeFilterLabel,
                style: theme.textTheme.labelMedium?.copyWith(
                  color: theme.colorScheme.onSurfaceVariant,
                ),
              ),
              const SizedBox(width: 8),
              Expanded(
                child: SingleChildScrollView(
                  scrollDirection: Axis.horizontal,
                  child: Row(
                    children: [
                      for (final entry in objectTypeIds.entries) ...[
                        OperationFilterChip(
                          label: _getTypeFilterLabel(context, entry.key),
                          icon: _getTypeIcon(entry.key),
                          isSelected: typeFilters.contains(entry.key),
                          color: _getTypeColor(entry.key),
                          onSelected: (_) => ref
                              .read(trashTypeFilterProvider.notifier)
                              .toggle(entry.key),
                        ),
                        if (entry.key != objectTypeIds.keys.last)
                          const SizedBox(width: 4),
                      ],
                    ],
                  ),
                ),
              ),
              if (hasActiveFilters)
                TextButton.icon(
                  onPressed: () {
                    ref.read(trashTimeFilterProvider.notifier).clear();
                    ref.read(trashTypeFilterProvider.notifier).clear();
                    onClearAll?.call();
                  },
                  icon: const Icon(Icons.clear_all, size: 16),
                  label: Text(l10n.commonClose),
                  style: TextButton.styleFrom(
                    padding: const EdgeInsets.symmetric(horizontal: 8),
                    minimumSize: Size.zero,
                  ),
                ),
            ],
          ),
        ],
      ),
    );
  }

  IconData _getTypeIcon(String typeId) {
    switch (typeId) {
      case 'page':
        return Icons.article_outlined;
      case 'collection':
        return Icons.folder_outlined;
      case 'item':
        return Icons.widgets_outlined;
      default:
        return Icons.data_object;
    }
  }

  Color _getTypeColor(String typeId) {
    switch (typeId) {
      case 'page':
        return Colors.blue.shade700;
      case 'collection':
        return Colors.green.shade700;
      case 'item':
        return Colors.orange.shade700;
      default:
        return Colors.grey.shade700;
    }
  }
}
