import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_animate/flutter_animate.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart'
    hide SensitivityLevel;
import 'package:solosoul_flutter/presentation/theme/app_theme.dart'
    show showOverlaySnackBar;
import 'package:solosoul_flutter/presentation/providers/profile_provider.dart';
import 'package:solosoul_flutter/presentation/providers/sensitivity_provider.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart'
    show authNotifierProvider;
import 'package:solosoul_flutter/core/services/field_history_service.dart'
    show fieldHistoriesProvider;
import 'package:solosoul_flutter/core/services/profile_storage_service.dart';
import 'package:solosoul_flutter/core/services/operation_notification.dart';
import 'package:solosoul_flutter/core/services/operation_logger.dart';
import 'package:solosoul_flutter/presentation/pages/operation_log_page.dart';
import 'package:solosoul_flutter/presentation/widgets/universal_entry_card.dart';
import 'package:solosoul_flutter/presentation/widgets/entry_action_builder.dart';
import 'package:solosoul_flutter/presentation/widgets/unified_form_section.dart'
    show UnifiedFormSection, FormFieldDef, EntryActionsContext,
        HistoryRecordingConfig;
import 'package:solosoul_flutter/presentation/widgets/responsive_label_field.dart'
    show ResponsiveLabelField, LabelValueField;
import 'package:solosoul_flutter/presentation/widgets/field_history_view.dart';
import 'package:solosoul_flutter/presentation/widgets/header_action_buttons.dart';

class TravelPage extends ConsumerStatefulWidget {
  const TravelPage({super.key});

  @override
  ConsumerState<TravelPage> createState() => _TravelPageState();
}

class _TravelPageState extends ConsumerState<TravelPage> {
  @override
  void initState() {
    super.initState();
    // Ensure profile is loaded if accessed directly
    Future.microtask(() {
      final notifier = ref.read(profileNotifierProvider.notifier);
      if (!notifier.isLoading && ref.read(profileNotifierProvider) == null) {
        notifier.loadProfile();
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('Travel'),
        actions: const [HeaderActionButtons()],
      ),
      body: SingleChildScrollView(
        padding: const EdgeInsets.all(24),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // OCR Scan button at top
            SizedBox(
              width: double.infinity,
              child: OutlinedButton.icon(
                onPressed: () => _showOCRUploadDialog(context),
                icon: const Icon(Icons.document_scanner_outlined),
                label: const Text('Scan Document with OCR'),
                style: OutlinedButton.styleFrom(
                  padding: const EdgeInsets.symmetric(vertical: 16),
                ),
              ),
            ).animate().fadeIn(duration: 400.ms),
            const SizedBox(height: 24),
            _PassportSection()
                .animate()
                .fadeIn(delay: 100.ms, duration: 400.ms)
                .slideX(begin: 0.05, end: 0),
            const SizedBox(height: 16),
            _VisaSection()
                .animate()
                .fadeIn(delay: 200.ms, duration: 400.ms)
                .slideX(begin: 0.05, end: 0),
            const SizedBox(height: 16),
            _TravelHistorySection()
                .animate()
                .fadeIn(delay: 300.ms, duration: 400.ms)
                .slideX(begin: 0.05, end: 0),
          ],
        ),
      ),
    );
  }

  void _showOCRUploadDialog(BuildContext context) {
    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('OCR Scan'),
        content: const Text(
          'OCR document scanning will be available after PaddleOCR integration.',
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: const Text('OK'),
          ),
        ],
      ),
    );
  }
}

// ============ Passport Section (using UnifiedFormSection) ============

class _PassportSection extends ConsumerStatefulWidget {
  @override
  ConsumerState<_PassportSection> createState() => _PassportSectionState();
}

class _PassportSectionState extends ConsumerState<_PassportSection> {
  late List<PassportData> _passports;
  bool _historyExpanded = false;

  @override
  void initState() {
    super.initState();
    _loadData();
  }

  void _loadData() {
    final travel = ref.read(profileNotifierProvider)?.travel;
    _passports = [
      ...?(travel?.activePassports.map(
        (p) => PassportData(
          id: p.id,
          country: p.country,
          number: p.number,
          issueDate: p.issueDate,
          expiryDate: p.expiryDate,
          holderName: p.holderName,
        ),
      )),
    ];
  }

  PassportData _createPassportFromValues(
    Map<String, String> values, {
    String? id,
  }) {
    return PassportData(
      id: id ?? generateEntryId(),
      country: values['passport.country']?.isEmpty == true
          ? null
          : values['passport.country'],
      number: values['passport.number']?.isEmpty == true
          ? null
          : values['passport.number'],
      expiryDate: values['passport.expiryDate']?.isEmpty == true
          ? null
          : values['passport.expiryDate'],
    );
  }

  Future<void> _onPassportDelete(PassportData passport) async {
    final index = _passports.indexOf(passport);
    if (index == -1) return;

    final isPrivacyMode =
        ref.read(sensitivitySettingsProvider).displayMode ==
        SensitivityDisplayMode.hidePrivate;

    final deletedId = passport.id;

    setState(() {
      _passports = List.from(_passports)..removeAt(index);
    });
    try {
      await ref
          .read(profileNotifierProvider.notifier)
          .softDelete(
            section: 'travel',
            itemType: 'passport',
            index: index,
            deletedItem: passport,
          );
    } catch (e) {
      setState(() {
        _passports = List.from(_passports)..insert(index, passport);
      });
      if (mounted) {
        showOverlaySnackBar(
          context,
          content: 'Failed to delete passport',
          type: SnackBarType.error,
        );
      }
      return;
    }

    if (mounted) {
      OperationNotification.show(
        context,
        message: OperationLogger.createNotification(
          section: LogSection.travel,
          action: LogAction.delete,
          itemName: passport.country ?? 'Passport',
          isPrivacyModeActive: isPrivacyMode,
        ),
        duration: const Duration(seconds: 5),
        onUndo: () async {
          await ref
              .read(profileNotifierProvider.notifier)
              .restore(section: 'travel', itemType: 'passport', id: deletedId);
        },
      );
    }
  }

