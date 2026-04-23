import 'dart:async' show unawaited;

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_animate/flutter_animate.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart'
    hide SensitivityLevel;
import 'package:solosoul_flutter/presentation/providers/profile_provider.dart';
import 'package:solosoul_flutter/presentation/providers/account_style_provider.dart';
import 'package:solosoul_flutter/presentation/utils/list_utils.dart';
import 'package:solosoul_flutter/core/services/profile_storage_service.dart';
import 'package:solosoul_flutter/core/services/operation_notification.dart';
import 'package:solosoul_flutter/core/services/operation_logger.dart';
import 'package:solosoul_flutter/presentation/pages/operation_log_page.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart'
    show authNotifierProvider;
import 'package:solosoul_flutter/presentation/widgets/unified_form_section.dart'
    show UnifiedFormSection, FormFieldDef, HistoryRecordingConfig;
import 'package:solosoul_flutter/presentation/widgets/entry_card_widget.dart';
import 'package:solosoul_flutter/presentation/widgets/header_action_buttons.dart';
import 'package:solosoul_flutter/core/services/clipboard_monitor_service.dart';
import 'package:solosoul_flutter/presentation/mixins/profile_section_mixin.dart';

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
            _BankAccountSection()
                .animate()
                .fadeIn(duration: 400.ms)
                .slideX(begin: 0.05, end: 0),
            const SizedBox(height: 16),
            _CardSection()
                .animate()
                .fadeIn(delay: 100.ms, duration: 400.ms)
                .slideX(begin: 0.05, end: 0),
            const SizedBox(height: 16),
            _TaxIdSection()
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
                          style: Theme.of(context).textTheme.bodySmall
                              ?.copyWith(
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

// ============ Bank Account Section (using UnifiedFormSection) ============

class _BankAccountSection extends ConsumerStatefulWidget {
  @override
  ConsumerState<_BankAccountSection> createState() =>
      _BankAccountSectionState();
}

class _BankAccountSectionState
    extends ProfileSectionState<_BankAccountSection> {
  Widget _buildBankAccountItem(
    BankAccountData account,
    Map<String, String> itemMap,
  ) {
    return EntryCardWidget<BankAccountData>(
      item: account,
      title: account.title ?? account.bankName ?? 'Bank Account',
      icon: Icons.account_balance,
      itemId: account.id,
      historyFieldId: 'bankAccount',
      isRestricted: true,
      fieldPrefix: 'bankAccount',
      itemData: itemMap.map((k, v) => MapEntry(k, v as dynamic)),
      excludeFields: const {'title'},
      formatAllFields: (e) => '${e.entryType}\n${e.toFormattedString()}',
    );
  }

  @override
  void loadItems() {
    // No-op: items are now sourced from bankAccountItemsProvider
  }

  BankAccountData _createAccountFromValues(
    Map<String, String> values, {
    String? id,
  }) {
    return BankAccountData(
      id: id ?? generateEntryId(),
      title: values['bankAccount.title']?.isEmpty == true
          ? null
          : values['bankAccount.title'],
      bankName: values['bankAccount.bankName']?.isEmpty == true
          ? null
          : values['bankAccount.bankName'],
      accountNumber: values['bankAccount.accountNumber']?.isEmpty == true
          ? null
          : values['bankAccount.accountNumber'],
      currency: values['bankAccount.currency']?.isEmpty == true
          ? null
          : values['bankAccount.currency'],
      swiftBic: values['bankAccount.swiftBic']?.isEmpty == true
          ? null
          : values['bankAccount.swiftBic'],
      sortCode: values['bankAccount.sortCode']?.isEmpty == true
          ? null
          : values['bankAccount.sortCode'],
    );
  }

  Map<String, String> _accountToMap(BankAccountData account) {
    return {
      'title': account.title ?? '',
      'bankName': account.bankName ?? '',
      'accountNumber': account.accountNumber ?? '',
      'currency': account.currency ?? '',
      'swiftBic': account.swiftBic ?? '',
      'sortCode': account.sortCode ?? '',
    };
  }

  /// Thin passthrough - softDelete is the only persistence operation.
  /// Optimistic UI, rollback, and notification are handled by handleDelete via callbacks.
  Future<void> _onAccountDelete(BankAccountData account) async {
    final accounts = ref.read(bankAccountItemsProvider);
    final index = accounts.indexById(account.id, (a) => a.id);
    if (index == -1) return;
    await ref
        .read(profileNotifierProvider.notifier)
        .softDelete(
          section: 'financial',
          itemType: 'bank_account',
          index: index,
          deletedItem: account,
        );
  }

  void _onDidDelete(BankAccountData account, int index) {
    final isPrivacyMode =
        ref.read(displayModeProvider) == SensitivityDisplayMode.hidePrivate;
    final deletedId = account.id;
    OperationNotification.show(
      context,
      message: OperationLogger.createNotification(
        section: LogSection.financial,
        action: LogAction.delete,
        itemName: account.bankName ?? 'Bank account',
        isPrivacyModeActive: isPrivacyMode,
      ),
      duration: const Duration(seconds: 5),
      onUndo: () async {
        await ref
            .read(profileNotifierProvider.notifier)
            .restore(
              section: 'financial',
              itemType: 'bank_account',
              id: deletedId,
            );
      },
    );
  }

  void _onDeleteFailed(BankAccountData account, int index) {
    showOverlaySnackBar(
      context,
      content: 'Failed to delete bank account',
      type: SnackBarType.error,
    );
  }

  Future<void> _onAccountSave(
    BankAccountData? newItem,
    Map<String, String> values,
    BankAccountData? editingItem,
  ) async {
    final wasAdding = editingItem == null;
    final BankAccountData accountToSave;
    if (wasAdding) {
      accountToSave = newItem!;
    } else {
      accountToSave = _createAccountFromValues(values, id: editingItem.id);
    }
    final itemName = accountToSave.bankName ?? 'Bank account';

    // Persist via provider with rollback on failure
    try {
      final currentFinancial = ref.read(profileNotifierProvider)?.financial;
      final accounts = ref.read(bankAccountItemsProvider);
      final updatedAccounts = wasAdding
          ? [...accounts, accountToSave]
          : accounts.map((a) => a.id == editingItem.id ? accountToSave : a).toList();
      final financial = FinancialData(
        bankAccounts: updatedAccounts,
        cards: currentFinancial?.cards ?? [],
        taxIds: currentFinancial?.taxIds ?? [],
      );
      await ref
          .read(profileNotifierProvider.notifier)
          .updateFinancialImmediate(financial);
    } on Exception catch (e) {
      if (mounted) {
        showOverlaySnackBar(context, content: 'Failed to save bank account: $e', type: SnackBarType.error);
      }
      return;
    }

    if (mounted) {
      final isPrivacyMode =
          ref.read(displayModeProvider) == SensitivityDisplayMode.hidePrivate;
      OperationNotification.show(
        context,
        message: OperationLogger.createNotification(
          section: LogSection.financial,
          action: wasAdding ? LogAction.create : LogAction.update,
          itemName: itemName,
          isPrivacyModeActive: isPrivacyMode,
        ),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    final accounts = ref.watch(bankAccountItemsProvider);
    return UnifiedFormSection<BankAccountData>(
      title: 'Bank Accounts',
      icon: Icons.account_balance_outlined,
      items: accounts,
      maxVisibleItems: 3,
      itemFactory: _createAccountFromValues,
      fieldDefs: [
        FormFieldDef(
          fieldId: 'bankAccount.title',
          label: 'Title',
          sensitivity: ref.watch(effectiveSensitivityProvider('bankAccount.title')),
        ),
        FormFieldDef(
          fieldId: 'bankAccount.bankName',
          label: 'Bank Name',
          sensitivity: ref.watch(effectiveSensitivityProvider('bankAccount.bankName')),
        ),
        FormFieldDef(
          fieldId: 'bankAccount.accountNumber',
          label: 'Account Number',
          sensitivity: ref.watch(
            effectiveSensitivityProvider('bankAccount.accountNumber'),
          ),
        ),
        FormFieldDef(
          fieldId: 'bankAccount.currency',
          label: 'Currency',
          sensitivity: ref.watch(effectiveSensitivityProvider('bankAccount.currency')),
        ),
        FormFieldDef(
          fieldId: 'bankAccount.swiftBic',
          label: 'SWIFT/BIC',
          sensitivity: ref.watch(effectiveSensitivityProvider('bankAccount.swiftBic')),
        ),
        FormFieldDef(
          fieldId: 'bankAccount.sortCode',
          label: 'Sort Code',
          sensitivity: ref.watch(effectiveSensitivityProvider('bankAccount.sortCode')),
        ),
      ],
      displayItemBuilder: _buildBankAccountItem,
      onDelete: _onAccountDelete,
      onSave: _onAccountSave,
      itemToMap: _accountToMap,
      onCopyAll: (account, text) async {
        unawaited(Clipboard.setData(ClipboardData(text: text)));
        unawaited(ClipboardMonitorService.instance.notifySensitiveCopied());
        showOverlaySnackBar(
          context,
          content: 'Copied to clipboard',
          type: SnackBarType.success,
        );
      },
      historyConfig: HistoryRecordingConfig<BankAccountData>(
        itemIdExtractor: (item) => item.id,
        fieldIdPrefix: 'bankAccount',
      ),
      historyAwareOnSave: (newItem, values, editingItem, [oldValues]) async {
        if (editingItem == null) return;
        final accountId = ref
            .read(authNotifierProvider.notifier)
            .selectedAccountId;
        if (accountId == null) return;
        await ref
            .read(fieldHistoriesProvider.notifier)
            .recordSnapshot(
              accountId: accountId,
              itemId: editingItem.id,
              fieldIdPrefix: 'bankAccount',
              allFieldValues: oldValues ?? {},
            );
        // Save is handled by onSave callback - historyAwareOnSave only records history
      },
      showHistoryExpansion: true,
      historyFieldIdPrefix: 'bankAccount',
      itemIdExtractor: (item) => item.id,
      onDidDelete: _onDidDelete,
      onDeleteFailed: _onDeleteFailed,
    );
  }
}

// ============ Card Section (using UnifiedFormSection) ============

class _CardSection extends ConsumerStatefulWidget {
  @override
  ConsumerState<_CardSection> createState() => _CardSectionState();
}

class _CardSectionState
    extends ProfileSectionState<_CardSection> {
  Widget _buildCardItem(CardData card, Map<String, String> itemMap) {
    return EntryCardWidget<CardData>(
      item: card,
      title: card.title ?? card.cardType ?? 'Card',
      icon: Icons.credit_card,
      itemId: card.id,
      historyFieldId: 'card',
      isRestricted: true,
      fieldPrefix: 'card',
      itemData: itemMap.map((k, v) => MapEntry(k, v as dynamic)),
      excludeFields: const {'title'},
      formatAllFields: (e) => '${e.entryType}\n${e.toFormattedString()}',
    );
  }

  @override
  void loadItems() {
    // No-op: items are now sourced from cardItemsProvider
  }

  CardData _createCardFromValues(Map<String, String> values, {String? id}) {
    return CardData(
      id: id ?? generateEntryId(),
      title: values['card.title']?.isEmpty == true
          ? null
          : values['card.title'],
      cardType: values['card.cardType']?.isEmpty == true
          ? null
          : values['card.cardType'],
      cardNumber: values['card.cardNumber']?.isEmpty == true
          ? null
          : values['card.cardNumber'],
      expiryDate: values['card.expiryDate']?.isEmpty == true
          ? null
          : values['card.expiryDate'],
      holderName: values['card.holderName']?.isEmpty == true
          ? null
          : values['card.holderName'],
      cvv: values['card.cvv']?.isEmpty == true ? null : values['card.cvv'],
    );
  }

  Map<String, String> _cardToMap(CardData card) {
    return {
      'title': card.title ?? '',
      'cardType': card.cardType ?? '',
      'cardNumber': card.cardNumber ?? '',
      'expiryDate': card.expiryDate ?? '',
      'holderName': card.holderName ?? '',
      'cvv': card.cvv ?? '',
    };
  }

  /// Thin passthrough - softDelete is the only persistence operation.
  /// Optimistic UI, rollback, and notification are handled by handleDelete via callbacks.
  Future<void> _onCardDelete(CardData card) async {
    final cards = ref.read(cardItemsProvider);
    final index = cards.indexById(card.id, (c) => c.id);
    if (index == -1) return;
    await ref
        .read(profileNotifierProvider.notifier)
        .softDelete(
          section: 'financial',
          itemType: 'card',
          index: index,
          deletedItem: card,
        );
  }

  void _onDidDelete(CardData card, int index) {
    final isPrivacyMode =
        ref.read(displayModeProvider) == SensitivityDisplayMode.hidePrivate;
    final deletedId = card.id;
    OperationNotification.show(
      context,
      message: OperationLogger.createNotification(
        section: LogSection.financial,
        action: LogAction.delete,
        itemName: card.cardType ?? 'Card',
        isPrivacyModeActive: isPrivacyMode,
      ),
      duration: const Duration(seconds: 5),
      onUndo: () async {
        await ref
            .read(profileNotifierProvider.notifier)
            .restore(section: 'financial', itemType: 'card', id: deletedId);
        loadItems();
        if (mounted) setState(() {});
      },
    );
  }

  void _onDeleteFailed(CardData card, int index) {
    showOverlaySnackBar(
      context,
      content: 'Failed to delete card',
      type: SnackBarType.error,
    );
  }

  Future<void> _onCardSave(
    CardData? newItem,
    Map<String, String> values,
    CardData? editingItem,
  ) async {
    final wasAdding = editingItem == null;
    final CardData cardToSave;
    if (wasAdding) {
      cardToSave = newItem!;
    } else {
      cardToSave = _createCardFromValues(values, id: editingItem.id);
    }
    final itemName = cardToSave.cardType ?? 'Card';

    // Persist via provider with rollback on failure
    try {
      final currentFinancial = ref.read(profileNotifierProvider)?.financial;
      final cards = ref.read(cardItemsProvider);
      final updatedCards = wasAdding
          ? [...cards, cardToSave]
          : cards.map((c) => c.id == editingItem.id ? cardToSave : c).toList();
      final financial = FinancialData(
        bankAccounts: currentFinancial?.bankAccounts ?? [],
        cards: updatedCards,
        taxIds: currentFinancial?.taxIds ?? [],
      );
      await ref
          .read(profileNotifierProvider.notifier)
          .updateFinancialImmediate(financial);
    } on Exception catch (e) {
      if (mounted) {
        showOverlaySnackBar(context, content: 'Failed to save card: $e', type: SnackBarType.error);
      }
      return;
    }

    if (mounted) {
      final isPrivacyMode =
          ref.read(displayModeProvider) == SensitivityDisplayMode.hidePrivate;
      OperationNotification.show(
        context,
        message: OperationLogger.createNotification(
          section: LogSection.financial,
          action: wasAdding ? LogAction.create : LogAction.update,
          itemName: itemName,
          isPrivacyModeActive: isPrivacyMode,
        ),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    final cards = ref.watch(cardItemsProvider);
    return UnifiedFormSection<CardData>(
      title: 'Cards',
      icon: Icons.credit_card_outlined,
      items: cards,
      maxVisibleItems: 3,
      itemFactory: _createCardFromValues,
      fieldDefs: [
        FormFieldDef(
          fieldId: 'card.title',
          label: 'Title',
          sensitivity: ref.watch(effectiveSensitivityProvider('card.title')),
        ),
        FormFieldDef(
          fieldId: 'card.cardType',
          label: 'Card Type (Visa, Mastercard, etc.)',
          sensitivity: ref.watch(effectiveSensitivityProvider('card.cardType')),
        ),
        FormFieldDef(
          fieldId: 'card.cardNumber',
          label: 'Card Number',
          sensitivity: ref.watch(effectiveSensitivityProvider('card.cardNumber')),
        ),
        FormFieldDef(
          fieldId: 'card.expiryDate',
          label: 'Expiry Date',
          sensitivity: ref.watch(effectiveSensitivityProvider('card.expiryDate')),
        ),
        FormFieldDef(
          fieldId: 'card.holderName',
          label: 'Holder Name',
          sensitivity: ref.watch(effectiveSensitivityProvider('card.holderName')),
        ),
        FormFieldDef(
          fieldId: 'card.cvv',
          label: 'CVV',
          sensitivity: ref.watch(effectiveSensitivityProvider('card.cvv')),
        ),
      ],
      displayItemBuilder: _buildCardItem,
      onDelete: _onCardDelete,
      onSave: _onCardSave,
      itemToMap: _cardToMap,
      onCopyAll: (card, text) async {
        unawaited(Clipboard.setData(ClipboardData(text: text)));
        unawaited(ClipboardMonitorService.instance.notifySensitiveCopied());
        showOverlaySnackBar(
          context,
          content: 'Copied to clipboard',
          type: SnackBarType.success,
        );
      },
      historyConfig: HistoryRecordingConfig<CardData>(
        itemIdExtractor: (item) => item.id,
        fieldIdPrefix: 'card',
      ),
      historyAwareOnSave: (newItem, values, editingItem, [oldValues]) async {
        if (editingItem == null) return;
        final accountId = ref
            .read(authNotifierProvider.notifier)
            .selectedAccountId;
        if (accountId == null) return;
        await ref
            .read(fieldHistoriesProvider.notifier)
            .recordSnapshot(
              accountId: accountId,
              itemId: editingItem.id,
              fieldIdPrefix: 'card',
              allFieldValues: oldValues ?? {},
            );
        // Save is handled by onSave callback - historyAwareOnSave only records history
      },
      showHistoryExpansion: true,
      historyFieldIdPrefix: 'card',
      itemIdExtractor: (item) => item.id,
      onDidDelete: _onDidDelete,
      onDeleteFailed: _onDeleteFailed,
    );
  }
}

// ============ Tax ID Section (using UnifiedFormSection) ============

class _TaxIdSection extends ConsumerStatefulWidget {
  @override
  ConsumerState<_TaxIdSection> createState() => _TaxIdSectionState();
}

class _TaxIdSectionState
    extends ProfileSectionState<_TaxIdSection> {
  Widget _buildTaxIdItem(TaxIdData taxId, Map<String, String> itemMap) {
    return EntryCardWidget<TaxIdData>(
      item: taxId,
      title: taxId.title ?? taxId.taxIdType ?? 'Tax ID',
      icon: Icons.badge,
      itemId: taxId.id,
      historyFieldId: 'taxId',
      isRestricted: true,
      fieldPrefix: 'taxId',
      itemData: itemMap.map((k, v) => MapEntry(k, v as dynamic)),
      excludeFields: const {'title'},
      formatAllFields: (e) => '${e.entryType}\n${e.toFormattedString()}',
    );
  }

  @override
  void loadItems() {
    // No-op: items are now sourced from taxIdItemsProvider
  }

  TaxIdData _createTaxIdFromValues(Map<String, String> values, {String? id}) {
    return TaxIdData(
      id: id ?? generateEntryId(),
      title: values['taxId.title']?.isEmpty == true
          ? null
          : values['taxId.title'],
      taxIdNumber: values['taxId.taxIdNumber']?.isEmpty == true
          ? null
          : values['taxId.taxIdNumber'],
      taxIdType: values['taxId.taxIdType']?.isEmpty == true
          ? null
          : values['taxId.taxIdType'],
      issuingAuthority: values['taxId.issuingAuthority']?.isEmpty == true
          ? null
          : values['taxId.issuingAuthority'],
      country: values['taxId.country']?.isEmpty == true
          ? null
          : values['taxId.country'],
    );
  }

  Map<String, String> _taxIdToMap(TaxIdData taxId) {
    return {
      'title': taxId.title ?? '',
      'taxIdNumber': taxId.taxIdNumber ?? '',
      'taxIdType': taxId.taxIdType ?? '',
      'issuingAuthority': taxId.issuingAuthority ?? '',
      'country': taxId.country ?? '',
    };
  }

  /// Thin passthrough - softDelete is the only persistence operation.
  /// Optimistic UI, rollback, and notification are handled by handleDelete via callbacks.
  Future<void> _onTaxIdDelete(TaxIdData taxId) async {
    final taxIds = ref.read(taxIdItemsProvider);
    final index = taxIds.indexById(taxId.id, (t) => t.id);
    if (index == -1) return;
    await ref
        .read(profileNotifierProvider.notifier)
        .softDelete(
          section: 'financial',
          itemType: 'tax_id',
          index: index,
          deletedItem: taxId,
        );
  }

  void _onTaxIdDidDelete(TaxIdData taxId, int index) {
    final isPrivacyMode =
        ref.read(displayModeProvider) == SensitivityDisplayMode.hidePrivate;
    final deletedId = taxId.id;
    OperationNotification.show(
      context,
      message: OperationLogger.createNotification(
        section: LogSection.financial,
        action: LogAction.delete,
        itemName: taxId.taxIdType ?? 'Tax ID',
        isPrivacyModeActive: isPrivacyMode,
      ),
      duration: const Duration(seconds: 5),
      onUndo: () async {
        await ref
            .read(profileNotifierProvider.notifier)
            .restore(section: 'financial', itemType: 'tax_id', id: deletedId);
      },
    );
  }

  void _onTaxIdDeleteFailed(TaxIdData taxId, int index) {
    showOverlaySnackBar(
      context,
      content: 'Failed to delete tax ID',
      type: SnackBarType.error,
    );
  }

  Future<void> _onTaxIdSave(
    TaxIdData? newItem,
    Map<String, String> values,
    TaxIdData? editingItem,
  ) async {
    final wasAdding = editingItem == null;
    final TaxIdData taxIdToSave;
    if (wasAdding) {
      taxIdToSave = newItem!;
    } else {
      taxIdToSave = _createTaxIdFromValues(values, id: editingItem.id);
    }
    final itemName = taxIdToSave.taxIdType ?? 'Tax ID';

    // Persist via provider with rollback on failure
    try {
      final currentFinancial = ref.read(profileNotifierProvider)?.financial;
      final taxIds = ref.read(taxIdItemsProvider);
      final updatedTaxIds = wasAdding
          ? [...taxIds, taxIdToSave]
          : taxIds.map((t) => t.id == editingItem.id ? taxIdToSave : t).toList();
      final financial = FinancialData(
        bankAccounts: currentFinancial?.bankAccounts ?? [],
        cards: currentFinancial?.cards ?? [],
        taxIds: updatedTaxIds,
      );
      await ref
          .read(profileNotifierProvider.notifier)
          .updateFinancialImmediate(financial);
    } on Exception catch (e) {
      if (mounted) {
        showOverlaySnackBar(context, content: 'Failed to save tax ID: $e', type: SnackBarType.error);
      }
      return;
    }

    if (mounted) {
      final isPrivacyMode =
          ref.read(displayModeProvider) == SensitivityDisplayMode.hidePrivate;
      OperationNotification.show(
        context,
        message: OperationLogger.createNotification(
          section: LogSection.financial,
          action: wasAdding ? LogAction.create : LogAction.update,
          itemName: itemName,
          isPrivacyModeActive: isPrivacyMode,
        ),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    final taxIds = ref.watch(taxIdItemsProvider);
    return UnifiedFormSection<TaxIdData>(
      title: 'Tax Identification',
      icon: Icons.receipt_long_outlined,
      items: taxIds,
      maxVisibleItems: 3,
      itemFactory: _createTaxIdFromValues,
      fieldDefs: [
        FormFieldDef(
          fieldId: 'taxId.title',
          label: 'Title',
          sensitivity: ref.watch(effectiveSensitivityProvider('taxId.title')),
        ),
        FormFieldDef(
          fieldId: 'taxId.taxIdNumber',
          label: 'Tax ID Number',
          sensitivity: ref.watch(effectiveSensitivityProvider('taxId.taxIdNumber')),
        ),
        FormFieldDef(
          fieldId: 'taxId.taxIdType',
          label: 'Tax ID Type (SSN, TIN, VAT, etc.)',
          sensitivity: ref.watch(effectiveSensitivityProvider('taxId.taxIdType')),
        ),
        FormFieldDef(
          fieldId: 'taxId.issuingAuthority',
          label: 'Issuing Authority',
          sensitivity: ref.watch(effectiveSensitivityProvider('taxId.issuingAuthority')),
        ),
        FormFieldDef(
          fieldId: 'taxId.country',
          label: 'Country',
          sensitivity: ref.watch(effectiveSensitivityProvider('taxId.country')),
        ),
      ],
      displayItemBuilder: _buildTaxIdItem,
      onDelete: _onTaxIdDelete,
      onSave: _onTaxIdSave,
      itemToMap: _taxIdToMap,
      onCopyAll: (taxId, text) async {
        unawaited(Clipboard.setData(ClipboardData(text: text)));
        unawaited(ClipboardMonitorService.instance.notifySensitiveCopied());
        showOverlaySnackBar(
          context,
          content: 'Copied to clipboard',
          type: SnackBarType.success,
        );
      },
      historyConfig: HistoryRecordingConfig<TaxIdData>(
        itemIdExtractor: (item) => item.id,
        fieldIdPrefix: 'taxId',
      ),
      historyAwareOnSave: (newItem, values, editingItem, [oldValues]) async {
        if (editingItem == null) return;
        final accountId = ref
            .read(authNotifierProvider.notifier)
            .selectedAccountId;
        if (accountId == null) return;
        await ref
            .read(fieldHistoriesProvider.notifier)
            .recordSnapshot(
              accountId: accountId,
              itemId: editingItem.id,
              fieldIdPrefix: 'taxId',
              allFieldValues: oldValues ?? {},
            );
        // Save is handled by onSave callback - historyAwareOnSave only records history
      },
      showHistoryExpansion: true,
      historyFieldIdPrefix: 'taxId',
      itemIdExtractor: (item) => item.id,
      onDidDelete: _onTaxIdDidDelete,
      onDeleteFailed: _onTaxIdDeleteFailed,
    );
  }
}
