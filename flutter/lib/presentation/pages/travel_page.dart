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
      appBar: AppBar(title: const Text('Travel')),
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

  PassportData _createPassportFromValues(Map<String, String> values, {String? id}) {
    return PassportData(
      id: generateEntryId(),
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

  Map<String, String> _passportToMap(PassportData passport) {
    return {
      'passport.country': passport.country ?? '',
      'passport.number': passport.number ?? '',
      'passport.expiryDate': passport.expiryDate ?? '',
    };
  }

  Future<void> _onPassportDelete(PassportData passport) async {
    final index = _passports.indexOf(passport);
    if (index == -1) return;

    final isPrivacyMode =
        ref.read(sensitivitySettingsProvider).displayMode ==
        SensitivityDisplayMode.hidePrivate;

    final deletedId = passport.id;
    await ref
        .read(profileNotifierProvider.notifier)
        .softDelete(
          section: 'travel',
          itemType: 'passport',
          index: index,
          deletedItem: passport,
        );

    setState(() {
      _passports = List.from(_passports)..removeAt(index);
    });

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
    Map<String, String> values,
    PassportData? editingItem,
  ) async {
    final newPassport = _createPassportFromValues(values);
    final wasAdding = editingItem == null;
    final itemName = newPassport.country ?? 'Passport';

    if (wasAdding) {
      _passports = List.from(_passports)..add(newPassport);
    } else {
      final index = _passports.indexOf(editingItem);
      if (index != -1) {
        _passports = List.from(_passports)..[index] = newPassport;
      }
    }

    final travel = TravelData(
      passports: _passports,
      visas: ref.read(profileNotifierProvider)?.travel?.visas ?? [],
      travelHistory:
          ref.read(profileNotifierProvider)?.travel?.travelHistory ?? [],
    );
    await ref.read(profileNotifierProvider.notifier).updateTravel(travel);

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
      displayItemBuilder: (passport) => _PassportItem(passport: passport),
      onDelete: _onPassportDelete,
      onSave: _onPassportSave,
      itemToMap: _passportToMap,
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
      id: generateEntryId(),
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

  Map<String, String> _visaToMap(VisaData visa) {
    return {
      'visa.country': visa.country ?? '',
      'visa.visaType': visa.visaType ?? '',
      'visa.number': visa.number ?? '',
      'visa.expiryDate': visa.expiryDate ?? '',
    };
  }

  Future<void> _onVisaDelete(VisaData visa) async {
    final index = _visas.indexOf(visa);
    if (index == -1) return;

    final isPrivacyMode =
        ref.read(sensitivitySettingsProvider).displayMode ==
        SensitivityDisplayMode.hidePrivate;

    final deletedId = visa.id;
    await ref
        .read(profileNotifierProvider.notifier)
        .softDelete(
          section: 'travel',
          itemType: 'visa',
          index: index,
          deletedItem: visa,
        );

    setState(() {
      _visas = List.from(_visas)..removeAt(index);
    });

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
    Map<String, String> values,
    VisaData? editingItem,
  ) async {
    final newVisa = _createVisaFromValues(values);
    final wasAdding = editingItem == null;
    final itemName = newVisa.country ?? 'Visa';

    // Update local state
    if (wasAdding) {
      _visas = List.from(_visas)..add(newVisa);
    } else {
      final index = _visas.indexOf(editingItem);
      if (index != -1) {
        _visas = List.from(_visas)..[index] = newVisa;
      }
    }

    // Persist via provider
    final travel = TravelData(
      passports: ref.read(profileNotifierProvider)?.travel?.passports ?? [],
      visas: _visas,
      travelHistory:
          ref.read(profileNotifierProvider)?.travel?.travelHistory ?? [],
    );
    await ref.read(profileNotifierProvider.notifier).updateTravel(travel);

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
      displayItemBuilder: (visa) => _VisaItem(visa: visa),
      onDelete: _onVisaDelete,
      onSave: _onVisaSave,
      itemToMap: _visaToMap,
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
    await ref
        .read(profileNotifierProvider.notifier)
        .softDelete(
          section: 'travel',
          itemType: 'travel_history',
          index: index,
          deletedItem: item,
        );

    setState(() {
      _history = List.from(_history)..removeAt(index);
    });

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
              .restore(section: 'travel', itemType: 'travel_history', id: deletedId);
        },
      );
    }
  }

  Future<void> _onHistorySave(
    Map<String, String> values,
    TravelHistoryData? editingItem,
  ) async {
    final dest = values['travel.destination']?.trim() ?? '';
    if (dest.isEmpty) return;
    final wasAdding = editingItem == null;

    if (wasAdding) {
      _history = List.from(_history)..add(TravelHistoryData(id: generateEntryId(), destination: dest));
    } else {
      final index = _history.indexOf(editingItem);
      if (index != -1) {
        _history = List.from(_history)..[index] = TravelHistoryData(id: editingItem.id, destination: dest);
      }
    }

    final travel = TravelData(
      passports: ref.read(profileNotifierProvider)?.travel?.passports ?? [],
      visas: ref.read(profileNotifierProvider)?.travel?.visas ?? [],
      travelHistory: _history,
    );
    await ref.read(profileNotifierProvider.notifier).updateTravel(travel);

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
      ),
      fieldDefs: const [
        FormFieldDef(
          fieldId: 'travel.destination',
          label: 'Destination',
          sensitivity: SensitivityLevel.public,
        ),
      ],
      displayItemBuilder: (item) => _TravelItem(
        title: item.destination,
        subtitle: '',
        icon: Icons.place,
      ),
      onDelete: _onHistoryDelete,
      onSave: _onHistorySave,
      itemToMap: (item) => {'travel.destination': item.destination},
    );
  }
}

