import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/presentation/providers/operation_log_provider.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart';
import 'package:solosoul_flutter/presentation/widgets/operation_filter_chip.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';

class OperationLogFilterSection extends ConsumerWidget {
  final VoidCallback onClearAll;

  const OperationLogFilterSection({
    super.key,
    required this.onClearAll,
  });

  void _toggleFilter<T>(
    WidgetRef ref,
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

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = AppLocalizations.of(context);
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
                '${l10n.operationLabelAction}:',
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
                        label: l10n.operationActionCreate,
                        icon: Icons.add_circle_outline,
                        isSelected: actionFilters.contains('create'),
                        color: AppTheme.successColor,
                        onSelected: (_) => _toggleFilter(ref, actionFilters, 'create', logActionFilterProvider),
                      ),
                      const SizedBox(width: 4),
                      OperationFilterChip(
                        label: l10n.operationActionUpdate,
                        icon: Icons.edit_outlined,
                        isSelected: actionFilters.contains('update'),
                        color: AppTheme.primaryColor,
                        onSelected: (_) => _toggleFilter(ref, actionFilters, 'update', logActionFilterProvider),
                      ),
                      const SizedBox(width: 4),
                      OperationFilterChip(
                        label: l10n.operationActionDelete,
                        icon: Icons.delete_outline,
                        isSelected: actionFilters.contains('delete'),
                        color: Colors.orange.shade700,
                        onSelected: (_) => _toggleFilter(ref, actionFilters, 'delete', logActionFilterProvider),
                      ),
                      const SizedBox(width: 4),
                      OperationFilterChip(
                        label: l10n.operationActionRestore,
                        icon: Icons.restore,
                        isSelected: actionFilters.contains('restore'),
                        color: Colors.blue,
                        onSelected: (_) => _toggleFilter(ref, actionFilters, 'restore', logActionFilterProvider),
                      ),
                      const SizedBox(width: 4),
                      OperationFilterChip(
                        label: l10n.operationActionPurge,
                        icon: Icons.delete_forever,
                        isSelected: actionFilters.contains('purge'),
                        color: AppTheme.errorColor,
                        onSelected: (_) => _toggleFilter(ref, actionFilters, 'purge', logActionFilterProvider),
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
                '${l10n.operationLabelDevice}:',
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
                        label: l10n.operationPlatformMacos,
                        icon: Icons.laptop_mac,
                        isSelected: deviceFilters.contains('macos'),
                        color: Colors.grey.shade700,
                        onSelected: (_) => _toggleFilter(ref, deviceFilters, 'macos', logDeviceFilterProvider),
                      ),
                      const SizedBox(width: 4),
                      OperationFilterChip(
                        label: l10n.operationPlatformIos,
                        icon: Icons.phone_iphone,
                        isSelected: deviceFilters.contains('ios'),
                        color: Colors.grey.shade700,
                        onSelected: (_) => _toggleFilter(ref, deviceFilters, 'ios', logDeviceFilterProvider),
                      ),
                      const SizedBox(width: 4),
                      OperationFilterChip(
                        label: l10n.operationPlatformAndroid,
                        icon: Icons.phone_android,
                        isSelected: deviceFilters.contains('android'),
                        color: Colors.grey.shade700,
                        onSelected: (_) => _toggleFilter(ref, deviceFilters, 'android', logDeviceFilterProvider),
                      ),
                      const SizedBox(width: 4),
                      OperationFilterChip(
                        label: l10n.operationPlatformWeb,
                        icon: Icons.web,
                        isSelected: deviceFilters.contains('web'),
                        color: Colors.grey.shade700,
                        onSelected: (_) => _toggleFilter(ref, deviceFilters, 'web', logDeviceFilterProvider),
                      ),
                    ],
                  ),
                ),
              ),
              if (actionFilters.isNotEmpty || deviceFilters.isNotEmpty)
                TextButton.icon(
                  onPressed: onClearAll,
                  icon: const Icon(Icons.clear_all, size: 16),
                  label: Text(l10n.operationLogClear),
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
}
