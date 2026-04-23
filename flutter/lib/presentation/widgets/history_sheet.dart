import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/presentation/providers/profile_provider.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart';
import 'package:solosoul_flutter/presentation/widgets/history_change_tile.dart';

/// History change item data class.
class HistoryChangeItem {
  final String itemId;
  final String fieldId;
  final Map<String, String> values;
  final DateTime timestamp;

  const HistoryChangeItem({
    required this.itemId,
    required this.fieldId,
    required this.values,
    required this.timestamp,
  });
}

/// Bottom sheet displaying field change history.
class HistorySheet extends StatelessWidget {
  final ScrollController scrollController;
  final WidgetRef ref;

  const HistorySheet({
    super.key,
    required this.scrollController,
    required this.ref,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Container(
      decoration: BoxDecoration(
        color: theme.colorScheme.surface,
        borderRadius: const BorderRadius.vertical(top: Radius.circular(16)),
      ),
      child: Column(
        children: [
          // Handle bar
          Container(
            margin: const EdgeInsets.only(top: 12),
            width: 40,
            height: 4,
            decoration: BoxDecoration(
              color: theme.colorScheme.outline.withValues(alpha: 0.4),
              borderRadius: BorderRadius.circular(2),
            ),
          ),
          // Title
          Padding(
            padding: const EdgeInsets.all(16),
            child: Row(
              children: [
                const Icon(Icons.history, color: AppTheme.primaryColor),
                const SizedBox(width: 8),
                Text(
                  'Field History',
                  style: theme.textTheme.titleLarge?.copyWith(
                    fontWeight: FontWeight.bold,
                  ),
                ),
              ],
            ),
          ),
          const Divider(height: 1),
          // History list
          Expanded(
            child: FutureBuilder(
              future: ref.read(fieldHistoriesProvider.notifier).loadHistories(),
              builder: (context, snapshot) {
                if (snapshot.connectionState == ConnectionState.waiting) {
                  return const Center(child: CircularProgressIndicator());
                }

                final histories = ref.watch(fieldHistoriesProvider);
                final allChanges = <HistoryChangeItem>[];

                // Collect all changes from all histories
                for (final itemEntry in histories.histories.entries) {
                  final itemId = itemEntry.key;
                  for (final fieldEntry in itemEntry.value.entries) {
                    final fieldId = fieldEntry.key;
                    final history = fieldEntry.value;
                    for (final entry in history.entries) {
                      allChanges.add(
                        HistoryChangeItem(
                          itemId: itemId,
                          fieldId: fieldId,
                          values: entry.values,
                          timestamp: entry.timestamp,
                        ),
                      );
                    }
                  }
                }

                // Sort by timestamp (newest first)
                allChanges.sort((a, b) => b.timestamp.compareTo(a.timestamp));

                if (allChanges.isEmpty) {
                  return Center(
                    child: Column(
                      mainAxisAlignment: MainAxisAlignment.center,
                      children: [
                        Icon(
                          Icons.history,
                          size: 64,
                          color: theme.colorScheme.outline,
                        ),
                        const SizedBox(height: 16),
                        Text(
                          'No history yet',
                          style: theme.textTheme.bodyLarge?.copyWith(
                            color: theme.colorScheme.outline,
                          ),
                        ),
                      ],
                    ),
                  );
                }

                return ListView.builder(
                  controller: scrollController,
                  padding: const EdgeInsets.symmetric(horizontal: 16),
                  itemCount: allChanges.length,
                  itemBuilder: (context, index) {
                    final change = allChanges[index];
                    return HistoryChangeTile(change: change, theme: theme);
                  },
                );
              },
            ),
          ),
        ],
      ),
    );
  }
}