// ============ Shared Widgets ============

/// Detailed passport display widget showing all fields with sensitivity masking
class _PassportItem extends ConsumerWidget {
  final PassportData passport;

  const _PassportItem({required this.passport});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);

    final fields = <LabelValueField>[];
    if (passport.country != null && passport.country!.isNotEmpty) {
      fields.add(LabelValueField(label: 'Country', value: passport.country!));
    }
    if (passport.number != null && passport.number!.isNotEmpty) {
      fields.add(LabelValueField(
        label: 'Passport Number',
        value: passport.number!,
        fieldId: 'passport.number',
        isSensitive: true,
      ));
    }
    if (passport.holderName != null && passport.holderName!.isNotEmpty) {
      fields.add(LabelValueField(
        label: 'Holder Name',
        value: passport.holderName!,
        fieldId: 'passport.holderName',
        isSensitive: true,
      ));
    }
    if (passport.issueDate != null && passport.issueDate!.isNotEmpty) {
      fields.add(LabelValueField(label: 'Issue Date', value: passport.issueDate!));
    }
    if (passport.expiryDate != null && passport.expiryDate!.isNotEmpty) {
      fields.add(LabelValueField(label: 'Expiry Date', value: passport.expiryDate!));
    }

    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 8),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // Header row with icon and country name
          Row(
            children: [
              Container(
                width: 40,
                height: 40,
                decoration: BoxDecoration(
                  color: AppTheme.primaryColor.withValues(alpha: 0.1),
                  borderRadius: BorderRadius.circular(8),
                ),
                child: Icon(Icons.book, size: 20, color: AppTheme.primaryColor),
              ),
              const SizedBox(width: 12),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    SelectableText(
                      passport.country ?? 'Passport',
                      style: theme.textTheme.bodyLarge?.copyWith(
                        fontWeight: FontWeight.w500,
                      ),
                    ),
                    const SizedBox(height: 8),
                    ResponsiveLabelField(
                      fields: fields,
                      labelValueSpacing: 4,
                    ),
                  ],
                ),
              ),
            ],
          ),
        ],
      ),
    );
  }
}

/// Detailed visa display widget showing all fields with sensitivity masking
class _VisaItem extends ConsumerWidget {
  final VisaData visa;

  const _VisaItem({required this.visa});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);

    final fields = <LabelValueField>[];
    if (visa.country != null && visa.country!.isNotEmpty) {
      fields.add(LabelValueField(label: 'Country', value: visa.country!));
    }
    if (visa.visaType != null && visa.visaType!.isNotEmpty) {
      fields.add(LabelValueField(
        label: 'Type',
        value: visa.visaType!,
        fieldId: 'visa.visaType',
        isSensitive: true,
      ));
    }
    if (visa.number != null && visa.number!.isNotEmpty) {
      fields.add(LabelValueField(
        label: 'Visa Number',
        value: visa.number!,
        fieldId: 'visa.number',
        isSensitive: true,
      ));
    }
    if (visa.issueDate != null && visa.issueDate!.isNotEmpty) {
      fields.add(LabelValueField(label: 'Issue Date', value: visa.issueDate!));
    }
    if (visa.expiryDate != null && visa.expiryDate!.isNotEmpty) {
      fields.add(LabelValueField(label: 'Expiry Date', value: visa.expiryDate!));
    }

    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 8),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // Header row with icon and country name
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
                  Icons.article,
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
                      visa.country ?? 'Visa',
                      style: theme.textTheme.bodyLarge?.copyWith(
                        fontWeight: FontWeight.w500,
                      ),
                    ),
                    const SizedBox(height: 8),
                    ResponsiveLabelField(
                      fields: fields,
                      labelValueSpacing: 4,
                    ),
                  ],
                ),
              ),
            ],
          ),
        ],
      ),
    );
  }
}

class _TravelItem extends ConsumerWidget {
  final String title;
  final String subtitle;
  final IconData icon;

  const _TravelItem({
    required this.title,
    required this.subtitle,
    required this.icon,
  });

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final fields = <LabelValueField>[];
    if (title.isNotEmpty) {
      fields.add(LabelValueField(label: 'Destination', value: title));
    }
    if (subtitle.isNotEmpty) {
      fields.add(LabelValueField(label: 'Notes', value: subtitle));
    }

    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 8),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Container(
            width: 40,
            height: 40,
            decoration: BoxDecoration(
              color: AppTheme.primaryColor.withValues(alpha: 0.1),
              borderRadius: BorderRadius.circular(8),
            ),
            child: Icon(icon, size: 20, color: AppTheme.primaryColor),
          ),
          const SizedBox(width: 12),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                ResponsiveLabelField(
                  fields: fields,
                  labelValueSpacing: 4,
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}
