import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart';
import 'package:solosoul_flutter/presentation/utils/auth_utils.dart';
import 'package:solosoul_flutter/presentation/widgets/header_action_buttons.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';
import 'package:solosoul_flutter/presentation/providers/operation_log_provider.dart';
import 'package:solosoul_flutter/presentation/widgets/operation_filter_chip.dart';
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
    return _buildLogView();
  }

  Widget _buildLogView() {
    final theme = Theme.of(context);
    final entries = ref.watch(operationLogFilteredEntriesProvider);

    // Apply search filter on top of action/device filters
    final filteredEntries = _searchQuery.isEmpty
        ? entries
        : entries.where((entry) {
            final lowerQuery = _searchQuery.toLowerCase();
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
              onPressed: () => _confirmClearLog(context),
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
              controller: _searchController,
              onChanged: (value) => setState(() => _searchQuery = value),
              decoration: InputDecoration(
                hintText: 'Search logs...',
                prefixIcon: const Icon(Icons.search),
                suffixIcon: _searchQuery.isNotEmpty
                    ? IconButton(
                        icon: const Icon(Icons.clear),
                        onPressed: () {
                          _searchController.clear();
                          setState(() => _searchQuery = '');
                        },
                      )
                    : null,
                border: OutlineInputBorder(borderRadius: BorderRadius.circular(12)),
                contentPadding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
              ),
            ),
          ),
          _buildFilterHeader(),
          AnimatedSwitcher(
            duration: const Duration(milliseconds: 300),
            child: _filterExpanded ? _buildFilterSection() : const SizedBox.shrink(),
          ),
          if (_searchQuery.isNotEmpty && filteredEntries.isNotEmpty)
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
                          _searchQuery.isNotEmpty ? Icons.search_off : Icons.filter_list_off,
                          size: 64,
                          color: theme.colorScheme.onSurfaceVariant,
                        ),
                        const SizedBox(height: 16),
                        Text(
                          _searchQuery.isNotEmpty ? 'No matching entries' : 'No matching entries',
                          style: theme.textTheme.titleMedium?.copyWith(
                            color: theme.colorScheme.onSurfaceVariant,
                          ),
                        ),
                        const SizedBox(height: 8),
                        Text(
                          _searchQuery.isNotEmpty
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

  Widget _buildFilterHeader() {
    final theme = Theme.of(context);
    final actionFilters = ref.watch(logActionFilterProvider);
    final deviceFilters = ref.watch(logDeviceFilterProvider);
    final hasActiveFilters = actionFilters.isNotEmpty || deviceFilters.isNotEmpty;

    return InkWell(
      onTap: () => setState(() => _filterExpanded = !_filterExpanded),
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
              turns: _filterExpanded ? 0.5 : 0,
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

  Widget _buildFilterSection() {
    final theme = Theme.of(context);
    final actionFilters = ref.watch(logActionFilterProvider);
    final deviceFilters = ref.watch(logDeviceFilterProvider);

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
          Row(
            children: [
              Text(
                'Action:',
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
                      OperationFilterChip(
                        label: 'Create',
                        icon: Icons.add_circle_outline,
                        isSelected: actionFilters.contains('create'),
                        color: AppTheme.successColor,
                        onSelected: (_) => _toggleFilter(actionFilters, 'create', logActionFilterProvider),
                      ),
                      const SizedBox(width: 4),
                      OperationFilterChip(
                        label: 'Update',
                        icon: Icons.edit_outlined,
                        isSelected: actionFilters.contains('update'),
                        color: AppTheme.primaryColor,
                        onSelected: (_) => _toggleFilter(actionFilters, 'update', logActionFilterProvider),
                      ),
                      const SizedBox(width: 4),
                      OperationFilterChip(
                        label: 'Delete',
                        icon: Icons.delete_outline,
                        isSelected: actionFilters.contains('delete'),
                        color: Colors.orange.shade700,
                        onSelected: (_) => _toggleFilter(actionFilters, 'delete', logActionFilterProvider),
                      ),
                      const SizedBox(width: 4),
                      OperationFilterChip(
                        label: 'Restore',
                        icon: Icons.restore,
                        isSelected: actionFilters.contains('restore'),
                        color: Colors.blue,
                        onSelected: (_) => _toggleFilter(actionFilters, 'restore', logActionFilterProvider),
                      ),
                      const SizedBox(width: 4),
                      OperationFilterChip(
                        label: 'Purge',
                        icon: Icons.delete_forever,
                        isSelected: actionFilters.contains('purge'),
                        color: AppTheme.errorColor,
                        onSelected: (_) => _toggleFilter(actionFilters, 'purge', logActionFilterProvider),
                      ),
                    ],
                  ),
                ),
              ),
            ],
          ),
          const SizedBox(height: 8),
          Row(
            children: [
              Text(
                'Device:',
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
                      OperationFilterChip(
                        label: 'macOS',
                        icon: Icons.laptop_mac,
                        isSelected: deviceFilters.contains('macos'),
                        color: Colors.grey.shade700,
                        onSelected: (_) => _toggleFilter(deviceFilters, 'macos', logDeviceFilterProvider),
                      ),
                      const SizedBox(width: 4),
                      OperationFilterChip(
                        label: 'iOS',
                        icon: Icons.phone_iphone,
                        isSelected: deviceFilters.contains('ios'),
                        color: Colors.grey.shade700,
                        onSelected: (_) => _toggleFilter(deviceFilters, 'ios', logDeviceFilterProvider),
                      ),
                      const SizedBox(width: 4),
                      OperationFilterChip(
                        label: 'Android',
                        icon: Icons.phone_android,
                        isSelected: deviceFilters.contains('android'),
                        color: Colors.grey.shade700,
                        onSelected: (_) => _toggleFilter(deviceFilters, 'android', logDeviceFilterProvider),
                      ),
                      const SizedBox(width: 4),
                      OperationFilterChip(
                        label: 'Web',
                        icon: Icons.web,
                        isSelected: deviceFilters.contains('web'),
                        color: Colors.grey.shade700,
                        onSelected: (_) => _toggleFilter(deviceFilters, 'web', logDeviceFilterProvider),
                      ),
                    ],
                  ),
                ),
              ),
              if (actionFilters.isNotEmpty || deviceFilters.isNotEmpty)
                TextButton.icon(
                  onPressed: _clearAllFilters,
                  icon: const Icon(Icons.clear_all, size: 16),
                  label: const Text('Clear'),
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

  void _toggleFilter<T>(
    Set<T> currentFilters,
    T value,
    dynamic provider,
  ) {
    final newFilters = Set<T>.from(currentFilters);
    if (newFilters.contains(value)) {
      newFilters.remove(value);
    } else {
      newFilters.add(value);
    }
    // ignore: avoid_dynamic_calls
    ref.read(provider.notifier).state = newFilters;
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
