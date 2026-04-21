import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_animate/flutter_animate.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart'
    hide SensitivityLevel;
import 'package:solosoul_flutter/presentation/theme/app_theme.dart'
    show showOverlaySnackBar;
import 'package:solosoul_flutter/presentation/providers/profile_provider.dart';
import 'package:solosoul_flutter/presentation/providers/account_style_provider.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart' show SensitivityLevel;
import 'package:solosoul_flutter/presentation/providers/sensitivity_provider.dart' show SensitivityDisplayMode;
import 'package:solosoul_flutter/presentation/widgets/responsive_label_field.dart';
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

class FinancialPage extends ConsumerStatefulWidget {
  const FinancialPage({super.key});

  @override
  ConsumerState<FinancialPage> createState() => _FinancialPageState();
}

class _FinancialPageState extends ConsumerState<FinancialPage> {
  @override
  void initState() {
    super.initState();
    Future.microtask(() {
      ref.read(profileNotifierProvider.notifier).loadProfile();
    });
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
                  Icon(
                    Icons.lock_outline,
                    color: AppTheme.primaryColor,
                    size: 24,
                  ),
                  const SizedBox(width: 12),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
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

class _BankAccountSectionState extends ConsumerState<_BankAccountSection>
    with WidgetsBindingObserver {
  Widget _buildBankAccountItem(BankAccountData account) {
    final fields = <LabelValueField>[
      if (account.accountNumber != null && account.accountNumber!.isNotEmpty)
        LabelValueField(
          label: 'Account Number',
          value: account.accountNumber!,
          fieldId: 'bankaccount.accountNumber',
          isSensitive: true,
        ),
      if (account.swiftBic != null && account.swiftBic!.isNotEmpty)
        LabelValueField(
          label: 'SWIFT/BIC',
          value: account.swiftBic!,
          fieldId: 'bankaccount.swiftBic',
          isSensitive: true,
        ),
    ];

    return EntryCardWidget<BankAccountData>(
      item: account,
      title: account.bankName ?? 'Bank Account',
      subtitle: account.currency,
      icon: Icons.account_balance,
      fields: fields,
      itemId: account.id,
      historyFieldId: 'bankAccount',
      isRestricted: true,
      formatAllFields: (e) => '${e.entryType}\n${e.toFormattedString()}',
    );
  }

  late List<BankAccountData> _accounts;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
    _loadData();
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    if (state == AppLifecycleState.resumed) {
      _loadData();
    }
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    super.dispose();
  }

  void _loadData() {
    final financial = ref.read(profileNotifierProvider)?.financial;
    _accounts = [
      ...?(financial?.activeBankAccounts.map(
        (b) => BankAccountData(
          id: b.id,
          bankName: b.bankName,
          accountNumber: b.accountNumber,
          currency: b.currency,
          swiftBic: b.swiftBic,
        ),
      )),
    ];
  }

  BankAccountData _createAccountFromValues(
    Map<String, String> values, {
    String? id,
  }) {
    return BankAccountData(
      id: id ?? generateEntryId(),
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
    );
  }

  Map<String, String> _accountToMap(BankAccountData account) {
    return {
      'bankAccount.bankName': account.bankName ?? '',
      'bankAccount.accountNumber': account.accountNumber ?? '',
      'bankAccount.currency': account.currency ?? '',
      'bankAccount.swiftBic': account.swiftBic ?? '',
    };
  }

  Future<void> _onAccountDelete(BankAccountData account) async {
    final index = _accounts.indexOf(account);
    if (index == -1) return;

    final isPrivacyMode =
        ref.read(displayModeProvider) ==
        SensitivityDisplayMode.hidePrivate;

    final deletedId = account.id;

    // Optimistic removal from UI
    setState(() {
      _accounts = List.from(_accounts)..removeAt(index);
    });

    try {
      await ref
          .read(profileNotifierProvider.notifier)
          .softDelete(
            section: 'financial',
            itemType: 'bank_account',
            index: index,
            deletedItem: account,
          );
    } catch (e) {
      // Rollback on failure
      setState(() {
        _accounts = List.from(_accounts)..insert(index, account);
      });
      if (mounted) {
        showOverlaySnackBar(
          context,
          content: 'Failed to delete bank account',
          type: SnackBarType.error,
        );
      }
      return;
    }

    if (mounted) {
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
      accountToSave = _createAccountFromValues(values, id: editingItem!.id);
    }
    final itemName = accountToSave.bankName ?? 'Bank account';

    if (wasAdding) {
      _accounts = List.from(_accounts)..add(accountToSave);
    } else {
      final index = _accounts.indexWhere((a) => a.id == editingItem!.id);
      if (index != -1) {
        _accounts = List.from(_accounts)..[index] = accountToSave;
      }
    }

    final currentFinancial = ref.read(profileNotifierProvider)?.financial;
    final financial = FinancialData(
      bankAccounts: _accounts,
      cards: currentFinancial?.cards ?? [],
      taxIds: currentFinancial?.taxIds ?? [],
    );
    await ref.read(profileNotifierProvider.notifier).updateFinancialImmediate(financial);

    if (mounted) {
      final isPrivacyMode =
          ref.read(displayModeProvider) ==
          SensitivityDisplayMode.hidePrivate;
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
    return UnifiedFormSection<BankAccountData>(
      title: 'Bank Accounts',
      icon: Icons.account_balance_outlined,
      items: _accounts,
      maxVisibleItems: 3,
      itemFactory: _createAccountFromValues,
      fieldDefs: [
        FormFieldDef(
          fieldId: 'bankAccount.bankName',
          label: 'Bank Name',
          sensitivity: ref.watch(fieldLevelProvider('bankAccount.bankName')),
        ),
        FormFieldDef(
          fieldId: 'bankAccount.accountNumber',
          label: 'Account Number',
          sensitivity: ref.watch(fieldLevelProvider('bankAccount.accountNumber')),
        ),
        FormFieldDef(
          fieldId: 'bankAccount.currency',
          label: 'Currency',
          sensitivity: ref.watch(fieldLevelProvider('bankAccount.currency')),
        ),
        FormFieldDef(
          fieldId: 'bankAccount.swiftBic',
          label: 'SWIFT/BIC',
          sensitivity: ref.watch(fieldLevelProvider('bankAccount.swiftBic')),
        ),
      ],
      displayItemBuilder: _buildBankAccountItem,
      onDelete: _onAccountDelete,
      onSave: _onAccountSave,
      itemToMap: _accountToMap,
      onCopyAll: (account, text) async {
        Clipboard.setData(ClipboardData(text: text));
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
        final accountId = ref.read(authNotifierProvider.notifier).selectedAccountId;
        if (accountId == null) return;
        await ref.read(fieldHistoriesProvider.notifier).recordSnapshot(
          accountId: accountId,
          itemId: editingItem.id,
          fieldIdPrefix: 'bankAccount',
          allFieldValues: oldValues ?? {},
        );
        await _onAccountSave(null, values, editingItem);
      },
    );
  }
}

// ============ Card Section (using UnifiedFormSection) ============

class _CardSection extends ConsumerStatefulWidget {
  @override
  ConsumerState<_CardSection> createState() => _CardSectionState();
}

class _CardSectionState extends ConsumerState<_CardSection>
    with WidgetsBindingObserver {
  Widget _buildCardItem(CardData card) {
    final fields = <LabelValueField>[
      if (card.cardNumber != null && card.cardNumber!.isNotEmpty)
        LabelValueField(
          label: 'Card Number',
          value: card.cardNumber!,
          fieldId: 'card.cardNumber',
          isSensitive: true,
        ),
      if (card.holderName != null && card.holderName!.isNotEmpty)
        LabelValueField(
          label: 'Holder Name',
          value: card.holderName!,
          fieldId: 'card.holderName',
          isSensitive: true,
        ),
    ];

    return EntryCardWidget<CardData>(
      item: card,
      title: card.cardType ?? 'Card',
      subtitle: card.expiryDate,
      icon: Icons.credit_card,
      fields: fields,
      itemId: card.id,
      historyFieldId: 'card',
      isRestricted: true,
      formatAllFields: (e) => '${e.entryType}\n${e.toFormattedString()}',
    );
  }

  late List<CardData> _cards;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
    _loadData();
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    if (state == AppLifecycleState.resumed) {
      _loadData();
    }
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    super.dispose();
  }

  void _loadData() {
    final financial = ref.read(profileNotifierProvider)?.financial;
    _cards = [
      ...?(financial?.activeCards.map(
        (c) => CardData(
          id: c.id,
          cardType: c.cardType,
          cardNumber: c.cardNumber,
          expiryDate: c.expiryDate,
          holderName: c.holderName,
        ),
      )),
    ];
  }

  CardData _createCardFromValues(Map<String, String> values, {String? id}) {
    return CardData(
      id: id ?? generateEntryId(),
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
    );
  }

  Map<String, String> _cardToMap(CardData card) {
    return {
      'card.cardType': card.cardType ?? '',
      'card.cardNumber': card.cardNumber ?? '',
      'card.expiryDate': card.expiryDate ?? '',
      'card.holderName': card.holderName ?? '',
    };
  }

  Future<void> _onCardDelete(CardData card) async {
    final index = _cards.indexOf(card);
    if (index == -1) return;

    final isPrivacyMode =
        ref.read(displayModeProvider) ==
        SensitivityDisplayMode.hidePrivate;

    final deletedId = card.id;

    setState(() {
      _cards = List.from(_cards)..removeAt(index);
    });

    try {
      await ref
          .read(profileNotifierProvider.notifier)
          .softDelete(
            section: 'financial',
            itemType: 'card',
            index: index,
            deletedItem: card,
          );
    } catch (e) {
      setState(() {
        _cards = List.from(_cards)..insert(index, card);
      });
      if (mounted) {
        showOverlaySnackBar(
          context,
          content: 'Failed to delete card',
          type: SnackBarType.error,
        );
      }
      return;
    }

    if (mounted) {
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
        },
      );
    }
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
      cardToSave = _createCardFromValues(values, id: editingItem!.id);
    }
    final itemName = cardToSave.cardType ?? 'Card';

    if (wasAdding) {
      _cards = List.from(_cards)..add(cardToSave);
    } else {
      final index = _cards.indexWhere((c) => c.id == editingItem!.id);
      if (index != -1) {
        _cards = List.from(_cards)..[index] = cardToSave;
      }
    }

    final currentFinancial = ref.read(profileNotifierProvider)?.financial;
    final financial = FinancialData(
      bankAccounts: currentFinancial?.bankAccounts ?? [],
      cards: _cards,
      taxIds: currentFinancial?.taxIds ?? [],
    );
    await ref.read(profileNotifierProvider.notifier).updateFinancialImmediate(financial);

    if (mounted) {
      final isPrivacyMode =
          ref.read(displayModeProvider) ==
          SensitivityDisplayMode.hidePrivate;
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
    return UnifiedFormSection<CardData>(
      title: 'Cards',
      icon: Icons.credit_card_outlined,
      items: _cards,
      maxVisibleItems: 3,
      itemFactory: _createCardFromValues,
      fieldDefs: [
        FormFieldDef(
          fieldId: 'card.cardType',
          label: 'Card Type (Visa, Mastercard, etc.)',
          sensitivity: ref.watch(fieldLevelProvider('card.cardType')),
        ),
        FormFieldDef(
          fieldId: 'card.cardNumber',
          label: 'Card Number',
          sensitivity: ref.watch(fieldLevelProvider('card.cardNumber')),
        ),
        FormFieldDef(
          fieldId: 'card.expiryDate',
          label: 'Expiry Date',
          sensitivity: ref.watch(fieldLevelProvider('card.expiryDate')),
        ),
        FormFieldDef(
          fieldId: 'card.holderName',
          label: 'Holder Name',
          sensitivity: ref.watch(fieldLevelProvider('card.holderName')),
        ),
      ],
      displayItemBuilder: _buildCardItem,
      onDelete: _onCardDelete,
      onSave: _onCardSave,
      itemToMap: _cardToMap,
      onCopyAll: (card, text) async {
        Clipboard.setData(ClipboardData(text: text));
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
        final accountId = ref.read(authNotifierProvider.notifier).selectedAccountId;
        if (accountId == null) return;
        await ref.read(fieldHistoriesProvider.notifier).recordSnapshot(
          accountId: accountId,
          itemId: editingItem.id,
          fieldIdPrefix: 'card',
          allFieldValues: oldValues ?? {},
        );
        await _onCardSave(null, values, editingItem);
      },
    );
  }
}

