import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_animate/flutter_animate.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/services/unified_object_service.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart'
    hide SensitivityLevel;
import 'package:solosoul_flutter/presentation/providers/account_style_provider.dart';
import 'package:solosoul_flutter/presentation/widgets/entry_card_widget.dart';
import 'package:solosoul_flutter/presentation/widgets/predefined_object_section.dart';
import 'package:solosoul_flutter/presentation/widgets/object_category_page.dart';
import 'package:solosoul_flutter/presentation/widgets/predefined_object_section_helpers.dart';
import 'package:solosoul_flutter/presentation/widgets/scan_document_button.dart';

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

    return ObjectCategoryPage(
      title: 'Financial',
      sections: [
            const ScanDocumentButton(parentId: DefaultSectionIds.taxId),
            const SizedBox(height: 16),
            PredefinedObjectSection(
              key: const ValueKey(DefaultSectionIds.bankAccount),
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
                    itemData: itemMap,
                    fieldPrefix: 'bankAccount',
                    excludeFields: const {'title'},
                  ),
              onDidDelete: buildOnDidDelete(
                context,
                logSection: LogSection.financial,
                isPrivacyMode: isPrivacyMode,
                ref: ref,
              ),
              onDeleteFailed: buildOnDeleteFailed(
                context,
                sectionLabel: 'bank account',
              ),
              onCopyAll: buildOnCopyAll(context),
            )
                .animate()
                .fadeIn(duration: 400.ms)
                .slideX(begin: 0.05, end: 0),
            const SizedBox(height: 16),
            PredefinedObjectSection(
              key: const ValueKey(DefaultSectionIds.card),
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
                    itemData: itemMap,
                    fieldPrefix: 'card',
                    excludeFields: const {'title'},
                  ),
              onDidDelete: buildOnDidDelete(
                context,
                logSection: LogSection.financial,
                isPrivacyMode: isPrivacyMode,
                ref: ref,
              ),
              onDeleteFailed: buildOnDeleteFailed(
                context,
                sectionLabel: 'card',
              ),
              onCopyAll: buildOnCopyAll(context),
            )
                .animate()
                .fadeIn(delay: 100.ms, duration: 400.ms)
                .slideX(begin: 0.05, end: 0),
            const SizedBox(height: 16),
            PredefinedObjectSection(
              key: const ValueKey(DefaultSectionIds.taxId),
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
                    itemData: itemMap,
                    fieldPrefix: 'taxId',
                    excludeFields: const {'title'},
                  ),
              onDidDelete: buildOnDidDelete(
                context,
                logSection: LogSection.financial,
                isPrivacyMode: isPrivacyMode,
                ref: ref,
              ),
              onDeleteFailed: buildOnDeleteFailed(
                context,
                sectionLabel: 'tax ID',
              ),
              onCopyAll: buildOnCopyAll(context),
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
    );
  }
}
