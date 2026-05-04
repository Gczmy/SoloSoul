import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart';
import 'package:solosoul_flutter/presentation/utils/auth_utils.dart';
import 'package:solosoul_flutter/presentation/widgets/header_action_buttons.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';
import 'package:solosoul_flutter/presentation/providers/operation_log_provider.dart';
import 'package:solosoul_flutter/presentation/widgets/operation_log_filter_section.dart';
import 'package:solosoul_flutter/presentation/models/operation_log_models.dart'
    show OperationEntry;
import 'package:solosoul_flutter/presentation/widgets/operation_tile.dart';

class OperationLogPage extends ConsumerStatefulWidget {
  const OperationLogPage({super.key});

  @override
  ConsumerState<OperationLogPage> createState() => _OperationLogPageState();
}

class _OperationLogPageState extends ConsumerState<OperationLogPage> {
  bool _filterExpanded = false;
  final _searchController = TextEditingController();
  String _searchQuery = '';

  @override
  void dispose() {
    _searchController.dispose();
    super.dispose();
  }

  @override
  void initState() {
    super.initState();
    _refreshLogs();
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted && !ref.read(isSensitiveAccessGrantedProvider)) {
        _verifyPassword();
      }
    });
  }

  Future<void> _refreshLogs() async {
    await OperationLogService.instance.refreshFromDisk();
  }

  Future<void> _verifyPassword() async {
    await verifyPasswordAndGrantAccess(
      context: context,
      ref: ref,
      message: 'Enter your master password to view the operation log.',
    );
  }

  @override
  Widget build(BuildContext context) {
    if (!ref.watch(isSensitiveAccessGrantedProvider)) {
      return Scaffold(
        appBar: AppBar(title: const Text('Operation Log')),
        body: Center(
          child: Column(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              Icon(
                Icons.lock_outline,
                size: 64,
                color: Theme.of(context).colorScheme.primary,
              ),
              const SizedBox(height: 24),
              Text(
                'Password Required',
                style: Theme.of(context).textTheme.headlineSmall,
              ),
              const SizedBox(height: 16),
              ElevatedButton(
                onPressed: _verifyPassword,
                child: const Text('Verify'),
              ),
            ],
          ),
        ),
      );
    }
    final entries = ref.watch(operationLogFilteredEntriesProvider);
    return _OperationLogView(
      searchController: _searchController,
      searchQuery: _searchQuery,
      entries: entries,
      onSearchChanged: (value) => setState(() => _searchQuery = value),
      onClearSearch: () {
        _searchController.clear();
        setState(() => _searchQuery = '');
      },
      onConfirmClearLog: () => _confirmClearLog(context),
      filterExpanded: _filterExpanded,
      onClearAllFilters: _clearAllFilters,
      filterHeader: _OperationLogFilterHeader(
        filterExpanded: _filterExpanded,
        actionFilters: ref.watch(logActionFilterProvider),
        deviceFilters: ref.watch(logDeviceFilterProvider),
        onToggle: () => setState(() => _filterExpanded = !_filterExpanded),
      ),
    );
  }

  void _clearAllFilters() {
    ref.read(logActionFilterProvider.notifier).clear();
    ref.read(logDeviceFilterProvider.notifier).clear();
  }

  void _confirmClearLog(BuildContext context) {
    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Clear Log'),
        content: const Text(
          'Are you sure you want to clear all operation history?',
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: const Text('Cancel'),
          ),
          TextButton(
            onPressed: () {
              OperationLogService.instance.clearEntries();
              Navigator.pop(context);
            },
            child: const Text('Clear', style: TextStyle(color: AppTheme.errorColor)),
          ),
        ],
      ),
    );
  }
}

class _OperationLogView extends StatelessWidget {
  final TextEditingController searchController;
  final String searchQuery;
  final List<OperationEntry> entries;
  final ValueChanged<String> onSearchChanged;
  final VoidCallback onClearSearch;
  final VoidCallback onConfirmClearLog;
  final bool filterExpanded;
  final VoidCallback onClearAllFilters;
  final Widget filterHeader;

