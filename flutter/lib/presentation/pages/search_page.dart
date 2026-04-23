import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/presentation/models/search_models.dart'
    show SearchResultItem, SearchState;
import 'package:solosoul_flutter/presentation/providers/search_provider.dart';
import 'package:solosoul_flutter/presentation/widgets/search_filters.dart';
import 'package:solosoul_flutter/presentation/widgets/search_result_tile.dart';
import 'package:solosoul_flutter/presentation/widgets/search_empty_state.dart';
import 'package:solosoul_flutter/presentation/widgets/history_sheet.dart'
    show HistorySheet;

/// Search Page
class SearchPage extends ConsumerStatefulWidget {
  const SearchPage({super.key});

  @override
  ConsumerState<SearchPage> createState() => _SearchPageState();
}

class _SearchPageState extends ConsumerState<SearchPage> {
  final _searchController = TextEditingController();
  final _focusNode = FocusNode();

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _focusNode.requestFocus();
    });
  }

  @override
  void dispose() {
    _searchController.dispose();
    _focusNode.dispose();
    super.dispose();
  }

  void _showHistorySheet(BuildContext context) {
    showModalBottomSheet(
      context: context,
      isScrollControlled: true,
      builder: (context) => DraggableScrollableSheet(
        initialChildSize: 0.7,
        minChildSize: 0.5,
        maxChildSize: 0.95,
        expand: false,
        builder: (context, scrollController) {
          return HistorySheet(
            scrollController: scrollController,
            ref: ref,
          );
        },
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final searchState = ref.watch(searchProvider);
    final theme = Theme.of(context);

    return Scaffold(
      appBar: AppBar(
        title: const Text('Search'),
        actions: [
          IconButton(
            icon: const Icon(Icons.history),
            tooltip: 'Field History',
            onPressed: () => _showHistorySheet(context),
          ),
        ],
      ),
      body: Column(
        children: [
          Padding(
            padding: const EdgeInsets.all(16),
            child: TextField(
              controller: _searchController,
              focusNode: _focusNode,
              decoration: InputDecoration(
                hintText: 'Search fields...',
                prefixIcon: const Icon(Icons.search),
                suffixIcon: _searchController.text.isNotEmpty
                    ? IconButton(
                        icon: const Icon(Icons.clear),
                        onPressed: () {
                          _searchController.clear();
                          ref.read(searchProvider.notifier).setQuery('');
                        },
                      )
                    : null,
                border: OutlineInputBorder(
                  borderRadius: BorderRadius.circular(12),
                ),
              ),
              onChanged: (value) {
                ref.read(searchProvider.notifier).setQuery(value);
              },
            ),
          ),
          SearchFilters(searchState: searchState),
          const Divider(height: 24),
          Expanded(child: _buildResults(searchState, theme)),
        ],
      ),
    );
  }

  Widget _buildResults(SearchState searchState, ThemeData theme) {
    if (searchState.query.isEmpty) {
      return const SearchEmptyState();
    }

    if (searchState.isSearching) {
      return const SearchLoadingState();
    }

    if (searchState.results.isEmpty) {
      return const SearchNoResultsState();
    }

    final groupedResults = <String, List<SearchResultItem>>{};
    for (final result in searchState.results) {
      groupedResults.putIfAbsent(result.sectionDisplayName, () => []);
      groupedResults[result.sectionDisplayName]!.add(result);
    }

    return ListView.builder(
      padding: const EdgeInsets.symmetric(horizontal: 16),
      itemCount: groupedResults.length,
      itemBuilder: (context, index) {
        final sectionName = groupedResults.keys.elementAt(index);
        final sectionResults = groupedResults[sectionName]!;

        return Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Padding(
              padding: const EdgeInsets.symmetric(vertical: 8),
              child: Text(
                sectionName,
                style: theme.textTheme.titleSmall?.copyWith(
                  fontWeight: FontWeight.bold,
                ),
              ),
            ),
            ...sectionResults.map(
              (result) => SearchResultTile(
                result: result,
                onReveal: () {
                  ref
                      .read(searchProvider.notifier)
                      .revealFieldWithContext(
                        context,
                        ref,
                        result.sensitivityLevel,
                        result.fieldPath,
                      );
                },
              ),
            ),
            const SizedBox(height: 8),
          ],
        );
      },
    );
  }
}