  Future<void> _onPassportSave(
    PassportData? newItem,
    Map<String, String> values,
    PassportData? editingItem,
  ) async {
    // For adds: newItem is already created by itemFactory with correct ID
    // For edits: create updated item via factory
    final wasAdding = editingItem == null;
    final PassportData passportToSave;
    if (wasAdding) {
      passportToSave = newItem!;
    } else {
      passportToSave = _createPassportFromValues(values, id: editingItem!.id);
    }
    final itemName = passportToSave.country ?? 'Passport';

    if (wasAdding) {
      _passports = List.from(_passports)..add(passportToSave);
    } else {
      final index = _passports.indexOf(editingItem);
      if (index != -1) {
        _passports = List.from(_passports)..[index] = passportToSave;
      }
    }

    final travel = TravelData(
      passports: _passports,
      visas: ref.read(profileNotifierProvider)?.travel?.visas ?? [],
      travelHistory:
          ref.read(profileNotifierProvider)?.travel?.travelHistory ?? [],
    );
    await ref.read(profileNotifierProvider.notifier).updateTravelImmediate(travel);

    if (mounted) {
      final isPrivacyMode =
          ref.read(sensitivitySettingsProvider).displayMode ==
          SensitivityDisplayMode.hidePrivate;
      OperationNotification.show(
        context,
        message: OperationLogger.createNotification(
          section: LogSection.travel,
          action: wasAdding ? LogAction.create : LogAction.update,
          itemName: itemName,
          isPrivacyModeActive: isPrivacyMode,
        ),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    return UnifiedFormSection<PassportData>(
      title: 'Passports',
      icon: Icons.flight_outlined,
      items: _passports,
      maxVisibleItems: 3,
      itemFactory: _createPassportFromValues,
      fieldDefs: const [
        FormFieldDef(
          fieldId: 'passport.country',
          label: 'Country',
          sensitivity: SensitivityLevel.public,
        ),
        FormFieldDef(
          fieldId: 'passport.number',
          label: 'Passport Number',
          sensitivity: SensitivityLevel.restricted,
        ),
        FormFieldDef(
          fieldId: 'passport.expiryDate',
          label: 'Expiry Date',
          sensitivity: SensitivityLevel.private,
        ),
      ],
      historyConfig: HistoryRecordingConfig<PassportData>(
        itemIdExtractor: (p) => p.id,
        fieldIdPrefix: 'passport',
      ),
      historyAwareOnSave: (newItem, values, editingItem, [oldValues]) async {
        if (editingItem == null) return;
        final accountId = ref.read(authNotifierProvider.notifier).selectedAccountId;
        if (accountId == null) return;
        await ref.read(fieldHistoriesProvider.notifier).recordSnapshot(
              accountId: accountId,
              itemId: editingItem.id,
              fieldIdPrefix: 'passport',
              allFieldValues: values,
            );
      },
      displayItemBuilder: (passport) {
        final fields = <LabelValueField>[];
        if (passport.country != null && passport.country!.isNotEmpty) {
          fields.add(
            LabelValueField(label: 'Country', value: passport.country!),
          );
        }
        if (passport.number != null && passport.number!.isNotEmpty) {
          fields.add(
            LabelValueField(
              label: 'Passport Number',
              value: passport.number!,
              fieldId: 'passport.number',
              isSensitive: true,
            ),
          );
        }
        if (passport.holderName != null && passport.holderName!.isNotEmpty) {
          fields.add(
            LabelValueField(
              label: 'Holder Name',
              value: passport.holderName!,
              fieldId: 'passport.holderName',
              isSensitive: true,
            ),
          );
        }
        if (passport.issueDate != null && passport.issueDate!.isNotEmpty) {
          fields.add(
            LabelValueField(label: 'Issue Date', value: passport.issueDate!),
          );
        }
        if (passport.expiryDate != null && passport.expiryDate!.isNotEmpty) {
          fields.add(
            LabelValueField(label: 'Expiry Date', value: passport.expiryDate!),
          );
        }
        final history = ref
            .watch(fieldHistoriesProvider.notifier)
            .getHistory(passport.id, 'passport');
        final hasHistory = history != null;
        return Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Builder(
              builder: (ctx) {
                final actionsContext = EntryActionsContext.of(ctx);
                final onEdit = actionsContext?.onEdit ?? () {};
                final onDelete = actionsContext?.onDelete ?? () {};
                final onCopy = actionsContext?.onCopy ?? (text) async {};
                return UniversalEntryCard(
                  title: Text(
                    passport.country ?? 'Passport',
                    style: Theme.of(
                      ctx,
                    ).textTheme.bodyLarge?.copyWith(fontWeight: FontWeight.w500),
                  ),
                  leading: const Icon(Icons.book, size: 20),
                  actions: EntryActionBuilder.buildActions(
                    context: ctx,
                    ref: ref,
                    onCopy: () => onCopy('${passport.entryType}\n${passport.toFormattedString()}'),
                    onEdit: onEdit,
                    onDelete: onDelete,
                    config: const EntryActionsConfig(
                      showCopy: true,
                      showEdit: true,
                      showDelete: true,
                    ),
                    isSensitive: fields.any((f) => f.isSensitive),
                  ),
                  bottomActions: [
                    TextButton.icon(
                      icon: Icon(_historyExpanded ? Icons.expand_less : Icons.history, size: 16),
                      label: Text('History(${history?.entries.length ?? 0})'),
                      onPressed: () => setState(() => _historyExpanded = !_historyExpanded),
                    ),
                  ],
                  children: fields.isNotEmpty
                      ? [
                          const SizedBox(height: 4),
                          ResponsiveLabelField(
                            fields: fields,
                            labelValueSpacing: 4,
                            layoutAxis: Axis.vertical,
                          ),
                        ]
                      : [],
                );
              },
            ),
            if (hasHistory && _historyExpanded)
              Padding(
                padding: const EdgeInsets.only(left: 32, bottom: 8),
                child: FieldHistoryView(
                  fieldName: 'passport',
                  history: history,
                ),
              ),
          ],
        );
      },
      onDelete: _onPassportDelete,
      onSave: _onPassportSave,
      itemToMap: (p) => {
        'passport.country': p.country ?? '',
        'passport.number': p.number ?? '',
        'passport.expiryDate': p.expiryDate ?? '',
      },
      onCopyAll: (passport, text) async {
        Clipboard.setData(ClipboardData(text: text));
        showOverlaySnackBar(
          context,
          content: 'Copied to clipboard',
          type: SnackBarType.success,
        );
      },
    );
  }
}

