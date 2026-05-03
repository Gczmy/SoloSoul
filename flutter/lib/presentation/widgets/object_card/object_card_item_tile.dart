import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/services/field_history_service.dart'
    show fieldHistoriesProvider;
import 'package:solosoul_flutter/presentation/utils/property_value_utils.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';
import 'package:solosoul_flutter/presentation/widgets/object_card/object_card_history_section.dart';
import 'package:solosoul_flutter/presentation/widgets/object_card/object_card_properties_list.dart';
import 'package:solosoul_flutter/presentation/widgets/password_verification_dialog.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart'
    show showOverlaySnackBar, SnackBarType;

class ObjectCardItemTile extends ConsumerWidget {
  final UnifiedObject item;
  final bool isHistoryExpanded;
  final VoidCallback onToggleHistory;
  final VoidCallback onCopy;
  final VoidCallback onStartEdit;
  final VoidCallback onDelete;
  final String historyFieldIdPrefix;
  final String Function(Map<String, String>)? nameExtractor;
  final String titlePropertyKey;

  const ObjectCardItemTile({
    super.key,
    required this.item,
    required this.isHistoryExpanded,
    required this.onToggleHistory,
    required this.onCopy,
    required this.onStartEdit,
    required this.onDelete,
    required this.historyFieldIdPrefix,
    this.nameExtractor,
    this.titlePropertyKey = 'Title',
  });

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);
    final history = ref.watch(
      fieldHistoriesProvider.select(
        (h) => h.getHistory(item.id, historyFieldIdPrefix),
      ),
    );
    final count = history?.entries.length ?? 0;
    final hasHist = count > 0;

    final requiresVerification = item.properties.values.any(
      (p) =>
          p.sensitivity == SensitivityLevel.sensitive ||
          p.sensitivity == SensitivityLevel.critical,
    );

    final iconData =
        isHistoryExpanded ? Icons.history_toggle_off : Icons.history;
    final iconColor = hasHist
        ? theme.colorScheme.onSurfaceVariant
        : theme.colorScheme.onSurfaceVariant.withValues(alpha: 0.4);
    final historyIcon = Icon(iconData, size: 20, color: iconColor);

    Future<void> handleHistoryPress() async {
      if (hasHist) {
        if (requiresVerification) {
          final isGranted = ref.read(isSensitiveAccessGrantedProvider);
          if (!isGranted) {
            final authNotifier = ref.read(authNotifierProvider.notifier);
            final selectedAccount = authNotifier.selectedAccount;
            final password = await showPasswordVerificationDialog(
              context: context,
              ref: ref,
              passwordHint: selectedAccount?.passwordHint,
              onVerify: authNotifier.verifyPasswordForSensitiveData,
            );
            if (password == null) return;
            ref.read(sensitivePageAccessProvider.notifier).markVerified();
          }
        }
        onToggleHistory();
      } else {
        if (context.mounted) {
          showOverlaySnackBar(
            context,
            content: 'No history available',
            type: SnackBarType.info,
          );
        }
      }
    }

    return Padding(
      padding: const EdgeInsets.only(bottom: 12),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    SelectableText(
                      objectItemDisplayTitle(
                        item,
                        nameExtractor: nameExtractor,
                        titlePropertyKey: titlePropertyKey,
                      ),
                      style: theme.textTheme.bodyLarge?.copyWith(
                        fontWeight: FontWeight.w500,
                      ),
                    ),
                    const SizedBox(height: 4),
                    ObjectCardPropertiesList(
                      item: item,
                      titlePropertyKey: titlePropertyKey,
                    ),
                  ],
                ),
              ),
              Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  IconButton(
                    icon: const Icon(Icons.copy_all, size: 20),
                    tooltip: 'Copy',
                    onPressed: onCopy,
                    visualDensity: VisualDensity.compact,
                  ),
                  IconButton(
                    icon: const Icon(Icons.edit_outlined, size: 20),
                    tooltip: 'Edit',
                    onPressed: onStartEdit,
                    visualDensity: VisualDensity.compact,
                  ),
                  IconButton(
                    icon: Stack(
                      clipBehavior: Clip.none,
                      children: [
                        historyIcon,
                        Positioned(
                          right: -6,
                          top: -6,
                          child: Text(
                            '$count',
                            style: TextStyle(
                              fontSize: 10,
                              color: iconColor,
                              fontWeight: FontWeight.w500,
                              height: 1,
                            ),
                          ),
                        ),
                      ],
                    ),
                    tooltip: hasHist ? 'History ($count)' : 'No history yet',
                    onPressed: handleHistoryPress,
                    visualDensity: VisualDensity.compact,
                  ),
                  const SizedBox(width: 8),
                  IconButton(
                    icon: const Icon(Icons.delete_outline, size: 20),
                    tooltip: 'Delete',
                    onPressed: onDelete,
                    visualDensity: VisualDensity.compact,
                  ),
                ],
              ),
            ],
          ),
          if (isHistoryExpanded) ...[
            const SizedBox(height: 8),
            ObjectCardHistorySection(history: history),
          ],
          const Divider(height: 16),
        ],
      ),
    );
  }
}
