import 'package:flutter/material.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/core/router/app_router.dart';
import 'package:solosoul_flutter/presentation/theme/glass_adapters.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/presentation/models/search_models.dart'
    show SearchResultItem, SearchState;
import 'package:solosoul_flutter/presentation/providers/search_provider.dart';
import 'package:solosoul_flutter/presentation/widgets/search_filters.dart';
import 'package:solosoul_flutter/presentation/widgets/search_result_tile.dart';
import 'package:solosoul_flutter/presentation/widgets/search_empty_state.dart';
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

  @override
  Widget build(BuildContext context) {
    final searchState = ref.watch(searchProvider);
    final theme = Theme.of(context);

    return Scaffold(
      appBar: SoloGlassAppBar(
        backRoute: AppRoutes.home,
        title: Text(AppLocalizations.of(context).searchTitle),
      ),
      body: Column(
        children: [
          Padding(
            padding: const EdgeInsets.all(16),
            child: TextField(
              controller: _searchController,
              focusNode: _focusNode,
              decoration: InputDecoration(
                hintText: AppLocalizations.of(context).searchHint,
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
          SearchFilters(resultCount: searchState.results.length),
          Expanded(
            child: _SearchResultsWidget(
              searchState: searchState,
              theme: theme,
              ref: ref,
            ),
          ),
        ],
      ),
    );
  }


}

// =============================================================================
// Sticky section header for search results
// =============================================================================

class _SearchHeaderDelegate extends SliverPersistentHeaderDelegate {
  final String title;
  final TextStyle? style;

  _SearchHeaderDelegate(this.title, {this.style});

  @override
  Widget build(BuildContext context, double shrinkOffset, bool overlapsContent) {
    final theme = Theme.of(context);
    return Container(
      color: theme.colorScheme.surface,
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
      alignment: Alignment.centerLeft,
      child: Text(
        title,
        style: style ??
            theme.textTheme.titleSmall?.copyWith(
              fontWeight: FontWeight.bold,
            ),
      ),
    );
  }

  @override
  double get maxExtent => 40;

  @override
  double get minExtent => 40;

  @override
  bool shouldRebuild(covariant _SearchHeaderDelegate oldDelegate) =>
      oldDelegate.title != title || oldDelegate.style != style;
}
class _SearchResultsWidget extends StatelessWidget {
  final SearchState searchState;
  final ThemeData theme;
  final WidgetRef ref;

  const _SearchResultsWidget({
    required this.searchState,
    required this.theme,
    required this.ref,
  });

  @override
  Widget build(BuildContext context) {
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

    // 按 section 名称排序，确保 UI 顺序稳定
    final sortedKeys = groupedResults.keys.toList()..sort();
    final headerStyle = theme.textTheme.titleSmall?.copyWith(
      fontWeight: FontWeight.bold,
    );

    return CustomScrollView(
      slivers: [
        for (final sectionName in sortedKeys) ...[
          SliverPersistentHeader(
            pinned: true,
            delegate: _SearchHeaderDelegate(sectionName, style: headerStyle),
          ),
          SliverPadding(
            padding: const EdgeInsets.symmetric(horizontal: 16),
            sliver: SliverList(
              delegate: SliverChildBuilderDelegate(
                (context, index) {
                  final result = groupedResults[sectionName]![index];
                  return SearchResultTile(
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
                  );
                },
                childCount: groupedResults[sectionName]!.length,
              ),
            ),
          ),
        ],
      ],
    );
  }
}
