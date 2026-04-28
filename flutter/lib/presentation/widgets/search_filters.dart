import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/presentation/providers/search_provider.dart';
import 'package:solosoul_flutter/presentation/models/search_models.dart' show SearchState;

/// Search filter chips widget for filtering by sensitivity level.
class SearchFilters extends ConsumerWidget {
  final SearchState searchState;

  const SearchFilters({
    super.key,
    required this.searchState,
  });

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 16),
      child: Wrap(
        spacing: 8,
        runSpacing: 8,
        crossAxisAlignment: WrapCrossAlignment.center,
        children: [
          FilterChip(
            label: const Text('Public'),
            selected: searchState.searchPublic,
            onSelected: (_) {
              ref.read(searchProvider.notifier).togglePublic();
            },
            avatar: searchState.searchPublic
                ? const Icon(Icons.check, size: 18)
                : null,
          ),
          FilterChip(
            label: const Text('Internal'),
            selected: searchState.searchInternal,
            onSelected: (_) {
              ref.read(searchProvider.notifier).toggleInternal();
            },
            avatar: searchState.searchInternal
                ? const Icon(Icons.check, size: 18)
                : null,
          ),
          FilterChip(
            label: const Text('Sensitive'),
            selected: searchState.searchSensitive,
            onSelected: (_) {
              ref.read(searchProvider.notifier).toggleSensitive();
            },
            avatar: searchState.searchSensitive
                ? const Icon(Icons.check, size: 18)
                : null,
          ),
          FilterChip(
            label: const Text('Restricted'),
            selected: searchState.searchRestricted,
            onSelected: (_) {
              ref.read(searchProvider.notifier).toggleRestricted();
            },
            avatar: searchState.searchRestricted
                ? const Icon(Icons.check, size: 18)
                : null,
          ),
          if (searchState.searchRestricted)
            TextButton.icon(
              icon: const Icon(Icons.lock_open, size: 18),
              label: const Text('Unlock'),
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