import 'dart:async' show unawaited;

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_animate/flutter_animate.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/services/unified_object_service.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart'
    hide SensitivityLevel;
import 'package:solosoul_flutter/presentation/providers/account_style_provider.dart';
import 'package:solosoul_flutter/core/services/operation_notification.dart';
import 'package:solosoul_flutter/core/services/operation_logger.dart';
import 'package:solosoul_flutter/presentation/models/operation_log_models.dart';
import 'package:solosoul_flutter/presentation/widgets/entry_card_widget.dart';
import 'package:solosoul_flutter/presentation/widgets/predefined_object_section.dart';
import 'package:solosoul_flutter/presentation/widgets/header_action_buttons.dart';
import 'package:solosoul_flutter/presentation/providers/unified_object_provider.dart'
    show unifiedObjectProvider;
import 'package:solosoul_flutter/core/services/clipboard_monitor_service.dart';

class FinancialPage extends ConsumerStatefulWidget {
  const FinancialPage({super.key});

  @override
  ConsumerState<FinancialPage> createState() => _FinancialPageState();
}

class _FinancialPageState extends ConsumerState<FinancialPage> {
  @override
  void initState() {
    super.initState();
  }

  @override
  Widget build(BuildContext context) {
    final isPrivacyMode =
        ref.read(accountStyleProvider).value?.displayMode ==
        SensitivityDisplayMode.hidePrivate;

    return Scaffold(
      appBar: AppBar(
        title: const Text('Financial'),
        actions: const [HeaderActionButtons()],
      ),
      body: SingleChildScrollView(
        padding: const EdgeInsets.all(24),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            PredefinedObjectSection(
              sectionId: DefaultSectionIds.bankAccount,
              typeId: 'financial_bank_account',
              title: 'Bank Accounts',
              icon: Icons.account_balance_outlined,
              maxVisibleItems: 3,
              displayItemBuilder:
                  (account, itemMap) => EntryCardWidget<UnifiedObject>(
                    item: account,
                    title: account.name,
                    icon: Icons.account_balance,
                    itemId: account.id,
                    historyFieldId: 'bankAccount',
                    isRestricted: true,
                    formatAllFields:
                        (a) => 'Bank Account\n${a.toFormattedString()}',
                    itemData: itemMap.map((k, v) => MapEntry(k, v as dynamic)),
                    fieldPrefix: 'bankAccount',
                    excludeFields: const {'title'},
                  ),
              onDidDelete: (item, index) {
                OperationNotification.show(
                  context,
                  message: OperationLogger.createNotification(
                    section: LogSection.financial,
                    action: LogAction.delete,
                    itemName: item.name,
                    isPrivacyModeActive: isPrivacyMode,
                  ),
                  duration: const Duration(seconds: 5),
                  onUndo: () async {
                    await ref
                        .read(unifiedObjectProvider.notifier)
                        .restoreDefaultItem(item.id);
                  },
                );
              },
              onDeleteFailed: (item, index) {
                showOverlaySnackBar(
                  context,
                  content: 'Failed to delete bank account',
                  type: SnackBarType.error,
                );
              },
              onCopyAll: (item, text) async {
                unawaited(Clipboard.setData(ClipboardData(text: text)));
                unawaited(
                  ClipboardMonitorService.instance.notifySensitiveCopied(),
                );
                showOverlaySnackBar(
                  context,
                  content: 'Copied to clipboard',
                  type: SnackBarType.success,
                );
              },
            )
                .animate()
                .fadeIn(duration: 400.ms)
                .slideX(begin: 0.05, end: 0),
            const SizedBox(height: 16),
            PredefinedObjectSection(
              sectionId: DefaultSectionIds.card,
              typeId: 'financial_card',
              title: 'Cards',
              icon: Icons.credit_card_outlined,
              maxVisibleItems: 3,
              displayItemBuilder:
                  (card, itemMap) => EntryCardWidget<UnifiedObject>(
                    item: card,
                    title: card.name,
                    icon: Icons.credit_card,
                    itemId: card.id,
                    historyFieldId: 'card',
                    isRestricted: true,
                    formatAllFields: (c) => 'Card\n${c.toFormattedString()}',
                    itemData: itemMap.map((k, v) => MapEntry(k, v as dynamic)),
                    fieldPrefix: 'card',
                    excludeFields: const {'title'},
                  ),
              onDidDelete: (item, index) {
                OperationNotification.show(
                  context,
                  message: OperationLogger.createNotification(
                    section: LogSection.financial,
                    action: LogAction.delete,
                    itemName: item.name,
                    isPrivacyModeActive: isPrivacyMode,
                  ),
                  duration: const Duration(seconds: 5),
                  onUndo: () async {
                    await ref
                        .read(unifiedObjectProvider.notifier)
                        .restoreDefaultItem(item.id);
                  },
                );
              },
              onDeleteFailed: (item, index) {
                showOverlaySnackBar(
                  context,
                  content: 'Failed to delete card',
                  type: SnackBarType.error,
                );
              },
              onCopyAll: (item, text) async {
                unawaited(Clipboard.setData(ClipboardData(text: text)));
                unawaited(
                  ClipboardMonitorService.instance.notifySensitiveCopied(),
                );
                showOverlaySnackBar(
                  context,
                  content: 'Copied to clipboard',
                  type: SnackBarType.success,
                );
              },
            )
                .animate()
                .fadeIn(delay: 100.ms, duration: 400.ms)
                .slideX(begin: 0.05, end: 0),
            const SizedBox(height: 16),
            PredefinedObjectSection(
              sectionId: DefaultSectionIds.taxId,
              typeId: 'financial_tax_id',
              title: 'Tax Identification',
              icon: Icons.receipt_long_outlined,
              maxVisibleItems: 3,
              displayItemBuilder:
                  (taxId, itemMap) => EntryCardWidget<UnifiedObject>(
                    item: taxId,
                    title: taxId.name,
                    icon: Icons.badge,
                    itemId: taxId.id,
                    historyFieldId: 'taxId',
                    isRestricted: true,
                    formatAllFields: (t) => 'Tax ID\n${t.toFormattedString()}',
                    itemData: itemMap.map((k, v) => MapEntry(k, v as dynamic)),
                    fieldPrefix: 'taxId',
                    excludeFields: const {'title'},
                  ),
              onDidDelete: (item, index) {
                OperationNotification.show(
                  context,
                  message: OperationLogger.createNotification(
                    section: LogSection.financial,
                    action: LogAction.delete,
                    itemName: item.name,
                    isPrivacyModeActive: isPrivacyMode,
                  ),
                  duration: const Duration(seconds: 5),
                  onUndo: () async {
                    await ref
                        .read(unifiedObjectProvider.notifier)
                        .restoreDefaultItem(item.id);
                  },
                );
              },
              onDeleteFailed: (item, index) {
                showOverlaySnackBar(
                  context,
                  content: 'Failed to delete tax ID',
                  type: SnackBarType.error,
                );
              },
              onCopyAll: (item, text) async {
                unawaited(Clipboard.setData(ClipboardData(text: text)));
                unawaited(
                  ClipboardMonitorService.instance.notifySensitiveCopied(),
                );
                showOverlaySnackBar(
                  context,
                  content: 'Copied to clipboard',
                  type: SnackBarType.success,
                );
              },
            )
                .animate()
                .fadeIn(delay: 200.ms, duration: 400.ms)
                .slideX(begin: 0.05, end: 0),
            const SizedBox(height: 32),
            Container(
              padding: const EdgeInsets.all(16),
              decoration: BoxDecoration(
                color: AppTheme.primaryColor.withValues(alpha: 0.05),
                borderRadius: BorderRadius.circular(12),
                border: Border.all(
                  color: AppTheme.primaryColor.withValues(alpha: 0.2),
                ),
              ),
              child: Row(
                children: [
                  const Icon(
                    Icons.lock_outline,
                    color: AppTheme.primaryColor,
                    size: 24,
                  ),
                  const SizedBox(width: 12),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        const Text(
                          'End-to-End Encrypted',
                          style: TextStyle(
                            fontWeight: FontWeight.w600,
                            color: AppTheme.primaryColor,
                          ),
                        ),
                        const SizedBox(height: 2),
                        Text(
                          'Your financial data is encrypted with AES-256-GCM',
                          style: Theme.of(context).textTheme.bodySmall?.copyWith(
                            color: Theme.of(
                              context,
                            ).colorScheme.onSurfaceVariant,
                          ),
                        ),
                      ],
                    ),
                  ),
                ],
              ),
            ).animate().fadeIn(delay: 300.ms, duration: 400.ms),
          ],
        ),
      ),
    );
  }
}
