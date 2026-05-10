import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/providers/trash_filter_provider.dart';
import 'package:solosoul_flutter/presentation/widgets/generic_filter_section.dart';

class TrashFilterSection extends ConsumerWidget {
  const TrashFilterSection({
    super.key,
    required this.resultCount,
    this.onClearAll,
  });

  final int resultCount;
  final VoidCallback? onClearAll;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);
    final l = AppLocalizations.of(context);
    final timeFilter = ref.watch(trashTimeFilterProvider);
    final typeFilters = ref.watch(trashTypeFilterProvider);

    final timeOptions = timeFilterOptions.map((opt) => FilterOption<String>(
      id: opt.$1,
      label: _getTimeFilterLabel(context, opt.$1),
      icon: Icons.schedule,
      color: theme.colorScheme.primary,
    )).toList();

    final typeOptions = objectTypeIds.entries.map((e) => FilterOption<String>(
      id: e.key,
      label: _getTypeFilterLabel(context, e.key),
      icon: _getTypeIcon(e.key),
      color: _getTypeColor(e.key),
    )).toList();

    return GenericFilterSection<String>(
      headerLabel: l.operationFilterLabel,
      filterGroups: [
        FilterGroup<String>(
          label: l.trashTimeFilterLabel,
          options: timeOptions,
          selectedIds: timeFilter == null ? {'all'} : {timeFilter},
          singleSelect: true,
          onSelectionChanged: (ids) {
            final id = ids.contains('all') ? null : ids.first;
            ref.read(trashTimeFilterProvider.notifier).setFilter(id);
          },
        ),
        FilterGroup<String>(
          label: l.trashTypeFilterLabel,
          options: typeOptions,
          selectedIds: typeFilters,
          onSelectionChanged: (ids) {
            ref.read(trashTypeFilterProvider.notifier).setFilters(ids);
          },
        ),
      ],
      resultCount: resultCount,
      showClearAll: true,
      onClearAll: () {
        ref.read(trashTimeFilterProvider.notifier).clear();
        ref.read(trashTypeFilterProvider.notifier).clear();
        onClearAll?.call();
      },
    );
  }

  String _getTimeFilterLabel(BuildContext context, String id) {
    final l10n = AppLocalizations.of(context);
    switch (id) {
      case 'all':        return l10n.trashTimeFilterAll;
      case '10days':     return l10n.trashTimeFilter10Days;
      case '1day':       return l10n.trashTimeFilter1Day;
      case '6hours':     return l10n.trashTimeFilter6Hours;
      case '1hour':      return l10n.trashTimeFilter1Hour;
      default:           return id;
    }
  }

  String _getTypeFilterLabel(BuildContext context, String typeId) {
    final l10n = AppLocalizations.of(context);
    switch (typeId) {
      case 'page':        return l10n.typePage;
      case 'collection':  return l10n.typeCollection;
      case 'item':        return l10n.typeItem;
      default:            return typeId;
    }
  }

  IconData _getTypeIcon(String typeId) {
    switch (typeId) {
      case 'page':        return Icons.article_outlined;
      case 'collection':   return Icons.folder_outlined;
      case 'item':        return Icons.widgets_outlined;
      default:            return Icons.data_object;
    }
  }

  Color _getTypeColor(String typeId) {
    switch (typeId) {
      case 'page':        return Colors.blue.shade700;
      case 'collection':   return Colors.green.shade700;
      case 'item':        return Colors.orange.shade700;
      default:            return Colors.grey.shade700;
    }
  }
}