// ============ Visa Section (using UnifiedFormSection) ============

class _VisaSection extends ConsumerStatefulWidget {
  @override
  ConsumerState<_VisaSection> createState() => _VisaSectionState();
}

class _VisaSectionState extends ConsumerState<_VisaSection> {
  late List<VisaData> _visas;
  bool _historyExpanded = false;

  @override
  void initState() {
    super.initState();
    _loadData();
  }

  void _loadData() {
    final travel = ref.read(profileNotifierProvider)?.travel;
    _visas = [
      ...?(travel?.activeVisas.map(
        (v) => VisaData(
          id: v.id,
          country: v.country,
          visaType: v.visaType,
          number: v.number,
          issueDate: v.issueDate,
          expiryDate: v.expiryDate,
        ),
      )),
    ];
  }

  VisaData _createVisaFromValues(Map<String, String> values, {String? id}) {
    return VisaData(
      id: id ?? generateEntryId(),
      country: values['visa.country']?.isEmpty == true
          ? null
          : values['visa.country'],
      visaType: values['visa.visaType']?.isEmpty == true
          ? null
          : values['visa.visaType'],
      number: values['visa.number']?.isEmpty == true
          ? null
          : values['visa.number'],
      expiryDate: values['visa.expiryDate']?.isEmpty == true
          ? null
          : values['visa.expiryDate'],
    );
  }

  Future<void> _onVisaDelete(VisaData visa) async {
    final index = _visas.indexOf(visa);
    if (index == -1) return;

    final isPrivacyMode =
        ref.read(sensitivitySettingsProvider).displayMode ==
        SensitivityDisplayMode.hidePrivate;

    final deletedId = visa.id;

    setState(() {
      _visas = List.from(_visas)..removeAt(index);
    });

    try {
      await ref
          .read(profileNotifierProvider.notifier)
          .softDelete(
            section: 'travel',
            itemType: 'visa',
            index: index,
            deletedItem: visa,
          );
    } catch (e) {
      setState(() {
        _visas = List.from(_visas)..insert(index, visa);
      });
      if (mounted) {
        showOverlaySnackBar(
          context,
          content: 'Failed to delete visa',
          type: SnackBarType.error,
        );
      }
      return;
    }

    if (mounted) {
      OperationNotification.show(
        context,
        message: OperationLogger.createNotification(
          section: LogSection.travel,
          action: LogAction.delete,
          itemName: visa.country ?? 'Visa',
          isPrivacyModeActive: isPrivacyMode,
        ),
        duration: const Duration(seconds: 5),
        onUndo: () async {
          await ref
              .read(profileNotifierProvider.notifier)
              .restore(section: 'travel', itemType: 'visa', id: deletedId);
        },
      );
    }
  }