  const _OperationLogView({
    required this.searchController,
    required this.searchQuery,
    required this.entries,
    required this.onSearchChanged,
    required this.onClearSearch,
    required this.onConfirmClearLog,
    required this.filterExpanded,
    required this.onClearAllFilters,
    required this.filterHeader,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final filteredEntries = searchQuery.isEmpty
        ? entries
        : entries.where((entry) {
            final lowerQuery = searchQuery.toLowerCase();
            return entry.description.toLowerCase().contains(lowerQuery) ||
                entry.section.toLowerCase().contains(lowerQuery) ||
                entry.action.toLowerCase().contains(lowerQuery);
          }).toList();

    return Scaffold(
      appBar: AppBar(
        title: const Text('Operation Log'),
        actions: [
          const HeaderActionButtons(),
          if (entries.isNotEmpty)
            IconButton(
              icon: const Icon(Icons.delete_outline),
              onPressed: onConfirmClearLog,
              tooltip: 'Clear log',
            ),
        ],
      ),
      body: Column(
        children: [
          // Search bar
          Padding(
            padding: const EdgeInsets.all(16),
            child: TextField(
              controller: searchController,
              onChanged: onSearchChanged,
              decoration: InputDecoration(
                hintText: 'Search logs...',
                prefixIcon: const Icon(Icons.search),
                suffixIcon: searchQuery.isNotEmpty
                    ? IconButton(
                        icon: const Icon(Icons.clear),
                        onPressed: onClearSearch,
                      )
                    : null,
                border: OutlineInputBorder(borderRadius: BorderRadius.circular(12)),
                contentPadding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
              ),
            ),
          ),
          filterHeader,
          AnimatedSwitcher(
            duration: const Duration(milliseconds: 300),
            child: filterExpanded
                ? OperationLogFilterSection(onClearAll: onClearAllFilters)
                : const SizedBox.shrink(),
          ),
          if (searchQuery.isNotEmpty && filteredEntries.isNotEmpty)
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 16),
              child: Align(
                alignment: Alignment.centerLeft,
                child: Text(
                  'Found ${filteredEntries.length} result${filteredEntries.length == 1 ? '' : 's'}',
                  style: theme.textTheme.bodySmall?.copyWith(
                    color: theme.colorScheme.onSurfaceVariant,
                  ),
                ),
              ),
            ),
          Expanded(
            child: filteredEntries.isEmpty
                ? Center(
                    child: Column(
                      mainAxisAlignment: MainAxisAlignment.center,
                      children: [
                        Icon(
                          searchQuery.isNotEmpty ? Icons.search_off : Icons.filter_list_off,
                          size: 64,
                          color: theme.colorScheme.onSurfaceVariant,
                        ),
                        const SizedBox(height: 16),
                        Text(
                          searchQuery.isNotEmpty ? 'No matching entries' : 'No matching entries',
                          style: theme.textTheme.titleMedium?.copyWith(
                            color: theme.colorScheme.onSurfaceVariant,
                          ),
                        ),
                        const SizedBox(height: 8),
                        Text(
                          searchQuery.isNotEmpty
                              ? 'Try a different search term'
                              : 'Try adjusting your filters',
                          style: theme.textTheme.bodySmall?.copyWith(
                            color: theme.colorScheme.onSurfaceVariant,
                          ),
                        ),
                      ],
                    ),
                  )
                : ListView.separated(
                    padding: const EdgeInsets.all(16),
                    itemCount: filteredEntries.length,
                    separatorBuilder: (_, a) => const SizedBox(height: 8),
                    itemBuilder: (context, index) {
                      final entry = filteredEntries[index];
                      return OperationTile(entry: entry);
                    },
                  ),
          ),
        ],
      ),
    );
  }
}

class _OperationLogFilterHeader extends StatelessWidget {
  final bool filterExpanded;
  final Set<String> actionFilters;
  final Set<String> deviceFilters;
  final VoidCallback onToggle;

  const _OperationLogFilterHeader({
    required this.filterExpanded,
    required this.actionFilters,
    required this.deviceFilters,
    required this.onToggle,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final hasActiveFilters = actionFilters.isNotEmpty || deviceFilters.isNotEmpty;

    return InkWell(
      onTap: onToggle,
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
              Icons.filter_list,
              size: 20,
              color: hasActiveFilters
                  ? theme.colorScheme.primary
                  : theme.colorScheme.onSurfaceVariant,
            ),
            const SizedBox(width: 8),
            Text(
              'Filters',
              style: theme.textTheme.titleSmall?.copyWith(
                color: hasActiveFilters
                    ? theme.colorScheme.primary
                    : theme.colorScheme.onSurfaceVariant,
              ),
            ),
            if (hasActiveFilters) ...[
              const SizedBox(width: 8),
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                decoration: BoxDecoration(
                  color: theme.colorScheme.primary.withValues(alpha: 0.1),
                  borderRadius: BorderRadius.circular(10),
                ),
                child: Text(
                  '${actionFilters.length + deviceFilters.length}',
                  style: theme.textTheme.labelSmall?.copyWith(
                    color: theme.colorScheme.primary,
                    fontWeight: FontWeight.bold,
                  ),
                ),
              ),
            ],
            const Spacer(),
            AnimatedRotation(
              turns: filterExpanded ? 0.5 : 0,
              duration: const Duration(milliseconds: 300),
              child: Icon(
                Icons.keyboard_arrow_down,
                color: theme.colorScheme.onSurfaceVariant,
              ),
            ),
          ],
        ),
      ),
    );
  }
}
