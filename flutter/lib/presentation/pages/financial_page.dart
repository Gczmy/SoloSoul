import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_animate/flutter_animate.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart'
    hide SensitivityLevel;
import 'package:solosoul_flutter/presentation/providers/profile_provider.dart';
import 'package:solosoul_flutter/presentation/providers/sensitivity_provider.dart';
import 'package:solosoul_flutter/presentation/widgets/responsive_label_field.dart';
import 'package:solosoul_flutter/core/services/profile_storage_service.dart';
import 'package:solosoul_flutter/core/services/operation_notification.dart';
import 'package:solosoul_flutter/core/services/operation_logger.dart';
import 'package:solosoul_flutter/presentation/pages/operation_log_page.dart';
import 'package:solosoul_flutter/presentation/widgets/unified_form_section.dart'
    show UnifiedFormSection, FormFieldDef;
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
        actions: const [
          HeaderActionButtons(),
        ],
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

class _BankAccountSectionState extends ConsumerState<_BankAccountSection> {
  late List<BankAccountData> _accounts;

  @override
  void initState() {
    super.initState();
    _loadData();
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _loadData();
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

  BankAccountData _createAccountFromValues(Map<String, String> values, {String? id}) {
    return BankAccountData(
      id: id ?? generateEntryId(),
      bankName: values['bankAccount.bankName']?.isEmpty == true
          ? null
          : values['bankAccount.bankName'],
      accountNumber:
          values['bankAccount.accountNumber']?.isEmpty == true
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
        ref.read(sensitivitySettingsProvider).displayMode ==
        SensitivityDisplayMode.hidePrivate;

    final deletedId = account.id;
    print('[FinancialPage] _onAccountDelete: deleting bank account at index=$index, bankName=${account.bankName}');

    final result = await ref
        .read(profileNotifierProvider.notifier)
        .softDelete(
          section: 'financial',
          itemType: 'bank_account',
          index: index,
          deletedItem: account,
        );

    print('[FinancialPage] _onAccountDelete: softDelete completed');

    setState(() {
      _accounts = List.from(_accounts)..removeAt(index);
    });

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
              .restore(section: 'financial', itemType: 'bank_account', id: deletedId);
        },
      );
    }
  }

  Future<void> _onAccountSave(
    Map<String, String> values,
    BankAccountData? editingItem,
  ) async {
    final newAccount = _createAccountFromValues(values, id: editingItem?.id);
    final wasAdding = editingItem == null;
    final itemName = newAccount.bankName ?? 'Bank account';

    if (wasAdding) {
      _accounts = List.from(_accounts)..add(newAccount);
    } else {
      final index = _accounts.indexOf(editingItem);
      if (index != -1) {
        _accounts = List.from(_accounts)..[index] = newAccount;
      }
    }

    final financial = FinancialData(
      bankAccounts: _accounts,
      cards: ref.read(profileNotifierProvider)?.financial?.cards ?? [],
      taxIds: ref.read(profileNotifierProvider)?.financial?.taxIds ?? [],
    );
    await ref.read(profileNotifierProvider.notifier).updateFinancial(financial);

    if (mounted) {
      final isPrivacyMode =
          ref.read(sensitivitySettingsProvider).displayMode ==
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
      fieldDefs: const [
        FormFieldDef(
          fieldId: 'bankAccount.bankName',
          label: 'Bank Name',
          sensitivity: SensitivityLevel.public,
        ),
        FormFieldDef(
          fieldId: 'bankAccount.accountNumber',
          label: 'Account Number',
          sensitivity: SensitivityLevel.restricted,
        ),
        FormFieldDef(
          fieldId: 'bankAccount.currency',
          label: 'Currency',
          sensitivity: SensitivityLevel.public,
        ),
        FormFieldDef(
          fieldId: 'bankAccount.swiftBic',
          label: 'SWIFT/BIC',
          sensitivity: SensitivityLevel.private,
        ),
      ],
      displayItemBuilder: (account) => _BankAccountItem(account: account),
      onDelete: _onAccountDelete,
      onSave: _onAccountSave,
      itemToMap: _accountToMap,
    );
  }
}

// ============ Card Section (using UnifiedFormSection) ============

class _CardSection extends ConsumerStatefulWidget {
  @override
  ConsumerState<_CardSection> createState() => _CardSectionState();
}

class _CardSectionState extends ConsumerState<_CardSection> {
  late List<CardData> _cards;

