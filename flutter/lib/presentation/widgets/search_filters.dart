import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/providers/search_provider.dart';
import 'package:solosoul_flutter/presentation/providers/sensitivity_provider.dart';
import 'package:solosoul_flutter/presentation/widgets/generic_filter_section.dart';

class SearchFilters extends ConsumerStatefulWidget {
  const SearchFilters({
    super.key,
    required this.resultCount,
  });

  final int resultCount;

  @override
  ConsumerState<SearchFilters> createState() => _SearchFiltersState();
}

class _SearchFiltersState extends ConsumerState<SearchFilters> {
  bool _collapsed = false;

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final searchState = ref.watch(searchProvider);
    final selectedIds = <SensitivityLevel>{
      if (searchState.searchPublic) SensitivityLevel.public,
      if (searchState.searchInternal) SensitivityLevel.internal,
      if (searchState.searchSensitive) SensitivityLevel.sensitive,
      if (searchState.searchRestricted) SensitivityLevel.critical,
    };

    final sensitivityOptions = [
      FilterOption(
        id: SensitivityLevel.public,
        label: l.sensitivityPublic,
        icon: Icons.public,
        color: Colors.green.shade600,
      ),
      FilterOption(
        id: SensitivityLevel.internal,
        label: l.sensitivityInternal,
        icon: Icons.shield_outlined,
        color: Colors.blue.shade600,
      ),
      FilterOption(
        id: SensitivityLevel.sensitive,
        label: l.sensitivitySensitive,
        icon: Icons.lock_outline,
        color: Colors.orange.shade600,
      ),
      FilterOption(
        id: SensitivityLevel.critical,
        label: l.sensitivityRestricted,
        icon: Icons.lock,
        color: Colors.red.shade600,
      ),
    ];

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        GenericFilterSection<SensitivityLevel>(
          headerLabel: l.operationFilterLabel,
          filterGroups: [
            FilterGroup(
              label: '',
              options: sensitivityOptions,
              selectedIds: selectedIds,
              onSelectionChanged: (ids) {
                final notifier = ref.read(searchProvider.notifier);
                notifier.setFilters(
                  searchPublic: ids.contains(SensitivityLevel.public),
                  searchInternal: ids.contains(SensitivityLevel.internal),
                  searchSensitive: ids.contains(SensitivityLevel.sensitive),
                  searchRestricted: ids.contains(SensitivityLevel.critical),
                );
              },
            ),
          ],
          resultCount: widget.resultCount,
          expanded: !_collapsed,
          onToggle: () => setState(() => _collapsed = !_collapsed),
          showClearAll: true,
          onClearAll: () {
            ref.read(searchProvider.notifier).setFilters(
              searchPublic: true,
              searchInternal: true,
              searchSensitive: true,
              searchRestricted: true,
            );
          },
        ),
        if (searchState.searchRestricted)
          Padding(
            padding: const EdgeInsets.only(left: 16, top: 4, bottom: 4),
            child: TextButton.icon(
              icon: const Icon(Icons.lock_open, size: 18),
              label: Text(l.searchUnlock),
              onPressed: () {
                ref.read(searchProvider.notifier).unlockAllRestricted(context, ref);
              },
            ),
          ),
      ],
    );
  }
}
