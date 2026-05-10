import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart';
import 'package:solosoul_flutter/presentation/theme/glass_adapters.dart';
import 'package:solosoul_flutter/presentation/utils/auth_utils.dart';
import 'package:solosoul_flutter/presentation/widgets/header_action_buttons.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart' show sensitivePageAccessProvider, isSensitiveAccessGrantedProvider;
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
      message: AppLocalizations.of(context).operationLogVerifyPassword,
    );
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    if (!ref.watch(isSensitiveAccessGrantedProvider)) {
      return Scaffold(
        appBar: SoloGlassAppBar(title: Text(l10n.operationLogTitle)),
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
                l10n.operationLogPasswordRequired,
                style: Theme.of(context).textTheme.headlineSmall,
              ),
              const SizedBox(height: 16),
              ElevatedButton(
                onPressed: _verifyPassword,
                child: Text(l10n.operationLogVerify),
              ),
            ],
          ),
        ),
      );
    }
    final entries = ref.watch(operationLogFilteredEntriesProvider);
    // Extend sensitive access timeout on user activity to avoid interrupting active browsing
    return Listener(
      onPointerDown: (_) => ref.read(sensitivePageAccessProvider.notifier).markVerified(),
      onPointerMove: (_) => ref.read(sensitivePageAccessProvider.notifier).markVerified(),
      child: _OperationLogView(
        searchController: _searchController,
        searchQuery: _searchQuery,
        entries: entries,
        onSearchChanged: (value) => setState(() => _searchQuery = value),
        onClearSearch: () {
          _searchController.clear();
          setState(() => _searchQuery = '');
        },
        onConfirmClearLog: () => _confirmClearLog(context),
        onClearAllFilters: _clearAllFilters,
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
        title: Text(AppLocalizations.of(context).operationLogClearLogTitle),
        content: Text(
          AppLocalizations.of(context).operationLogClearConfirm,
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: Text(AppLocalizations.of(context).commonCancel),
          ),
          TextButton(
            onPressed: () {
              OperationLogService.instance.clearEntries();
              Navigator.pop(context);
            },
            child: Text(AppLocalizations.of(context).operationLogClear, style: const TextStyle(color: AppTheme.errorColor)),
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
  final VoidCallback onClearAllFilters;

  const _OperationLogView({
    required this.searchController,
    required this.searchQuery,
    required this.entries,
    required this.onSearchChanged,
    required this.onClearSearch,
    required this.onConfirmClearLog,
    required this.onClearAllFilters,
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
      appBar: SoloGlassAppBar(
        title: Text(AppLocalizations.of(context).operationLogTitle),
        actions: [
          const HeaderActionButtons(),
          if (entries.isNotEmpty)
            IconButton(
              icon: const Icon(Icons.delete_outline),
              onPressed: onConfirmClearLog,
              tooltip: AppLocalizations.of(context).operationLogClearLog,
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
                hintText: AppLocalizations.of(context).operationLogSearchHint,
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
          OperationLogFilterSection(
            resultCount: filteredEntries.length,
            onClearAll: onClearAllFilters,
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
                          AppLocalizations.of(context).operationLogNoMatching,
                          style: theme.textTheme.titleMedium?.copyWith(
                            color: theme.colorScheme.onSurfaceVariant,
                          ),
                        ),
                        const SizedBox(height: 8),
                        Text(
                          searchQuery.isNotEmpty
                              ? AppLocalizations.of(context).operationLogTryDifferent
                              : AppLocalizations.of(context).operationLogAdjustFilters,
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