  @override
  void initState() {
    super.initState();
    _loadData();
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _loadData();
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
        ref.read(sensitivitySettingsProvider).displayMode ==
        SensitivityDisplayMode.hidePrivate;

    final deletedId = card.id;
    await ref
        .read(profileNotifierProvider.notifier)
        .softDelete(
          section: 'financial',
          itemType: 'card',
          index: index,
          deletedItem: card,
        );

    setState(() {
      _cards = List.from(_cards)..removeAt(index);
    });

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
    Map<String, String> values,
    CardData? editingItem,
  ) async {
    final newCard = _createCardFromValues(values, id: editingItem?.id);
    final wasAdding = editingItem == null;
    final itemName = newCard.cardType ?? 'Card';

    if (wasAdding) {
      _cards = List.from(_cards)..add(newCard);
    } else {
      final index = _cards.indexOf(editingItem);
      if (index != -1) {
        _cards = List.from(_cards)..[index] = newCard;
      }
    }

    final financial = FinancialData(
      bankAccounts:
          ref.read(profileNotifierProvider)?.financial?.bankAccounts ?? [],
      cards: _cards,
      taxIds: ref.read(profileNotifierProvider)?.financial?.taxIds ?? [],
    );
    await ref.read(profileNotifierProvider.notifier).updateFinancial(financial);

    if (mounted) {
      final isPrivacyMode =
          ref.read(sensitivitySettingsProvider).displayMode ==
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
      fieldDefs: const [
        FormFieldDef(
          fieldId: 'card.cardType',
          label: 'Card Type (Visa, Mastercard, etc.)',
          sensitivity: SensitivityLevel.public,
        ),
        FormFieldDef(
          fieldId: 'card.cardNumber',
          label: 'Card Number',
          sensitivity: SensitivityLevel.restricted,
        ),
        FormFieldDef(
          fieldId: 'card.expiryDate',
          label: 'Expiry Date',
          sensitivity: SensitivityLevel.private,
        ),
        FormFieldDef(
          fieldId: 'card.holderName',
          label: 'Holder Name',
          sensitivity: SensitivityLevel.private,
        ),
      ],
      displayItemBuilder: (card) => _CardItem(card: card),
      onDelete: _onCardDelete,
      onSave: _onCardSave,
      itemToMap: _cardToMap,
    );
  }
}

// ============ Tax ID Section (using UnifiedFormSection) ============

class _TaxIdSection extends ConsumerStatefulWidget {
  @override
  ConsumerState<_TaxIdSection> createState() =>
      _TaxIdSectionState();
}

class _TaxIdSectionState extends ConsumerState<_TaxIdSection> {
  late List<TaxIdData> _taxIds;

  @override
  void initState() {
    super.initState();
    _loadData();
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _loadData();
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
        ref.read(sensitivitySettingsProvider).displayMode ==
        SensitivityDisplayMode.hidePrivate;

    final deletedId = taxId.id;
    await ref
        .read(profileNotifierProvider.notifier)
        .softDelete(
          section: 'financial',
          itemType: 'tax_id',
          index: index,
          deletedItem: taxId,
        );

    setState(() {
      _taxIds = List.from(_taxIds)..removeAt(index);
    });

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
    Map<String, String> values,
    TaxIdData? editingItem,
  ) async {
    final newTaxId = _createTaxIdFromValues(values, id: editingItem?.id);
    final wasAdding = editingItem == null;
    final itemName = newTaxId.taxIdType ?? 'Tax ID';

    if (wasAdding) {
      _taxIds = List.from(_taxIds)..add(newTaxId);
    } else {
      final index = _taxIds.indexOf(editingItem);
      if (index != -1) {
        _taxIds = List.from(_taxIds)..[index] = newTaxId;
      }
    }

    final financial = FinancialData(
      bankAccounts:
          ref.read(profileNotifierProvider)?.financial?.activeBankAccounts ?? [],
      cards: ref.read(profileNotifierProvider)?.financial?.activeCards ?? [],
      taxIds: _taxIds,
    );
    await ref.read(profileNotifierProvider.notifier).updateFinancial(financial);

    if (mounted) {
      final isPrivacyMode =
          ref.read(sensitivitySettingsProvider).displayMode ==
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
          sensitivity: SensitivityLevel.restricted,
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
      displayItemBuilder: (taxId) => _TaxIdItem(taxId: taxId),
      onDelete: _onTaxIdDelete,
      onSave: _onTaxIdSave,
      itemToMap: _taxIdToMap,
    );
  }
}

