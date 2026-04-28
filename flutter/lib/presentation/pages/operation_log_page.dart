import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart';
import 'package:solosoul_flutter/presentation/widgets/header_action_buttons.dart';
import 'package:solosoul_flutter/presentation/widgets/password_verification_dialog.dart';
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
  bool _dialogShown = false;

  @override
  void initState() {
    super.initState();
    _refreshLogs();
  }

  Future<void> _refreshLogs() async {
    await OperationLogService.instance.refreshFromDisk();
    if (mounted) setState(() {});
  }

  Future<void> _verifyPassword() async {
    _dialogShown = true;
    // Use the shared password verification dialog with biometric support
    final authNotifier = ref.read(authNotifierProvider.notifier);
    final selectedAccount = authNotifier.selectedAccount;
    final result = await showPasswordVerificationDialog(
      context: context,
      ref: ref,
      message: 'Enter your master password to view the operation log.',
      passwordHint: selectedAccount?.passwordHint,
      onVerify: authNotifier.verifyPasswordForSensitiveData,
    );

    if (!mounted) return;

    if (result != null) {
      // Mark as verified in shared sensitive page access
      ref.read(sensitivePageAccessProvider.notifier).markVerified();
    }
  }

  @override
  Widget build(BuildContext context) {
    if (!ref.watch(isSensitiveAccessGrantedProvider)) {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (!_dialogShown) _verifyPassword();
      });
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
          _buildFilterHeader(),
          AnimatedSwitcher(
            duration: const Duration(milliseconds: 300),
            child: _filterExpanded ? _buildFilterSection() : const SizedBox.shrink(),
          ),
          Expanded(
            child: entries.isEmpty
                ? Center(
                    child: Column(
                      mainAxisAlignment: MainAxisAlignment.center,
                      children: [
                        Icon(
                          Icons.filter_list_off,
                          size: 64,
                          color: theme.colorScheme.onSurfaceVariant,
                        ),
                        const SizedBox(height: 16),
                        Text(
                          'No matching entries',
                          style: theme.textTheme.titleMedium?.copyWith(
                            color: theme.colorScheme.onSurfaceVariant,
                          ),
                        ),
                        const SizedBox(height: 8),
                        Text(
                          'Try adjusting your filters',
                          style: theme.textTheme.bodySmall?.copyWith(
                            color: theme.colorScheme.onSurfaceVariant,
                          ),
                        ),
                      ],
                    ),
                  )
                : ListView.separated(
                    padding: const EdgeInsets.all(16),
                    itemCount: entries.length,
                    separatorBuilder: (_, a) => const SizedBox(height: 8),
                    itemBuilder: (context, index) {
                      final entry = entries[index];
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
    final sensitivityFilters = ref.watch(logSensitivityFilterProvider);
    final hasActiveFilters =
        actionFilters.isNotEmpty || deviceFilters.isNotEmpty || sensitivityFilters.isNotEmpty;

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
                  '${actionFilters.length + deviceFilters.length + sensitivityFilters.length}',
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
    final sensitivityFilters = ref.watch(logSensitivityFilterProvider);

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
            ],
          ),
          const SizedBox(height: 8),
          Row(
            children: [
              Text(
                'Privacy:',
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
                        label: 'Critical',
                        icon: Icons.lock,
                        isSelected: sensitivityFilters.contains(SensitivityLevel.critical),
                        color: Colors.red,
                        onSelected: (_) => _toggleFilter(sensitivityFilters, SensitivityLevel.critical, logSensitivityFilterProvider),
                      ),
                      const SizedBox(width: 4),
                      OperationFilterChip(
                        label: 'Sensitive',
                        icon: Icons.visibility_off,
                        isSelected: sensitivityFilters.contains(SensitivityLevel.sensitive),
                        color: Colors.orange,
                        onSelected: (_) => _toggleFilter(sensitivityFilters, SensitivityLevel.sensitive, logSensitivityFilterProvider),
                      ),
                      const SizedBox(width: 4),
                      OperationFilterChip(
                        label: 'Internal',
                        icon: Icons.folder,
                        isSelected: sensitivityFilters.contains(SensitivityLevel.internal),
                        color: Colors.green,
                        onSelected: (_) => _toggleFilter(sensitivityFilters, SensitivityLevel.internal, logSensitivityFilterProvider),
                      ),
                      const SizedBox(width: 4),
                      OperationFilterChip(
                        label: 'Public',
                        icon: Icons.public,
                        isSelected: sensitivityFilters.contains(SensitivityLevel.public),
                        color: Colors.blue,
                        onSelected: (_) => _toggleFilter(sensitivityFilters, SensitivityLevel.public, logSensitivityFilterProvider),
                      ),
                    ],
                  ),
                ),
              ),
              if (actionFilters.isNotEmpty || deviceFilters.isNotEmpty || sensitivityFilters.isNotEmpty)
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
    ref.read(logSensitivityFilterProvider.notifier).clear();
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
              setState(() {});
            },
            child: const Text('Clear', style: TextStyle(color: AppTheme.errorColor)),
          ),
        ],
      ),
    );
  }
}
