import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/presentation/providers/search_provider.dart';
import 'package:solosoul_flutter/presentation/models/search_models.dart' show SearchState;
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';

/// Search filter chips widget for filtering by sensitivity level.
class SearchFilters extends ConsumerWidget {
  final SearchState searchState;

  const SearchFilters({
    super.key,
    required this.searchState,
  });

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);
    final unselectedBg = theme.colorScheme.surfaceContainerHighest;

    ChipThemeData chipTheme(bool selected) {
      return ChipThemeData(
        backgroundColor: selected ? null : unselectedBg,
        side: selected ? null : BorderSide.none,
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
      );
    }

    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 16),
      child: Wrap(
        spacing: 8,
        runSpacing: 8,
        crossAxisAlignment: WrapCrossAlignment.center,
        children: [
          ChipTheme(
            data: chipTheme(searchState.searchPublic),
            child: FilterChip(
              label: Text(l10n.sensitivityPublic),
              selected: searchState.searchPublic,
              onSelected: (_) {
                ref.read(searchProvider.notifier).togglePublic();
              },
              showCheckmark: false,
            ),
          ),
          ChipTheme(
            data: chipTheme(searchState.searchInternal),
            child: FilterChip(
              label: Text(l10n.sensitivityInternal),
              selected: searchState.searchInternal,
              onSelected: (_) {
                ref.read(searchProvider.notifier).toggleInternal();
              },
              showCheckmark: false,
            ),
          ),
          ChipTheme(
            data: chipTheme(searchState.searchSensitive),
            child: FilterChip(
              label: Text(l10n.sensitivitySensitive),
              selected: searchState.searchSensitive,
              onSelected: (_) {
                ref.read(searchProvider.notifier).toggleSensitive();
              },
              showCheckmark: false,
            ),
          ),
          ChipTheme(
            data: chipTheme(searchState.searchRestricted),
            child: FilterChip(
              label: Text(l10n.sensitivityRestricted),
              selected: searchState.searchRestricted,
              onSelected: (_) {
                ref.read(searchProvider.notifier).toggleRestricted();
              },
              showCheckmark: false,
            ),
          ),
          if (searchState.searchRestricted)
            TextButton.icon(
              icon: const Icon(Icons.lock_open, size: 18),
              label: Text(l10n.searchUnlock),
              onPressed: () {
                ref
                    .read(searchProvider.notifier)
                    .unlockAllRestricted(context, ref);
              },
            ),
        ],
      ),
    );
  }
}