// ============ Detailed Widgets ============

/// Detailed bank account display widget showing all fields with sensitivity masking
class _BankAccountItem extends ConsumerWidget {
  final BankAccountData account;

  const _BankAccountItem({required this.account});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);

    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 8),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // Header row
          Row(
            children: [
              Container(
                width: 40,
                height: 40,
                decoration: BoxDecoration(
                  color: AppTheme.primaryColor.withValues(alpha: 0.1),
                  borderRadius: BorderRadius.circular(8),
                ),
                child: Icon(
                  Icons.account_balance,
                  size: 20,
                  color: AppTheme.primaryColor,
                ),
              ),
              const SizedBox(width: 12),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    SelectableText(
                      account.bankName ?? 'Bank Account',
                      style: theme.textTheme.bodyLarge?.copyWith(
                        fontWeight: FontWeight.w500,
                      ),
                    ),
                    if (account.currency != null &&
                        account.currency!.isNotEmpty)
                      SelectableText(
                        account.currency!,
                        style: theme.textTheme.bodySmall?.copyWith(
                          color: theme.colorScheme.onSurfaceVariant,
                        ),
                      ),
                  ],
                ),
              ),
            ],
          ),
          const SizedBox(height: 8),
          // Field rows using ResponsiveLabelField
          ResponsiveLabelField(
            layoutAxis: Axis.vertical,
            fields: [
              if (account.accountNumber != null &&
                  account.accountNumber!.isNotEmpty)
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
            ],
          ),
        ],
      ),
    );
  }
}

/// Detailed card display widget showing all fields with sensitivity masking
class _CardItem extends ConsumerWidget {
  final CardData card;

  const _CardItem({required this.card});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);

    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 8),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // Header row
          Row(
            children: [
              Container(
                width: 40,
                height: 40,
                decoration: BoxDecoration(
                  color: AppTheme.primaryColor.withValues(alpha: 0.1),
                  borderRadius: BorderRadius.circular(8),
                ),
                child: Icon(
                  Icons.credit_card,
                  size: 20,
                  color: AppTheme.primaryColor,
                ),
              ),
              const SizedBox(width: 12),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    SelectableText(
                      card.cardType ?? 'Card',
                      style: theme.textTheme.bodyLarge?.copyWith(
                        fontWeight: FontWeight.w500,
                      ),
                    ),
                    if (card.expiryDate != null &&
                        card.expiryDate!.isNotEmpty)
                      SelectableText(
                        card.expiryDate!,
                        style: theme.textTheme.bodySmall?.copyWith(
                          color: theme.colorScheme.onSurfaceVariant,
                        ),
                      ),
                  ],
                ),
              ),
            ],
          ),
          const SizedBox(height: 8),
          // Field rows using ResponsiveLabelField
          ResponsiveLabelField(
            layoutAxis: Axis.vertical,
            fields: [
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
            ],
          ),
        ],
      ),
    );
  }
}

/// Detailed tax ID display widget showing all fields with sensitivity masking
class _TaxIdItem extends ConsumerWidget {
  final TaxIdData taxId;

  const _TaxIdItem({required this.taxId});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);

    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 8),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // Header row
          Row(
            children: [
              Container(
                width: 40,
                height: 40,
                decoration: BoxDecoration(
                  color: AppTheme.primaryColor.withValues(alpha: 0.1),
                  borderRadius: BorderRadius.circular(8),
                ),
                child: Icon(
                  Icons.badge,
                  size: 20,
                  color: AppTheme.primaryColor,
                ),
              ),
              const SizedBox(width: 12),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    SelectableText(
                      taxId.taxIdType ?? 'Tax ID',
                      style: theme.textTheme.bodyLarge?.copyWith(
                        fontWeight: FontWeight.w500,
                      ),
                    ),
                    if (taxId.country != null &&
                        taxId.country!.isNotEmpty)
                      SelectableText(
                        taxId.country!,
                        style: theme.textTheme.bodySmall?.copyWith(
                          color: theme.colorScheme.onSurfaceVariant,
                        ),
                      ),
                  ],
                ),
              ),
            ],
          ),
          const SizedBox(height: 8),
          // Field rows using ResponsiveLabelField
          ResponsiveLabelField(
            layoutAxis: Axis.vertical,
            fields: [
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
            ],
          ),
        ],
      ),
    );
  }
}