  Future<void> _onVisaSave(
    VisaData? newItem,
    Map<String, String> values,
    VisaData? editingItem,
  ) async {
    final wasAdding = editingItem == null;
    final VisaData visaToSave;
    if (wasAdding) {
      visaToSave = newItem!;
    } else {
      visaToSave = _createVisaFromValues(values, id: editingItem!.id);
    }
    final itemName = visaToSave.country ?? 'Visa';

    // Update local state
    if (wasAdding) {
      _visas = List.from(_visas)..add(visaToSave);
    } else {
      final index = _visas.indexOf(editingItem);
      if (index != -1) {
        _visas = List.from(_visas)..[index] = visaToSave;
      }
    }

    // Persist via provider
    final travel = TravelData(
      passports: ref.read(profileNotifierProvider)?.travel?.passports ?? [],
      visas: _visas,
      travelHistory:
          ref.read(profileNotifierProvider)?.travel?.travelHistory ?? [],
    );
    await ref.read(profileNotifierProvider.notifier).updateTravelImmediate(travel);

    if (mounted) {
      final isPrivacyMode =
          ref.read(sensitivitySettingsProvider).displayMode ==
          SensitivityDisplayMode.hidePrivate;
      OperationNotification.show(
        context,
        message: OperationLogger.createNotification(
          section: LogSection.travel,
          action: wasAdding ? LogAction.create : LogAction.update,
          itemName: itemName,
          isPrivacyModeActive: isPrivacyMode,
        ),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    return UnifiedFormSection<VisaData>(
      title: 'Visas',
      icon: Icons.article_outlined,
      items: _visas,
      maxVisibleItems: 3,
      itemFactory: _createVisaFromValues,
      fieldDefs: const [
        FormFieldDef(
          fieldId: 'visa.country',
          label: 'Country',
          sensitivity: SensitivityLevel.public,
        ),
        FormFieldDef(
          fieldId: 'visa.visaType',
          label: 'Visa Type',
          sensitivity: SensitivityLevel.private,
        ),
        FormFieldDef(
          fieldId: 'visa.number',
          label: 'Visa Number',
          sensitivity: SensitivityLevel.restricted,
        ),
        FormFieldDef(
          fieldId: 'visa.expiryDate',
          label: 'Expiry Date',
          sensitivity: SensitivityLevel.private,
        ),
      ],
      historyConfig: HistoryRecordingConfig<VisaData>(
        itemIdExtractor: (v) => v.id,
        fieldIdPrefix: 'visa',
      ),
      historyAwareOnSave: (newItem, values, editingItem, [oldValues]) async {
        if (editingItem == null) return;
        final accountId = ref.read(authNotifierProvider.notifier).selectedAccountId;
        if (accountId == null) return;
        await ref.read(fieldHistoriesProvider.notifier).recordSnapshot(
              accountId: accountId,
              itemId: editingItem.id,
              fieldIdPrefix: 'visa',
              allFieldValues: values,
            );
      },
      displayItemBuilder: (visa) {
        final fields = <LabelValueField>[];
        if (visa.country != null && visa.country!.isNotEmpty) {
          fields.add(LabelValueField(label: 'Country', value: visa.country!));
        }
        if (visa.visaType != null && visa.visaType!.isNotEmpty) {
          fields.add(
            LabelValueField(
              label: 'Type',
              value: visa.visaType!,
              fieldId: 'visa.visaType',
              isSensitive: true,
            ),
          );
        }
        if (visa.number != null && visa.number!.isNotEmpty) {
          fields.add(
            LabelValueField(
              label: 'Visa Number',
              value: visa.number!,
              fieldId: 'visa.number',
              isSensitive: true,
            ),
          );
        }
        if (visa.issueDate != null && visa.issueDate!.isNotEmpty) {
          fields.add(
            LabelValueField(label: 'Issue Date', value: visa.issueDate!),
          );
        }
        if (visa.expiryDate != null && visa.expiryDate!.isNotEmpty) {
          fields.add(
            LabelValueField(label: 'Expiry Date', value: visa.expiryDate!),
          );
        }
        final history = ref
            .watch(fieldHistoriesProvider.notifier)
            .getHistory(visa.id, 'visa');
        final hasHistory = history != null;
        return Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Builder(
              builder: (ctx) {
                final actionsContext = EntryActionsContext.of(ctx);
                final onEdit = actionsContext?.onEdit ?? () {};
                final onDelete = actionsContext?.onDelete ?? () {};
                final onCopy = actionsContext?.onCopy ?? (text) async {};
                return UniversalEntryCard(
                  title: Text(
                    visa.country ?? 'Visa',
                    style: Theme.of(
                      ctx,
                    ).textTheme.bodyLarge?.copyWith(fontWeight: FontWeight.w500),
                  ),
                  leading: const Icon(Icons.article, size: 20),
                  actions: EntryActionBuilder.buildActions(
                    context: ctx,
                    ref: ref,
                    onCopy: () => onCopy('${visa.entryType}\n${visa.toFormattedString()}'),
                    onEdit: onEdit,
                    onDelete: onDelete,
                    config: const EntryActionsConfig(
                      showCopy: true,
                      showEdit: true,
                      showDelete: true,
                    ),
                    isSensitive: fields.any((f) => f.isSensitive),
                  ),
                  bottomActions: [
                    TextButton.icon(
                      icon: Icon(_historyExpanded ? Icons.expand_less : Icons.history, size: 16),
                      label: Text('History(${history?.entries.length ?? 0})'),
                      onPressed: () => setState(() => _historyExpanded = !_historyExpanded),
                    ),
                  ],
                  children: fields.isNotEmpty
                      ? [
                          const SizedBox(height: 4),
                          ResponsiveLabelField(
                            fields: fields,
                            labelValueSpacing: 4,
                            layoutAxis: Axis.vertical,
                          ),
                        ]
                      : [],
                );
              },
            ),
            if (hasHistory && _historyExpanded)
              Padding(
                padding: const EdgeInsets.only(left: 32, bottom: 8),
                child: FieldHistoryView(
                  fieldName: 'visa',
                  history: history,
                ),
              ),
          ],
        );
      },
      onDelete: _onVisaDelete,
      onSave: _onVisaSave,
      itemToMap: (v) => {
        'visa.country': v.country ?? '',
        'visa.visaType': v.visaType ?? '',
        'visa.number': v.number ?? '',
        'visa.expiryDate': v.expiryDate ?? '',
      },
      onCopyAll: (visa, text) async {
        Clipboard.setData(ClipboardData(text: text));
        showOverlaySnackBar(
          context,
          content: 'Copied to clipboard',
          type: SnackBarType.success,
        );
      },
    );
  }
}

// ============ Travel History Section (using UnifiedFormSection) ============

