import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/providers/operation_log_provider.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart';
import 'package:solosoul_flutter/presentation/widgets/generic_filter_section.dart';

class OperationLogFilterSection extends ConsumerStatefulWidget {
  const OperationLogFilterSection({
    super.key,
    required this.resultCount,
    this.onClearAll,
  });

  final int resultCount;
  final VoidCallback? onClearAll;

  @override
  ConsumerState<OperationLogFilterSection> createState() => _OperationLogFilterSectionState();
}

class _OperationLogFilterSectionState extends ConsumerState<OperationLogFilterSection> {
  bool _collapsed = false;

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final actionFilters = ref.watch(logActionFilterProvider);
    final deviceFilters = ref.watch(logDeviceFilterProvider);

    final actionOptions = [
      FilterOption(id: 'create', label: l.operationActionCreate,
          icon: Icons.add_circle_outline, color: AppTheme.successColor),
      FilterOption(id: 'update', label: l.operationActionUpdate,
          icon: Icons.edit_outlined, color: AppTheme.primaryColor),
      FilterOption(id: 'delete', label: l.operationActionDelete,
          icon: Icons.delete_outline, color: Colors.orange.shade700),
      FilterOption(id: 'restore', label: l.operationActionRestore,
          icon: Icons.restore, color: Colors.blue),
      FilterOption(id: 'purge', label: l.operationActionPurge,
          icon: Icons.delete_forever, color: AppTheme.errorColor),
    ];

    final deviceOptions = [
      FilterOption(id: 'macos', label: l.operationPlatformMacos,
          icon: Icons.laptop_mac, color: Colors.grey.shade700),
      FilterOption(id: 'ios', label: l.operationPlatformIos,
          icon: Icons.phone_iphone, color: Colors.grey.shade700),
      FilterOption(id: 'android', label: l.operationPlatformAndroid,
          icon: Icons.phone_android, color: Colors.grey.shade700),
      FilterOption(id: 'web', label: l.operationPlatformWeb,
          icon: Icons.web, color: Colors.grey.shade700),
      FilterOption(id: 'windows', label: l.operationPlatformWindows,
          icon: Icons.desktop_windows, color: Colors.grey.shade700),
      FilterOption(id: 'linux', label: l.operationPlatformLinux,
          icon: Icons.computer, color: Colors.grey.shade700),
    ];

    return GenericFilterSection<String>(
      headerLabel: l.operationLogFilters,
      filterGroups: [
        FilterGroup<String>(
          label: '${l.operationLabelAction}:',
          options: actionOptions,
          selectedIds: actionFilters,
          onSelectionChanged: (ids) {
            ref.read(logActionFilterProvider.notifier).setFilters(ids);
          },
        ),
        FilterGroup<String>(
          label: '${l.operationLabelDevice}:',
          options: deviceOptions,
          selectedIds: deviceFilters,
          onSelectionChanged: (ids) {
            ref.read(logDeviceFilterProvider.notifier).setFilters(ids);
          },
        ),
      ],
      resultCount: widget.resultCount,
      expanded: !_collapsed,
      onToggle: () => setState(() => _collapsed = !_collapsed),
      showClearAll: true,
      onClearAll: () {
        ref.read(logActionFilterProvider.notifier).clear();
        ref.read(logDeviceFilterProvider.notifier).clear();
        widget.onClearAll?.call();
      },
    );
  }
}