// ============ Tax ID Section (using UnifiedFormSection) ============

class _TaxIdSection extends ConsumerStatefulWidget {
  @override
  ConsumerState<_TaxIdSection> createState() => _TaxIdSectionState();
}

class _TaxIdSectionState extends ConsumerState<_TaxIdSection>
    with WidgetsBindingObserver {
  Widget _buildTaxIdItem(TaxIdData taxId) {
    final fields = <LabelValueField>[
      if (taxId.taxIdNumber != null && taxId.taxIdNumber!.isNotEmpty)
        LabelValueField(
          label: 'Tax ID Number',
          value: taxId.taxIdNumber!,
          fieldId: 'taxId.taxIdNumber',
          isSensitive: true,
        ),
      if (taxId.issuingAuthority != null && taxId.issuingAuthority!.isNotEmpty)
        LabelValueField(
          label: 'Issuing Authority',
          value: taxId.issuingAuthority!,
          fieldId: 'taxId.issuingAuthority',
          isSensitive: false,
        ),
    ];

    return EntryCardWidget<TaxIdData>(
      item: taxId,
      title: taxId.taxIdType ?? 'Tax ID',
      subtitle: taxId.country,
      icon: Icons.badge,
      fields: fields,
      itemId: taxId.id,
      historyFieldId: 'taxId',
      isRestricted: true,
      formatAllFields: (e) => '${e.entryType}\n${e.toFormattedString()}',
    );
  }

  late List<TaxIdData> _taxIds;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
    _loadData();
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    if (state == AppLifecycleState.resumed) {
      _loadData();
    }
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    super.dispose();
  }

  void _loadData() {
    final financial = ref.read(profileNotifierProvider)?.financial;
    _taxIds = [
      ...?(financial?.activeTaxIds.map(
        (t) => TaxIdData(
          id: t.id,
          taxIdNumber: t.taxIdNumber,
          taxIdType: t.taxIdType,
          issuingAuthority: t.issuingAuthority,
          country: t.country,
        ),
      )),
    ];
  }

  TaxIdData _createTaxIdFromValues(Map<String, String> values, {String? id}) {
    return TaxIdData(
      id: id ?? generateEntryId(),
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
      'taxId.taxIdNumber': taxId.taxIdNumber ?? '',
      'taxId.taxIdType': taxId.taxIdType ?? '',
      'taxId.issuingAuthority': taxId.issuingAuthority ?? '',
      'taxId.country': taxId.country ?? '',
    };
  }

  Future<void> _onTaxIdDelete(TaxIdData taxId) async {
    final index = _taxIds.indexOf(taxId);
    if (index == -1) return;

    final isPrivacyMode =
        ref.read(displayModeProvider) ==
        SensitivityDisplayMode.hidePrivate;

    final deletedId = taxId.id;

    setState(() {
      _taxIds = List.from(_taxIds)..removeAt(index);
    });

    try {
      await ref
          .read(profileNotifierProvider.notifier)
          .softDelete(
            section: 'financial',
            itemType: 'tax_id',
            index: index,
            deletedItem: taxId,
          );
    } catch (e) {
      setState(() {
        _taxIds = List.from(_taxIds)..insert(index, taxId);
      });
      if (mounted) {
        showOverlaySnackBar(
          context,
          content: 'Failed to delete tax ID',
          type: SnackBarType.error,
        );
      }
      return;
    }

    if (mounted) {
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
      taxIdToSave = _createTaxIdFromValues(values, id: editingItem!.id);
    }
    final itemName = taxIdToSave.taxIdType ?? 'Tax ID';

    if (wasAdding) {
      _taxIds = List.from(_taxIds)..add(taxIdToSave);
    } else {
      final index = _taxIds.indexWhere((t) => t.id == editingItem!.id);
      if (index != -1) {
        _taxIds = List.from(_taxIds)..[index] = taxIdToSave;
      }
    }

    final currentFinancial = ref.read(profileNotifierProvider)?.financial;
    final financial = FinancialData(
      bankAccounts: currentFinancial?.activeBankAccounts ?? [],
      cards: currentFinancial?.activeCards ?? [],
      taxIds: _taxIds,
    );
    await ref.read(profileNotifierProvider.notifier).updateFinancialImmediate(financial);

    if (mounted) {
      final isPrivacyMode =
          ref.read(displayModeProvider) ==
          SensitivityDisplayMode.hidePrivate;
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
    return UnifiedFormSection<TaxIdData>(
      title: 'Tax Identification',
      icon: Icons.receipt_long_outlined,
      items: _taxIds,
      maxVisibleItems: 3,
      itemFactory: _createTaxIdFromValues,
      fieldDefs: const [
        FormFieldDef(
          fieldId: 'taxId.taxIdNumber',
          label: 'Tax ID Number',
          sensitivity: SensitivityLevel.critical,
        ),
        FormFieldDef(
          fieldId: 'taxId.taxIdType',
          label: 'Tax ID Type (SSN, TIN, VAT, etc.)',
          sensitivity: SensitivityLevel.public,
        ),
        FormFieldDef(
          fieldId: 'taxId.issuingAuthority',
          label: 'Issuing Authority',
          sensitivity: SensitivityLevel.public,
        ),
        FormFieldDef(
          fieldId: 'taxId.country',
          label: 'Country',
          sensitivity: SensitivityLevel.public,
        ),
      ],
      displayItemBuilder: _buildTaxIdItem,
      onDelete: _onTaxIdDelete,
      onSave: _onTaxIdSave,
      itemToMap: _taxIdToMap,
      onCopyAll: (taxId, text) async {
        Clipboard.setData(ClipboardData(text: text));
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
        final accountId = ref.read(authNotifierProvider.notifier).selectedAccountId;
        if (accountId == null) return;
        await ref.read(fieldHistoriesProvider.notifier).recordSnapshot(
          accountId: accountId,
          itemId: editingItem.id,
          fieldIdPrefix: 'taxId',
          allFieldValues: oldValues ?? {},
        );
        await _onTaxIdSave(null, values, editingItem);
      },
    );
  }
}