class _TravelHistorySection extends ConsumerStatefulWidget {
  @override
  ConsumerState<_TravelHistorySection> createState() =>
      _TravelHistorySectionState();
}

class _TravelHistorySectionState extends ConsumerState<_TravelHistorySection> {
  late List<TravelHistoryData> _history;
  bool _historyExpanded = false;

  @override
  void initState() {
    super.initState();
    _loadData();
  }

  void _loadData() {
    final travel = ref.read(profileNotifierProvider)?.travel;
    _history = [...(travel?.activeTravelHistory ?? [])];
  }

  Future<void> _onHistoryDelete(TravelHistoryData item) async {
    final index = _history.indexOf(item);
    if (index == -1) return;

    final isPrivacyMode =
        ref.read(sensitivitySettingsProvider).displayMode ==
        SensitivityDisplayMode.hidePrivate;

    final deletedId = item.id;

    setState(() {
      _history = List.from(_history)..removeAt(index);
    });

    try {
      await ref
          .read(profileNotifierProvider.notifier)
          .softDelete(
            section: 'travel',
            itemType: 'travel_history',
            index: index,
            deletedItem: item,
          );
    } catch (e) {
      setState(() {
        _history = List.from(_history)..insert(index, item);
      });
      if (mounted) {
        showOverlaySnackBar(
          context,
          content: 'Failed to delete travel history',
          type: SnackBarType.error,
        );
      }
      return;
    }

    if (mounted) {
      OperationNotification.show(
        context,
        message: OperationLogger.createNotification(
          section: LogSection.travel,
          action: LogAction.delete,
          itemName: item.destination,
          isPrivacyModeActive: isPrivacyMode,
        ),
        duration: const Duration(seconds: 5),
        onUndo: () async {
          await ref
              .read(profileNotifierProvider.notifier)
              .restore(
                section: 'travel',
                itemType: 'travel_history',
                id: deletedId,
              );
        },
      );
    }
  }

  Future<void> _onHistorySave(
    TravelHistoryData? newItem,
    Map<String, String> values,
    TravelHistoryData? editingItem,
  ) async {
    final dest = values['travel.destination']?.trim() ?? '';
    if (dest.isEmpty) return;
    final wasAdding = editingItem == null;

    // For adds: use the item already created by itemFactory (with correct ID)
    // For edits: create via inline factory
    final TravelHistoryData itemToSave;
    if (wasAdding) {
      itemToSave = newItem!;
    } else {
      itemToSave = TravelHistoryData(
        id: editingItem!.id,
        destination: dest,
        date: values['travel.date']?.isEmpty == true
            ? null
            : values['travel.date'],
        departureCity: values['travel.departureCity']?.isEmpty == true
            ? null
            : values['travel.departureCity'],
        departureTime: values['travel.departureTime']?.isEmpty == true
            ? null
            : values['travel.departureTime'],
        arrivalTime: values['travel.arrivalTime']?.isEmpty == true
            ? null
            : values['travel.arrivalTime'],
        flightNumber: values['travel.flightNumber']?.isEmpty == true
            ? null
            : values['travel.flightNumber'],
        ticketPrice: values['travel.ticketPrice']?.isEmpty == true
            ? null
            : values['travel.ticketPrice'],
        airline: values['travel.airline']?.isEmpty == true
            ? null
            : values['travel.airline'],
        travelType: values['travel.travelType']?.isEmpty == true
            ? null
            : values['travel.travelType'],
      );
    }

    if (wasAdding) {
      _history = List.from(_history)..add(itemToSave);
    } else {
      final index = _history.indexOf(editingItem);
      if (index != -1) {
        _history = List.from(_history)..[index] = itemToSave;
      }
    }

    final travel = TravelData(
      passports: ref.read(profileNotifierProvider)?.travel?.passports ?? [],
      visas: ref.read(profileNotifierProvider)?.travel?.visas ?? [],
      travelHistory: _history,
    );
    await ref.read(profileNotifierProvider.notifier).updateTravelImmediate(travel);

    if (mounted) {
      final isPrivacyMode =
          ref.read(sensitivitySettingsProvider).displayMode ==
          SensitivityDisplayMode.hidePrivate;
      OperationNotification.show(
        context,
        message: OperationLogger.createNotification(
          section: LogSection.travel,
          action: wasAdding ? LogAction.create : LogAction.update,
          itemName: dest,
          isPrivacyModeActive: isPrivacyMode,
        ),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    return UnifiedFormSection<TravelHistoryData>(
      title: 'Travel History',
      icon: Icons.history,
      items: _history,
      maxVisibleItems: 3,
      itemFactory: (values, {String? id}) => TravelHistoryData(
        id: id ?? generateEntryId(),
        destination: values['travel.destination']?.trim() ?? '',
        date: values['travel.date']?.isEmpty == true
            ? null
            : values['travel.date'],
        departureCity: values['travel.departureCity']?.isEmpty == true
            ? null
            : values['travel.departureCity'],
        departureTime: values['travel.departureTime']?.isEmpty == true
            ? null
            : values['travel.departureTime'],
        arrivalTime: values['travel.arrivalTime']?.isEmpty == true
            ? null
            : values['travel.arrivalTime'],
        flightNumber: values['travel.flightNumber']?.isEmpty == true
            ? null
            : values['travel.flightNumber'],
        ticketPrice: values['travel.ticketPrice']?.isEmpty == true
            ? null
            : values['travel.ticketPrice'],
        airline: values['travel.airline']?.isEmpty == true
            ? null
            : values['travel.airline'],
        travelType: values['travel.travelType']?.isEmpty == true
            ? null
            : values['travel.travelType'],
      ),
      historyConfig: HistoryRecordingConfig<TravelHistoryData>(
        itemIdExtractor: (t) => t.id,
        fieldIdPrefix: 'travel',
      ),
      historyAwareOnSave: (newItem, values, editingItem, [oldValues]) async {
        if (editingItem == null) return;
        final accountId = ref.read(authNotifierProvider.notifier).selectedAccountId;
        if (accountId == null) return;
        await ref.read(fieldHistoriesProvider.notifier).recordSnapshot(
              accountId: accountId,
              itemId: editingItem.id,
              fieldIdPrefix: 'travel',
              allFieldValues: values,
            );
      },
      fieldDefs: const [
        FormFieldDef(
          fieldId: 'travel.destination',
          label: 'Destination',
          sensitivity: SensitivityLevel.public,
        ),
        FormFieldDef(
          fieldId: 'travel.travelType',
          label: 'Travel Type',
          sensitivity: SensitivityLevel.public,
        ),
        FormFieldDef(
          fieldId: 'travel.date',
          label: 'Date',
          sensitivity: SensitivityLevel.public,
        ),
        FormFieldDef(
          fieldId: 'travel.departureCity',
          label: 'Departure City',
          sensitivity: SensitivityLevel.public,
        ),
        FormFieldDef(
          fieldId: 'travel.departureTime',
          label: 'Departure Time',
          sensitivity: SensitivityLevel.public,
        ),
        FormFieldDef(
          fieldId: 'travel.arrivalTime',
          label: 'Arrival Time',
          sensitivity: SensitivityLevel.public,
        ),
        FormFieldDef(
          fieldId: 'travel.flightNumber',
          label: 'Flight/Train/Bus Number',
          sensitivity: SensitivityLevel.public,
        ),
        FormFieldDef(
          fieldId: 'travel.ticketPrice',
          label: 'Ticket Price',
          sensitivity: SensitivityLevel.public,
        ),
        FormFieldDef(
          fieldId: 'travel.airline',
          label: 'Airline/Operator',
          sensitivity: SensitivityLevel.public,
        ),
      ],
      customFormBuilder:
          (context, theme, controllers, mode, onSubmit, onCancel) {
            final travelType = controllers['travel.travelType']?.text ?? '';
            return Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisSize: MainAxisSize.min,
              children: [
                Text(
                  mode == 'adding'
                      ? 'Add Travel History'
                      : 'Edit Travel History',
                  style: theme.textTheme.titleSmall?.copyWith(
                    fontWeight: FontWeight.w600,
                  ),
                ),
                const SizedBox(height: 12),
                // Travel Type Dropdown
                DropdownButtonFormField<String>(
                  value: travelType.isEmpty ? null : travelType,
                  decoration: const InputDecoration(
                    labelText: 'Travel Type',
                    border: OutlineInputBorder(),
                  ),
                  items: const [
                    DropdownMenuItem(
                      value: 'Airplane',
                      child: Text('Airplane'),
                    ),
                    DropdownMenuItem(value: 'Train', child: Text('Train')),
                    DropdownMenuItem(value: 'Bus', child: Text('Bus')),
                    DropdownMenuItem(value: 'Taxi', child: Text('Taxi')),
                    DropdownMenuItem(value: 'Drive', child: Text('Drive')),
                    DropdownMenuItem(value: 'Other', child: Text('Other')),
                  ],
                  onChanged: (value) {
                    controllers['travel.travelType']?.text = value ?? '';
                  },
                ),
                const SizedBox(height: 12),
                // Destination (always shown)
                TextField(
                  controller: controllers['travel.destination'],
                  maxLength: kMaxFieldLength,
                  decoration: const InputDecoration(
                    labelText: 'Destination',
                    border: OutlineInputBorder(),
                    counterText: '',
                  ),
                ),
                const SizedBox(height: 12),
                // Date (always shown)
                TextField(
                  controller: controllers['travel.date'],
                  maxLength: kMaxFieldLength,
                  decoration: const InputDecoration(
                    labelText: 'Date',
                    border: OutlineInputBorder(),
                    counterText: '',
                  ),
                ),
                const SizedBox(height: 12),
                // Departure/Arrival City (always shown)
                Row(
                  children: [
                    Expanded(
                      child: TextField(
                        controller: controllers['travel.departureCity'],
                        maxLength: kMaxFieldLength,
                        decoration: const InputDecoration(
                          labelText: 'Departure City',
                          border: OutlineInputBorder(),
                          counterText: '',
                        ),
                      ),
                    ),
                    const SizedBox(width: 12),
                    Expanded(
                      child: TextField(
                        controller: controllers['travel.arrivalCity'],
                        maxLength: kMaxFieldLength,
                        decoration: const InputDecoration(
                          labelText: 'Arrival City',
                          border: OutlineInputBorder(),
                          counterText: '',
                        ),
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 12),
                // Conditional fields based on travel type
                if (travelType == 'Airplane') ...[
                  Row(
                    children: [
                      Expanded(
                        child: TextField(
                          controller: controllers['travel.departureTime'],
                          maxLength: kMaxFieldLength,
                          decoration: const InputDecoration(
                            labelText: 'Departure Time',
                            border: OutlineInputBorder(),
                            counterText: '',
                          ),
                        ),
                      ),
                      const SizedBox(width: 12),
                      Expanded(
                        child: TextField(
                          controller: controllers['travel.arrivalTime'],
                          maxLength: kMaxFieldLength,
                          decoration: const InputDecoration(
                            labelText: 'Arrival Time',
                            border: OutlineInputBorder(),
                            counterText: '',
                          ),
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: 12),
                  TextField(
                    controller: controllers['travel.airline'],
                    maxLength: kMaxFieldLength,
                    decoration: const InputDecoration(
                      labelText: 'Airline',
                      border: OutlineInputBorder(),
                      counterText: '',
                    ),
                  ),
                  const SizedBox(height: 12),
                  TextField(
                    controller: controllers['travel.flightNumber'],
                    maxLength: kMaxFieldLength,
                    decoration: const InputDecoration(
                      labelText: 'Flight Number',
                      border: OutlineInputBorder(),
                      counterText: '',
                    ),
                  ),
                  const SizedBox(height: 12),
                  TextField(
                    controller: controllers['travel.ticketPrice'],
                    maxLength: kMaxFieldLength,
                    decoration: const InputDecoration(
                      labelText: 'Ticket Price',
                      border: OutlineInputBorder(),
                      counterText: '',
                    ),
                  ),
                ] else if (travelType == 'Train') ...[
                  Row(
                    children: [
                      Expanded(
                        child: TextField(
                          controller: controllers['travel.departureTime'],
                          maxLength: kMaxFieldLength,
                          decoration: const InputDecoration(
                            labelText: 'Departure Time',
                            border: OutlineInputBorder(),
                            counterText: '',
                          ),
                        ),
                      ),
                      const SizedBox(width: 12),
                      Expanded(
                        child: TextField(
                          controller: controllers['travel.arrivalTime'],
                          maxLength: kMaxFieldLength,
                          decoration: const InputDecoration(
                            labelText: 'Arrival Time',
                            border: OutlineInputBorder(),
                            counterText: '',
                          ),
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: 12),
                  TextField(
                    controller: controllers['travel.flightNumber'],
                    maxLength: kMaxFieldLength,
                    decoration: const InputDecoration(
                      labelText: 'Train Number',
                      border: OutlineInputBorder(),
                      counterText: '',
                    ),
                  ),
                  const SizedBox(height: 12),
                  TextField(
                    controller: controllers['travel.ticketPrice'],
                    maxLength: kMaxFieldLength,
                    decoration: const InputDecoration(
                      labelText: 'Ticket Price',
                      border: OutlineInputBorder(),
                      counterText: '',
                    ),
                  ),
                ] else if (travelType == 'Bus') ...[
                  Row(
                    children: [
                      Expanded(
                        child: TextField(
                          controller: controllers['travel.departureTime'],
                          maxLength: kMaxFieldLength,
                          decoration: const InputDecoration(
                            labelText: 'Departure Time',
                            border: OutlineInputBorder(),
                            counterText: '',
                          ),
                        ),
                      ),
                      const SizedBox(width: 12),
                      Expanded(
                        child: TextField(
                          controller: controllers['travel.arrivalTime'],
                          maxLength: kMaxFieldLength,
                          decoration: const InputDecoration(
                            labelText: 'Arrival Time',
                            border: OutlineInputBorder(),
                            counterText: '',
                          ),
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: 12),
                  TextField(
                    controller: controllers['travel.flightNumber'],
                    maxLength: kMaxFieldLength,
                    decoration: const InputDecoration(
                      labelText: 'Bus Number',
                      border: OutlineInputBorder(),
                      counterText: '',
                    ),
                  ),
                  const SizedBox(height: 12),
                  TextField(
                    controller: controllers['travel.ticketPrice'],
                    maxLength: kMaxFieldLength,
                    decoration: const InputDecoration(
                      labelText: 'Ticket Price',
                      border: OutlineInputBorder(),
                      counterText: '',
                    ),
                  ),
                ] else if (travelType == 'Taxi') ...[
                  Row(
                    children: [
                      Expanded(
                        child: TextField(
                          controller: controllers['travel.departureTime'],
                          maxLength: kMaxFieldLength,
                          decoration: const InputDecoration(
                            labelText: 'Departure Time',
                            border: OutlineInputBorder(),
                            counterText: '',
                          ),
                        ),
                      ),
                      const SizedBox(width: 12),
                      Expanded(
                        child: TextField(
                          controller: controllers['travel.arrivalTime'],
                          maxLength: kMaxFieldLength,
                          decoration: const InputDecoration(
                            labelText: 'Arrival Time',
                            border: OutlineInputBorder(),
                            counterText: '',
                          ),
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: 12),
                  TextField(
                    controller: controllers['travel.ticketPrice'],
                    maxLength: kMaxFieldLength,
                    decoration: const InputDecoration(
                      labelText: 'Price',
                      border: OutlineInputBorder(),
                      counterText: '',
                    ),
                  ),
                ] else if (travelType == 'Drive') ...[
                  Row(
                    children: [
                      Expanded(
                        child: TextField(
                          controller: controllers['travel.departureTime'],
                          maxLength: kMaxFieldLength,
                          decoration: const InputDecoration(
                            labelText: 'Departure Time',
                            border: OutlineInputBorder(),
                            counterText: '',
                          ),
                        ),
                      ),
                      const SizedBox(width: 12),
                      Expanded(
                        child: TextField(
                          controller: controllers['travel.arrivalTime'],
                          maxLength: kMaxFieldLength,
                          decoration: const InputDecoration(
                            labelText: 'Arrival Time',
                            border: OutlineInputBorder(),
                            counterText: '',
                          ),
                        ),
                      ),
                    ],
                  ),
                ] else ...[
                  // Other or no type selected
                  Row(
                    children: [
                      Expanded(
                        child: TextField(
                          controller: controllers['travel.departureTime'],
                          maxLength: kMaxFieldLength,
                          decoration: const InputDecoration(
                            labelText: 'Departure Time',
                            border: OutlineInputBorder(),
                            counterText: '',
                          ),
                        ),
                      ),
                      const SizedBox(width: 12),
                      Expanded(
                        child: TextField(
                          controller: controllers['travel.arrivalTime'],
                          maxLength: kMaxFieldLength,
                          decoration: const InputDecoration(
                            labelText: 'Arrival Time',
                            border: OutlineInputBorder(),
                            counterText: '',
                          ),
                        ),
                      ),
                    ],
                  ),
                ],
                const SizedBox(height: 16),
                Row(
                  mainAxisAlignment: MainAxisAlignment.end,
                  children: [
                    TextButton(
                      onPressed: onCancel,
                      child: const Text('Cancel'),
                    ),
                    const SizedBox(width: 8),
                    FilledButton(
                      onPressed: onSubmit,
                      child: Text(mode == 'adding' ? 'Add' : 'Save'),
                    ),
                  ],
                ),
              ],
            );
          },
      displayItemBuilder: (item) {
        final fields = <LabelValueField>[];
        if (item.date != null && item.date!.isNotEmpty) {
          fields.add(LabelValueField(label: 'Date', value: item.date!));
        }
        if (item.departureCity != null && item.departureCity!.isNotEmpty) {
          fields.add(
            LabelValueField(label: 'Departure', value: item.departureCity!),
          );
        }
        if (item.departureTime != null && item.departureTime!.isNotEmpty) {
          fields.add(
            LabelValueField(
              label: 'Departure Time',
              value: item.departureTime!,
            ),
          );
        }
        if (item.arrivalTime != null && item.arrivalTime!.isNotEmpty) {
          fields.add(
            LabelValueField(label: 'Arrival Time', value: item.arrivalTime!),
          );
        }
        if (item.flightNumber != null && item.flightNumber!.isNotEmpty) {
          fields.add(
            LabelValueField(label: 'Flight', value: item.flightNumber!),
          );
        }
        if (item.ticketPrice != null && item.ticketPrice!.isNotEmpty) {
          fields.add(LabelValueField(label: 'Price', value: item.ticketPrice!));
        }
        if (item.airline != null && item.airline!.isNotEmpty) {
          fields.add(LabelValueField(label: 'Airline', value: item.airline!));
        }
        final history = ref
            .watch(fieldHistoriesProvider.notifier)
            .getHistory(item.id, 'travel');
        final hasHistory = history != null;
        return Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Builder(
              builder: (ctx) {
                final actionsContext = EntryActionsContext.of(ctx);
                final onEdit = actionsContext?.onEdit ?? () {};
                final onDelete = actionsContext?.onDelete ?? () {};
                final onCopy = actionsContext?.onCopy ?? (text) async {};
                return UniversalEntryCard(
                  title: Text(
                    item.destination,
                    style: Theme.of(
                      ctx,
                    ).textTheme.bodyLarge?.copyWith(fontWeight: FontWeight.w500),
                  ),
                  subtitle: (item.flightNumber ?? item.date ?? '').isNotEmpty
                      ? Text(
                          item.flightNumber ?? item.date ?? '',
                          style: Theme.of(ctx).textTheme.bodySmall?.copyWith(
                            color: Theme.of(ctx).colorScheme.onSurfaceVariant,
                          ),
                        )
                      : null,
                  leading: const Icon(Icons.place, size: 20),
                  actions: EntryActionBuilder.buildActions(
                    context: ctx,
                    ref: ref,
                    onCopy: () => onCopy('${item.entryType}\n${item.toFormattedString()}'),
                    onEdit: onEdit,
                    onDelete: onDelete,
                    config: const EntryActionsConfig(
                      showCopy: true,
                      showEdit: true,
                      showDelete: true,
                    ),
                    isSensitive: fields.any((f) => f.isSensitive),
                  ),
                  bottomActions: [
                    TextButton.icon(
                      icon: Icon(_historyExpanded ? Icons.expand_less : Icons.history, size: 16),
                      label: Text('History(${history?.entries.length ?? 0})'),
                      onPressed: () => setState(() => _historyExpanded = !_historyExpanded),
                    ),
                  ],
                  children: fields.isNotEmpty
                      ? [
                          const SizedBox(height: 4),
                          ResponsiveLabelField(
                            fields: fields,
                            labelValueSpacing: 4,
                            layoutAxis: Axis.vertical,
                          ),
                        ]
                      : [],
                );
              },
            ),
            if (hasHistory && _historyExpanded)
              Padding(
                padding: const EdgeInsets.only(left: 32, bottom: 8),
                child: FieldHistoryView(
                  fieldName: 'travel',
                  history: history,
                ),
              ),
          ],
        );
      },
      onDelete: _onHistoryDelete,
      onSave: _onHistorySave,
      itemToMap: (item) => {
        'travel.destination': item.destination,
        'travel.date': item.date ?? '',
        'travel.departureCity': item.departureCity ?? '',
        'travel.departureTime': item.departureTime ?? '',
        'travel.arrivalTime': item.arrivalTime ?? '',
        'travel.flightNumber': item.flightNumber ?? '',
        'travel.ticketPrice': item.ticketPrice ?? '',
        'travel.airline': item.airline ?? '',
      },
      onCopyAll: (item, text) async {
        Clipboard.setData(ClipboardData(text: text));
        showOverlaySnackBar(
          context,
          content: 'Copied to clipboard',
          type: SnackBarType.success,
        );
      },
    );
  }

}
