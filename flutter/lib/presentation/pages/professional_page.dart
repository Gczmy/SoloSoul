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
import 'package:solosoul_flutter/core/services/profile_storage_service.dart';
import 'package:solosoul_flutter/presentation/widgets/unified_form_section.dart'
    show UnifiedFormSection, FormFieldDef, EntryActionsContext, HistoryRecordingConfig;
import 'package:solosoul_flutter/presentation/widgets/header_action_buttons.dart';
import 'package:solosoul_flutter/presentation/widgets/responsive_label_field.dart'
    show ResponsiveLabelField, LabelValueField;
import 'package:solosoul_flutter/presentation/widgets/universal_entry_card.dart';
import 'package:solosoul_flutter/presentation/widgets/entry_action_builder.dart';
import 'package:solosoul_flutter/presentation/widgets/field_history_view.dart';
import 'package:solosoul_flutter/core/services/field_history_service.dart';
import 'package:solosoul_flutter/core/services/operation_notification.dart';
import 'package:solosoul_flutter/core/services/operation_logger.dart';
import 'package:solosoul_flutter/presentation/pages/operation_log_page.dart'
    show LogSection, LogAction;
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart'
    show authNotifierProvider;

class ProfessionalPage extends ConsumerStatefulWidget {
  const ProfessionalPage({super.key});

  @override
  ConsumerState<ProfessionalPage> createState() => _ProfessionalPageState();
}

class _ProfessionalPageState extends ConsumerState<ProfessionalPage> {
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
        title: const Text('Professional'),
        actions: const [HeaderActionButtons()],
      ),
      body: SingleChildScrollView(
        padding: const EdgeInsets.all(24),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // Upload CV button at top
            SizedBox(
              width: double.infinity,
              child: OutlinedButton.icon(
                onPressed: () => _showCVUploadDialog(context),
                icon: const Icon(Icons.upload_file_outlined),
                label: const Text('Upload CV / Resume'),
                style: OutlinedButton.styleFrom(
                  padding: const EdgeInsets.symmetric(vertical: 16),
                ),
              ),
            ).animate().fadeIn(duration: 400.ms),
            const SizedBox(height: 24),
            _EducationSection()
                .animate()
                .fadeIn(delay: 100.ms, duration: 400.ms)
                .slideX(begin: 0.05, end: 0),
            const SizedBox(height: 16),
            _EmploymentSection()
                .animate()
                .fadeIn(delay: 200.ms, duration: 400.ms)
                .slideX(begin: 0.05, end: 0),
            const SizedBox(height: 16),
            _AwardSection()
                .animate()
                .fadeIn(delay: 300.ms, duration: 400.ms)
                .slideX(begin: 0.05, end: 0),
            const SizedBox(height: 16),
            _SkillsSection()
                .animate()
                .fadeIn(delay: 400.ms, duration: 400.ms)
                .slideX(begin: 0.05, end: 0),
            const SizedBox(height: 16),
            _LanguageSection()
                .animate()
                .fadeIn(delay: 500.ms, duration: 400.ms)
                .slideX(begin: 0.05, end: 0),
          ],
        ),
      ),
    );
  }

  void _showCVUploadDialog(BuildContext context) {
    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Upload CV'),
        content: const Text(
          'CV upload and parsing will be available in a future update.',
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

// ============ Education Section ============

class _EducationSection extends ConsumerStatefulWidget {
  @override
  ConsumerState<_EducationSection> createState() => _EducationSectionState();
}

class _EducationSectionState extends ConsumerState<_EducationSection> {
  late List<EducationData> _items;

  static const _degreeOrder = {
    'PhD': 0,
    'Master': 1,
    'Bachelor': 2,
    'Senior High': 3,
    'Junior High': 4,
    'Elementary': 5,
  };

  int _degreeSortOrder(EducationData e) {
    final degree = e.degree ?? '';
    if (e.degreeCustom != null &&
        e.degreeCustom!.isNotEmpty &&
        !_degreeOrder.containsKey(degree)) {
      return -1; // Custom degrees come before preset options
    }
    return _degreeOrder[degree] ?? 6;
  }

  void _loadData() {
    final professional = ref.read(profileNotifierProvider)?.professional;
    _items = [
      ...?(professional?.activeEducation.map(
        (e) => EducationData(
          id: e.id,
          institution: e.institution,
          degree: e.degree,
          degreeCustom: e.degreeCustom,
          field: e.field,
          startDate: e.startDate,
          endDate: e.endDate,
        ),
      )),
    ];
    _items.sort((a, b) => _degreeSortOrder(a).compareTo(_degreeSortOrder(b)));
  }

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

  EducationData _createFromValues(Map<String, String> values, {String? id}) {
    return EducationData(
      id: id ?? generateEntryId(),
      institution: values['education.institution']?.isEmpty == true
          ? null
          : values['education.institution'],
      degree: values['education.degree']?.isEmpty == true
          ? null
          : values['education.degree'],
      degreeCustom: values['education.degreeCustom']?.isEmpty == true
          ? null
          : values['education.degreeCustom'],
      field: values['education.fieldOfStudy']?.isEmpty == true
          ? null
          : values['education.fieldOfStudy'],
      startDate: values['education.startDate']?.isEmpty == true
          ? null
          : values['education.startDate'],
      endDate: values['education.endDate']?.isEmpty == true
          ? null
          : values['education.endDate'],
    );
  }

  Map<String, String> _educationToMap(EducationData edu) {
    return {
      'education.institution': edu.institution ?? '',
      'education.degree': edu.degree ?? '',
      'education.degreeCustom': edu.degreeCustom ?? '',
      'education.fieldOfStudy': edu.field ?? '',
      'education.startDate': edu.startDate ?? '',
      'education.endDate': edu.endDate ?? '',
    };
  }

  Future<void> _onDelete(EducationData item) async {
    final index = _items.indexOf(item);
    if (index == -1) return;
    final isPrivacyMode =
        ref.read(sensitivitySettingsProvider).displayMode ==
        SensitivityDisplayMode.hidePrivate;
    final deletedId = item.id;

    setState(() {
      _items = List.from(_items)..removeAt(index);
    });

    try {
      await ref
          .read(profileNotifierProvider.notifier)
          .softDelete(
            section: 'professional',
            itemType: 'education',
            index: index,
            deletedItem: item,
          );
    } catch (e) {
      setState(() {
        _items = List.from(_items)..insert(index, item);
      });
      if (mounted) {
        showOverlaySnackBar(
          context,
          content: 'Failed to delete education',
          type: SnackBarType.error,
        );
      }
      return;
    }

    if (mounted) {
      OperationNotification.show(
        context,
        message: OperationLogger.createNotification(
          section: LogSection.professional,
          action: LogAction.delete,
          itemName: item.institution ?? 'Education',
          isPrivacyModeActive: isPrivacyMode,
        ),
        duration: const Duration(seconds: 5),
        onUndo: () async {
          await ref
              .read(profileNotifierProvider.notifier)
              .restore(
                section: 'professional',
                itemType: 'education',
                id: deletedId,
              );
        },
      );
    }
  }

  Future<void> _onSave(
    EducationData? newItem,
    Map<String, String> values,
    EducationData? editingItem,
  ) async {
    final wasAdding = editingItem == null;
    final EducationData itemToSave;
    if (wasAdding) {
      itemToSave = newItem!;
    } else {
      itemToSave = _createFromValues(values, id: editingItem!.id);
    }
    if (wasAdding) {
      _items = List.from(_items)..add(itemToSave);
    } else {
      final index = _items.indexOf(editingItem);
      if (index != -1) {
        _items = List.from(_items)..[index] = itemToSave;
      }
    }
    final professional = ProfessionalData(
      education: _items,
      employment:
          ref.read(profileNotifierProvider)?.professional?.employment ?? [],
      skills: ref.read(profileNotifierProvider)?.professional?.skills ?? [],
      languages:
          ref.read(profileNotifierProvider)?.professional?.languages ?? [],
    );
    await ref
        .read(profileNotifierProvider.notifier)
        .updateProfessionalImmediate(professional);
    if (mounted) {
      final isPrivacyMode =
          ref.read(sensitivitySettingsProvider).displayMode ==
          SensitivityDisplayMode.hidePrivate;
      OperationNotification.show(
        context,
        message: OperationLogger.createNotification(
          section: LogSection.professional,
          action: wasAdding ? LogAction.create : LogAction.update,
          itemName: itemToSave.institution ?? 'Education',
          isPrivacyModeActive: isPrivacyMode,
        ),
      );
    }
  }

  static const _degreeOptions = [
    'Elementary',
    'Junior High',
    'Senior High',
    'Bachelor',
    'Master',
    'PhD',
  ];

  @override
  Widget build(BuildContext context) {
    return UnifiedFormSection<EducationData>(
      title: 'Education',
      icon: Icons.school_outlined,
      items: _items,
      maxVisibleItems: 3,
      itemFactory: _createFromValues,
      fieldDefs: const [
        FormFieldDef(
          fieldId: 'education.institution',
          label: 'Institution',
          sensitivity: SensitivityLevel.public,
        ),
        FormFieldDef(
          fieldId: 'education.degree',
          label: 'Degree',
          sensitivity: SensitivityLevel.public,
        ),
        FormFieldDef(
          fieldId: 'education.degreeCustom',
          label: 'Custom Degree',
          sensitivity: SensitivityLevel.public,
        ),
        FormFieldDef(
          fieldId: 'education.fieldOfStudy',
          label: 'Field of Study',
          sensitivity: SensitivityLevel.public,
        ),
        FormFieldDef(
          fieldId: 'education.startDate',
          label: 'Start Date',
          sensitivity: SensitivityLevel.public,
        ),
        FormFieldDef(
          fieldId: 'education.endDate',
          label: 'End Date',
          sensitivity: SensitivityLevel.public,
        ),
      ],
      customFormBuilder:
          (ctx, theme, controllers, mode, onSubmit, onCancel) {
            final degreeController = controllers['education.degree']!;
            final degreeCustomController =
                controllers['education.degreeCustom']!;
            final isCustomSelected = !_degreeOptions.contains(
              degreeController.text,
            );

            return Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisSize: MainAxisSize.min,
              children: [
                Text(
                  mode == 'adding' ? 'Add Education' : 'Edit Education',
                  style: theme.textTheme.titleSmall?.copyWith(
                    fontWeight: FontWeight.w600,
                  ),
                ),
                const SizedBox(height: 12),
                TextField(
                  controller: controllers['education.institution'],
                  decoration: const InputDecoration(
                    labelText: 'Institution',
                    border: OutlineInputBorder(),
                  ),
                ),
                const SizedBox(height: 12),
                Row(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Expanded(
                      child: DropdownButtonFormField<String>(
                        value: isCustomSelected
                            ? null
                            : (degreeController.text.isEmpty
                                  ? null
                                  : degreeController.text),
                        decoration: const InputDecoration(
                          labelText: 'Degree',
                          border: OutlineInputBorder(),
                        ),
                        items: [
                          ..._degreeOptions.map(
                            (d) => DropdownMenuItem(value: d, child: Text(d)),
                          ),
                          const DropdownMenuItem(
                            value: 'other',
                            child: Text('Other'),
                          ),
                        ],
                        onChanged: (value) {
                          if (value == 'other') {
                            degreeController.clear();
                          } else if (value != null) {
                            degreeController.text = value;
                            degreeCustomController.clear();
                          }
                        },
                      ),
                    ),
                    const SizedBox(width: 8),
                    Expanded(
                      child: TextField(
                        controller: degreeCustomController,
                        decoration: const InputDecoration(
                          labelText: 'Custom Degree',
                          hintText: 'Please specify',
                          border: OutlineInputBorder(),
                        ),
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 12),
                TextField(
                  controller: controllers['education.fieldOfStudy'],
                  decoration: const InputDecoration(
                    labelText: 'Field of Study',
                    border: OutlineInputBorder(),
                  ),
                ),
                const SizedBox(height: 12),
                TextField(
                  controller: controllers['education.startDate'],
                  decoration: const InputDecoration(
                    labelText: 'Start Date',
                    border: OutlineInputBorder(),
                  ),
                ),
                const SizedBox(height: 12),
                TextField(
                  controller: controllers['education.endDate'],
                  decoration: const InputDecoration(
                    labelText: 'End Date',
                    border: OutlineInputBorder(),
                  ),
                ),
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
      displayItemBuilder: (item) =>
          _EducationItem(item: item, onEdit: () {}, onDelete: () {}),
      onDelete: _onDelete,
      onSave: _onSave,
      itemToMap: _educationToMap,
      historyConfig: HistoryRecordingConfig<EducationData>(
        itemIdExtractor: (e) => e.id,
        fieldIdPrefix: 'education',
      ),
      historyAwareOnSave: (newItem, values, editingItem, [oldValues]) async {
        if (editingItem == null) return;
        final accountId = ref.read(authNotifierProvider.notifier).selectedAccountId;
        if (accountId == null) return;
        await ref.read(fieldHistoriesProvider.notifier).recordSnapshot(
          accountId: accountId,
          itemId: editingItem.id,
          fieldIdPrefix: 'education',
          allFieldValues: values,
        );
      },
    );
  }
}

class _EducationItem extends ConsumerStatefulWidget {
  final EducationData item;
  final VoidCallback onEdit;
  final VoidCallback onDelete;

  const _EducationItem({
    required this.item,
    required this.onEdit,
    required this.onDelete,
  });

  @override
  ConsumerState<_EducationItem> createState() => _EducationItemState();
}

class _EducationItemState extends ConsumerState<_EducationItem> {
  bool _historyExpanded = false;

  String _displayDegree(EducationData e) {
    if (e.degree != null && e.degree!.isNotEmpty) {
      return e.degree!;
    }
    if (e.degreeCustom != null && e.degreeCustom!.isNotEmpty) {
      return e.degreeCustom!;
    }
    return '';
  }

  @override
  Widget build(BuildContext context) {
    final subtitleParts = [
      _displayDegree(widget.item),
      widget.item.field,
    ].where((p) => p != null && p.isNotEmpty).join(' - ');

    final fields = <LabelValueField>[
      if (_displayDegree(widget.item).isNotEmpty)
        LabelValueField(label: 'Degree', value: _displayDegree(widget.item)),
      if (widget.item.field != null && widget.item.field!.isNotEmpty)
        LabelValueField(label: 'Field', value: widget.item.field!),
      if (widget.item.startDate != null && widget.item.startDate!.isNotEmpty)
        LabelValueField(label: 'Start Date', value: widget.item.startDate!),
      if (widget.item.endDate != null && widget.item.endDate!.isNotEmpty)
        LabelValueField(label: 'End Date', value: widget.item.endDate!),
    ];

    final history = ref
        .watch(fieldHistoriesProvider.notifier)
        .getHistory(widget.item.id, 'education');
    final hasHistory = history != null;

    // Get actions from EntryActionsContext (set by UnifiedFormSection)
    final actionsContext = EntryActionsContext.of(context);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        UniversalEntryCard(
          title: SelectableText(
            widget.item.institution ?? 'Institution',
            style: Theme.of(
              context,
            ).textTheme.bodyLarge?.copyWith(fontWeight: FontWeight.w500),
          ),
          subtitle: subtitleParts.isEmpty
              ? null
              : SelectableText(
                  subtitleParts,
                  style: Theme.of(context).textTheme.bodySmall?.copyWith(
                    color: Theme.of(context).colorScheme.onSurfaceVariant,
                  ),
                ),
          leading: Icon(
            Icons.school,
            size: 20,
            color: Theme.of(context).colorScheme.onSurfaceVariant,
          ),
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
          actions: actionsContext != null
              ? EntryActionBuilder.buildActions(
                  context: context,
                  ref: ref,
                  onCopy: () {
                    final formatted = '${widget.item.entryType}\n${widget.item.toFormattedString()}';
                    Clipboard.setData(ClipboardData(text: formatted));
                    showOverlaySnackBar(
                      context,
                      content: 'Copied to clipboard',
                      type: SnackBarType.success,
                    );
                  },
                  onEdit: actionsContext.onEdit ?? widget.onEdit,
                  onDelete: actionsContext.onDelete ?? widget.onDelete,
                  config: EntryActionsConfig(
                    showCopy: true,
                    showEdit: true,
                    showDelete: true,
                    showHistory: true,
                  ),
                  hasHistory: hasHistory,
                  historyExpanded: _historyExpanded,
                  onHistoryToggle: () =>
                      setState(() => _historyExpanded = !_historyExpanded),
                  isSensitive: fields.any((f) => f.isSensitive),
                )
              : [
                  IconButton(
                    icon: const Icon(Icons.edit_outlined, size: 20),
                    tooltip: 'Edit',
                    onPressed: widget.onEdit,
                  ),
                  IconButton(
                    icon: const Icon(Icons.delete_outline, size: 20),
                    tooltip: 'Delete',
                    onPressed: widget.onDelete,
                  ),
                ],
        ),
        if (hasHistory && _historyExpanded)
          Padding(
            padding: const EdgeInsets.only(left: 32, bottom: 8),
            child: FieldHistoryView(
              fieldName: 'education',
              history: history,
            ),
          ),
      ],
    );
  }
}

// ============ Employment Section ============

class _EmploymentSection extends ConsumerStatefulWidget {
  @override
  ConsumerState<_EmploymentSection> createState() => _EmploymentSectionState();
}

class _EmploymentSectionState extends ConsumerState<_EmploymentSection> {
  late List<EmploymentData> _items;

  @override
  void initState() {
    super.initState();
    _loadData();
  }

  void _loadData() {
    final professional = ref.read(profileNotifierProvider)?.professional;
    _items = [
      ...?(professional?.activeEmployment.map(
        (e) => EmploymentData(
          id: e.id,
          company: e.company,
          position: e.position,
          responsibilities: e.responsibilities,
          startDate: e.startDate,
          endDate: e.endDate,
        ),
      )),
    ];
  }

  EmploymentData _createFromValues(Map<String, String> values, {String? id}) {
    return EmploymentData(
      id: id ?? generateEntryId(),
      company: values['employment.company']?.isEmpty == true
          ? null
          : values['employment.company'],
      position: values['employment.position']?.isEmpty == true
          ? null
          : values['employment.position'],
      responsibilities: values['employment.responsibilities']?.isEmpty == true
          ? null
          : values['employment.responsibilities'],
      startDate: values['employment.startDate']?.isEmpty == true
          ? null
          : values['employment.startDate'],
      endDate: values['employment.endDate']?.isEmpty == true
          ? null
          : values['employment.endDate'],
    );
  }

  Map<String, String> _employmentToMap(EmploymentData emp) {
    return {
      'employment.company': emp.company ?? '',
      'employment.position': emp.position ?? '',
      'employment.responsibilities': emp.responsibilities ?? '',
      'employment.startDate': emp.startDate ?? '',
      'employment.endDate': emp.endDate ?? '',
    };
  }

  Future<void> _onDelete(EmploymentData item) async {
    final index = _items.indexOf(item);
    if (index == -1) return;
    final isPrivacyMode =
        ref.read(sensitivitySettingsProvider).displayMode ==
        SensitivityDisplayMode.hidePrivate;
    final deletedId = item.id;

    setState(() {
      _items = List.from(_items)..removeAt(index);
    });

    try {
      await ref
          .read(profileNotifierProvider.notifier)
          .softDelete(
            section: 'professional',
            itemType: 'employment',
            index: index,
            deletedItem: item,
          );
    } catch (e) {
      setState(() {
        _items = List.from(_items)..insert(index, item);
      });
      if (mounted) {
        showOverlaySnackBar(
          context,
          content: 'Failed to delete employment',
          type: SnackBarType.error,
        );
      }
      return;
    }

    if (mounted) {
      OperationNotification.show(
        context,
        message: OperationLogger.createNotification(
          section: LogSection.professional,
          action: LogAction.delete,
          itemName: item.company ?? 'Employment',
          isPrivacyModeActive: isPrivacyMode,
        ),
        duration: const Duration(seconds: 5),
        onUndo: () async {
          await ref
              .read(profileNotifierProvider.notifier)
              .restore(
                section: 'professional',
                itemType: 'employment',
                id: deletedId,
              );
        },
      );
    }
  }

  Future<void> _onSave(
    EmploymentData? newItem,
    Map<String, String> values,
    EmploymentData? editingItem,
  ) async {
    final wasAdding = editingItem == null;
    final EmploymentData itemToSave;
    if (wasAdding) {
      itemToSave = newItem!;
    } else {
      itemToSave = _createFromValues(values, id: editingItem!.id);
    }
    if (wasAdding) {
      _items = List.from(_items)..add(itemToSave);
    } else {
      final index = _items.indexOf(editingItem);
      if (index != -1) {
        _items = List.from(_items)..[index] = itemToSave;
      }
    }
    final professional = ProfessionalData(
      education:
          ref.read(profileNotifierProvider)?.professional?.education ?? [],
      employment: _items,
      skills: ref.read(profileNotifierProvider)?.professional?.skills ?? [],
      languages:
          ref.read(profileNotifierProvider)?.professional?.languages ?? [],
      awards: ref.read(profileNotifierProvider)?.professional?.awards ?? [],
    );
    await ref
        .read(profileNotifierProvider.notifier)
        .updateProfessionalImmediate(professional);
    if (mounted) {
      final isPrivacyMode =
          ref.read(sensitivitySettingsProvider).displayMode ==
          SensitivityDisplayMode.hidePrivate;
      OperationNotification.show(
        context,
        message: OperationLogger.createNotification(
          section: LogSection.professional,
          action: wasAdding ? LogAction.create : LogAction.update,
          itemName: itemToSave.company ?? 'Employment',
          isPrivacyModeActive: isPrivacyMode,
        ),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    return UnifiedFormSection<EmploymentData>(
      title: 'Employment',
      icon: Icons.work_outlined,
      items: _items,
      maxVisibleItems: 3,
      itemFactory: _createFromValues,
      fieldDefs: const [
        FormFieldDef(
          fieldId: 'employment.company',
          label: 'Company',
          sensitivity: SensitivityLevel.public,
        ),
        FormFieldDef(
          fieldId: 'employment.position',
          label: 'Position',
          sensitivity: SensitivityLevel.public,
        ),
        FormFieldDef(
          fieldId: 'employment.responsibilities',
          label: 'Responsibilities',
          sensitivity: SensitivityLevel.public,
        ),
        FormFieldDef(
          fieldId: 'employment.startDate',
          label: 'Start Date',
          sensitivity: SensitivityLevel.public,
        ),
        FormFieldDef(
          fieldId: 'employment.endDate',
          label: 'End Date',
          sensitivity: SensitivityLevel.public,
        ),
      ],
      displayItemBuilder: (item) =>
          _EmploymentItem(item: item, onEdit: () {}, onDelete: () {}),
      onDelete: _onDelete,
      onSave: _onSave,
      itemToMap: _employmentToMap,
      historyConfig: HistoryRecordingConfig<EmploymentData>(
        itemIdExtractor: (e) => e.id,
        fieldIdPrefix: 'employment',
      ),
      historyAwareOnSave: (newItem, values, editingItem, [oldValues]) async {
        if (editingItem == null) return;
        final accountId = ref.read(authNotifierProvider.notifier).selectedAccountId;
        if (accountId == null) return;
        await ref.read(fieldHistoriesProvider.notifier).recordSnapshot(
          accountId: accountId,
          itemId: editingItem.id,
          fieldIdPrefix: 'employment',
          allFieldValues: values,
        );
      },
    );
  }
}

class _EmploymentItem extends ConsumerStatefulWidget {
  final EmploymentData item;
  final VoidCallback onEdit;
  final VoidCallback onDelete;

  const _EmploymentItem({
    required this.item,
    required this.onEdit,
    required this.onDelete,
  });

  @override
  ConsumerState<_EmploymentItem> createState() => _EmploymentItemState();
}

class _EmploymentItemState extends ConsumerState<_EmploymentItem> {
  bool _historyExpanded = false;

  @override
  Widget build(BuildContext context) {
    final fields = <LabelValueField>[
      if (widget.item.position != null && widget.item.position!.isNotEmpty)
        LabelValueField(label: 'Position', value: widget.item.position!),
      if (widget.item.responsibilities != null && widget.item.responsibilities!.isNotEmpty)
        LabelValueField(
          label: 'Responsibilities',
          value: widget.item.responsibilities!,
        ),
      if (widget.item.startDate != null && widget.item.startDate!.isNotEmpty)
        LabelValueField(label: 'Start Date', value: widget.item.startDate!),
      if (widget.item.endDate != null && widget.item.endDate!.isNotEmpty)
        LabelValueField(label: 'End Date', value: widget.item.endDate!),
    ];

    final history = ref
        .watch(fieldHistoriesProvider.notifier)
        .getHistory(widget.item.id, 'employment');
    final hasHistory = history != null;

    final actionsContext = EntryActionsContext.of(context);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        UniversalEntryCard(
          title: SelectableText(
            widget.item.company ?? 'Company',
            style: Theme.of(
              context,
            ).textTheme.bodyLarge?.copyWith(fontWeight: FontWeight.w500),
          ),
          subtitle: widget.item.position != null
              ? SelectableText(
                  widget.item.position!,
                  style: Theme.of(context).textTheme.bodySmall?.copyWith(
                    color: Theme.of(context).colorScheme.onSurfaceVariant,
                  ),
                )
              : null,
          leading: Icon(
            Icons.work,
            size: 20,
            color: Theme.of(context).colorScheme.onSurfaceVariant,
          ),
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
          actions: actionsContext != null
              ? EntryActionBuilder.buildActions(
                  context: context,
                  ref: ref,
                  onCopy: () {
                    final formatted = '${widget.item.entryType}\n${widget.item.toFormattedString()}';
                    Clipboard.setData(ClipboardData(text: formatted));
                    showOverlaySnackBar(
                      context,
                      content: 'Copied to clipboard',
                      type: SnackBarType.success,
                    );
                  },
                  onEdit: actionsContext.onEdit ?? widget.onEdit,
                  onDelete: actionsContext.onDelete ?? widget.onDelete,
                  config: EntryActionsConfig(
                    showCopy: true,
                    showEdit: true,
                    showDelete: true,
                    showHistory: true,
                  ),
                  hasHistory: hasHistory,
                  historyExpanded: _historyExpanded,
                  onHistoryToggle: () =>
                      setState(() => _historyExpanded = !_historyExpanded),
                  isSensitive: fields.any((f) => f.isSensitive),
                )
              : [
                  IconButton(
                    icon: const Icon(Icons.edit_outlined, size: 20),
                    tooltip: 'Edit',
                    onPressed: widget.onEdit,
                  ),
                  IconButton(
                    icon: const Icon(Icons.delete_outline, size: 20),
                    tooltip: 'Delete',
                    onPressed: widget.onDelete,
                  ),
                ],
        ),
        if (hasHistory && _historyExpanded)
          Padding(
            padding: const EdgeInsets.only(left: 32, bottom: 8),
            child: FieldHistoryView(
              fieldName: 'employment',
              history: history,
            ),
          ),
      ],
    );
  }
}

// ============ Skills Section ============

class _SkillsSection extends ConsumerStatefulWidget {
  @override
  ConsumerState<_SkillsSection> createState() => _SkillsSectionState();
}

class _SkillsSectionState extends ConsumerState<_SkillsSection> {
  late List<SkillData> _items;

  @override
  void initState() {
    super.initState();
    _loadData();
  }

  void _loadData() {
    final professional = ref.read(profileNotifierProvider)?.professional;
    _items = [...?(professional?.activeSkills)];
  }

  SkillData _createFromValues(Map<String, String> values, {String? id}) {
    final name = values['skill.name']?.trim() ?? '';
    final level = values['skill.level']?.trim() ?? '';
    return SkillData(
      id: id ?? generateEntryId(),
      name: name,
      level: level.isEmpty ? null : level,
    );
  }

  Map<String, String> _skillToMap(SkillData skill) {
    return {'skill.name': skill.name, 'skill.level': skill.level ?? ''};
  }

  Future<void> _onDelete(SkillData item) async {
    final index = _items.indexOf(item);
    if (index == -1) return;
    final isPrivacyMode =
        ref.read(sensitivitySettingsProvider).displayMode ==
        SensitivityDisplayMode.hidePrivate;
    final deletedId = item.id;

    setState(() {
      _items = List.from(_items)..removeAt(index);
    });

    try {
      await ref
          .read(profileNotifierProvider.notifier)
          .softDelete(
            section: 'professional',
            itemType: 'skill',
            index: index,
            deletedItem: item,
          );
    } catch (e) {
      setState(() {
        _items = List.from(_items)..insert(index, item);
      });
      if (mounted) {
        showOverlaySnackBar(
          context,
          content: 'Failed to delete skill',
          type: SnackBarType.error,
        );
      }
      return;
    }

    if (mounted) {
      OperationNotification.show(
        context,
        message: OperationLogger.createNotification(
          section: LogSection.professional,
          action: LogAction.delete,
          itemName: item.toString(),
          isPrivacyModeActive: isPrivacyMode,
        ),
        duration: const Duration(seconds: 5),
        onUndo: () async {
          await ref
              .read(profileNotifierProvider.notifier)
              .restore(
                section: 'professional',
                itemType: 'skill',
                id: deletedId,
              );
        },
      );
    }
  }

  Future<void> _onSave(
    SkillData? newItem,
    Map<String, String> values,
    SkillData? editingItem,
  ) async {
    final wasAdding = editingItem == null;
    final SkillData itemToSave;
    if (wasAdding) {
      itemToSave = newItem!;
    } else {
      itemToSave = _createFromValues(values, id: editingItem!.id);
    }
    if (itemToSave.name.isEmpty) return;
    if (wasAdding) {
      _items = List.from(_items)..add(itemToSave);
    } else {
      final index = _items.indexOf(editingItem);
      if (index != -1) {
        _items = List.from(_items)..[index] = itemToSave;
      }
    }
    final professional = ProfessionalData(
      education:
          ref.read(profileNotifierProvider)?.professional?.education ?? [],
      employment:
          ref.read(profileNotifierProvider)?.professional?.employment ?? [],
      skills: _items,
      languages:
          ref.read(profileNotifierProvider)?.professional?.languages ?? [],
    );
    await ref
        .read(profileNotifierProvider.notifier)
        .updateProfessionalImmediate(professional);
    if (mounted) {
      final isPrivacyMode =
          ref.read(sensitivitySettingsProvider).displayMode ==
          SensitivityDisplayMode.hidePrivate;
      OperationNotification.show(
        context,
        message: OperationLogger.createNotification(
          section: LogSection.professional,
          action: wasAdding ? LogAction.create : LogAction.update,
          itemName: itemToSave.toString(),
          isPrivacyModeActive: isPrivacyMode,
        ),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    return UnifiedFormSection<SkillData>(
      title: 'Skills',
      icon: Icons.star_outline,
      items: _items,
      maxVisibleItems: 3,
      itemFactory: _createFromValues,
      fieldDefs: const [
        FormFieldDef(
          fieldId: 'skill.name',
          label: 'Skill Name',
          sensitivity: SensitivityLevel.public,
        ),
        FormFieldDef(
          fieldId: 'skill.level',
          label: 'Level (e.g. Beginner, Intermediate, Expert)',
          sensitivity: SensitivityLevel.public,
        ),
      ],
      displayItemBuilder: (item) =>
          _SkillItem(item: item, onEdit: () {}, onDelete: () {}),
      onDelete: _onDelete,
      onSave: _onSave,
      itemToMap: _skillToMap,
      historyConfig: HistoryRecordingConfig<SkillData>(
        itemIdExtractor: (s) => s.id,
        fieldIdPrefix: 'skill',
      ),
      historyAwareOnSave: (newItem, values, editingItem, [oldValues]) async {
        if (editingItem == null) return;
        final accountId = ref.read(authNotifierProvider.notifier).selectedAccountId;
        if (accountId == null) return;
        await ref.read(fieldHistoriesProvider.notifier).recordSnapshot(
          accountId: accountId,
          itemId: editingItem.id,
          fieldIdPrefix: 'skill',
          allFieldValues: values,
        );
      },
    );
  }
}

class _SkillItem extends ConsumerStatefulWidget {
  final SkillData item;
  final VoidCallback onEdit;
  final VoidCallback onDelete;

  const _SkillItem({
    required this.item,
    required this.onEdit,
    required this.onDelete,
  });

  @override
  ConsumerState<_SkillItem> createState() => _SkillItemState();
}

class _SkillItemState extends ConsumerState<_SkillItem> {
  bool _historyExpanded = false;

  @override
  Widget build(BuildContext context) {
    final fields = <LabelValueField>[
      if (widget.item.level != null && widget.item.level!.isNotEmpty)
        LabelValueField(label: 'Proficiency', value: widget.item.level!),
    ];

    final history = ref
        .watch(fieldHistoriesProvider.notifier)
        .getHistory(widget.item.id, 'skill');
    final hasHistory = history != null;

    final actionsContext = EntryActionsContext.of(context);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        UniversalEntryCard(
          title: SelectableText(
            widget.item.name,
            style: Theme.of(
              context,
            ).textTheme.bodyLarge?.copyWith(fontWeight: FontWeight.w500),
          ),
          leading: Icon(
            Icons.star,
            size: 20,
            color: Theme.of(context).colorScheme.onSurfaceVariant,
          ),
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
          actions: actionsContext != null
              ? EntryActionBuilder.buildActions(
                  context: context,
                  ref: ref,
                  onCopy: () {
                    final formatted = '${widget.item.entryType}\n${widget.item.toFormattedString()}';
                    Clipboard.setData(ClipboardData(text: formatted));
                    showOverlaySnackBar(
                      context,
                      content: 'Copied to clipboard',
                      type: SnackBarType.success,
                    );
                  },
                  onEdit: actionsContext.onEdit ?? widget.onEdit,
                  onDelete: actionsContext.onDelete ?? widget.onDelete,
                  config: EntryActionsConfig(
                    showCopy: true,
                    showEdit: true,
                    showDelete: true,
                    showHistory: true,
                  ),
                  hasHistory: hasHistory,
                  historyExpanded: _historyExpanded,
                  onHistoryToggle: () =>
                      setState(() => _historyExpanded = !_historyExpanded),
                  isSensitive: fields.any((f) => f.isSensitive),
                )
              : [
                  IconButton(
                    icon: const Icon(Icons.edit_outlined, size: 20),
                    tooltip: 'Edit',
                    onPressed: widget.onEdit,
                  ),
                  IconButton(
                    icon: const Icon(Icons.delete_outline, size: 20),
                    tooltip: 'Delete',
                    onPressed: widget.onDelete,
                  ),
                ],
        ),
        if (hasHistory && _historyExpanded)
          Padding(
            padding: const EdgeInsets.only(left: 32, bottom: 8),
            child: FieldHistoryView(
              fieldName: 'skill',
              history: history,
            ),
          ),
      ],
    );
  }
}

// ============ Language Section ============

class _LanguageSection extends ConsumerStatefulWidget {
  @override
  ConsumerState<_LanguageSection> createState() => _LanguageSectionState();
}

class _LanguageSectionState extends ConsumerState<_LanguageSection> {
  late List<LanguageData> _items;

  @override
  void initState() {
    super.initState();
    _loadData();
  }

  void _loadData() {
    final professional = ref.read(profileNotifierProvider)?.professional;
    _items = [...?(professional?.activeLanguages)];
  }

  LanguageData _createFromValues(Map<String, String> values, {String? id}) {
    final name = values['language.name']?.trim() ?? '';
    final proficiency = values['language.proficiency']?.trim() ?? '';
    return LanguageData(
      id: id ?? generateEntryId(),
      name: name,
      proficiency: proficiency.isEmpty ? null : proficiency,
    );
  }

  Map<String, String> _languageToMap(LanguageData language) {
    return {
      'language.name': language.name,
      'language.proficiency': language.proficiency ?? '',
    };
  }

  Future<void> _onDelete(LanguageData item) async {
    final index = _items.indexOf(item);
    if (index == -1) return;
    final isPrivacyMode =
        ref.read(sensitivitySettingsProvider).displayMode ==
        SensitivityDisplayMode.hidePrivate;
    final deletedId = item.id;

    setState(() {
      _items = List.from(_items)..removeAt(index);
    });

    try {
      await ref
          .read(profileNotifierProvider.notifier)
          .softDelete(
            section: 'professional',
            itemType: 'language',
            index: index,
            deletedItem: item,
          );
    } catch (e) {
      setState(() {
        _items = List.from(_items)..insert(index, item);
      });
      if (mounted) {
        showOverlaySnackBar(
          context,
          content: 'Failed to delete language',
          type: SnackBarType.error,
        );
      }
      return;
    }

    if (mounted) {
      OperationNotification.show(
        context,
        message: OperationLogger.createNotification(
          section: LogSection.professional,
          action: LogAction.delete,
          itemName: item.toString(),
          isPrivacyModeActive: isPrivacyMode,
        ),
        duration: const Duration(seconds: 5),
        onUndo: () async {
          await ref
              .read(profileNotifierProvider.notifier)
              .restore(
                section: 'professional',
                itemType: 'language',
                id: deletedId,
              );
        },
      );
    }
  }

  Future<void> _onSave(
    LanguageData? newItem,
    Map<String, String> values,
    LanguageData? editingItem,
  ) async {
    final wasAdding = editingItem == null;
    final LanguageData itemToSave;
    if (wasAdding) {
      itemToSave = newItem!;
    } else {
      itemToSave = _createFromValues(values, id: editingItem!.id);
    }
    if (itemToSave.name.isEmpty) return;
    if (wasAdding) {
      _items = List.from(_items)..add(itemToSave);
    } else {
      final index = _items.indexOf(editingItem);
      if (index != -1) {
        _items = List.from(_items)..[index] = itemToSave;
      }
    }
    final professional = ProfessionalData(
      education:
          ref.read(profileNotifierProvider)?.professional?.education ?? [],
      employment:
          ref.read(profileNotifierProvider)?.professional?.employment ?? [],
      skills: ref.read(profileNotifierProvider)?.professional?.skills ?? [],
      languages: _items,
    );
    await ref
        .read(profileNotifierProvider.notifier)
        .updateProfessionalImmediate(professional);
    if (mounted) {
      final isPrivacyMode =
          ref.read(sensitivitySettingsProvider).displayMode ==
          SensitivityDisplayMode.hidePrivate;
      OperationNotification.show(
        context,
        message: OperationLogger.createNotification(
          section: LogSection.professional,
          action: wasAdding ? LogAction.create : LogAction.update,
          itemName: itemToSave.toString(),
          isPrivacyModeActive: isPrivacyMode,
        ),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    return UnifiedFormSection<LanguageData>(
      title: 'Languages',
      icon: Icons.translate,
      items: _items,
      maxVisibleItems: 3,
      itemFactory: _createFromValues,
      fieldDefs: const [
        FormFieldDef(
          fieldId: 'language.name',
          label: 'Language',
          sensitivity: SensitivityLevel.public,
        ),
        FormFieldDef(
          fieldId: 'language.proficiency',
          label: 'Proficiency (e.g. Native, Fluent, Intermediate)',
          sensitivity: SensitivityLevel.public,
        ),
      ],
      displayItemBuilder: (item) =>
          _LanguageItem(item: item, onEdit: () {}, onDelete: () {}),
      onDelete: _onDelete,
      onSave: _onSave,
      itemToMap: _languageToMap,
      historyConfig: HistoryRecordingConfig<LanguageData>(
        itemIdExtractor: (l) => l.id,
        fieldIdPrefix: 'language',
      ),
      historyAwareOnSave: (newItem, values, editingItem, [oldValues]) async {
        if (editingItem == null) return;
        final accountId = ref.read(authNotifierProvider.notifier).selectedAccountId;
        if (accountId == null) return;
        await ref.read(fieldHistoriesProvider.notifier).recordSnapshot(
          accountId: accountId,
          itemId: editingItem.id,
          fieldIdPrefix: 'language',
          allFieldValues: values,
        );
      },
    );
  }
}

class _LanguageItem extends ConsumerStatefulWidget {
  final LanguageData item;
  final VoidCallback onEdit;
  final VoidCallback onDelete;

  const _LanguageItem({
    required this.item,
    required this.onEdit,
    required this.onDelete,
  });

  @override
  ConsumerState<_LanguageItem> createState() => _LanguageItemState();
}

class _LanguageItemState extends ConsumerState<_LanguageItem> {
  bool _historyExpanded = false;

  @override
  Widget build(BuildContext context) {
    final fields = <LabelValueField>[
      if (widget.item.proficiency != null && widget.item.proficiency!.isNotEmpty)
        LabelValueField(label: 'Proficiency', value: widget.item.proficiency!),
    ];

    final history = ref
        .watch(fieldHistoriesProvider.notifier)
        .getHistory(widget.item.id, 'language');
    final hasHistory = history != null;

    final actionsContext = EntryActionsContext.of(context);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        UniversalEntryCard(
          title: SelectableText(
            widget.item.name,
            style: Theme.of(
              context,
            ).textTheme.bodyLarge?.copyWith(fontWeight: FontWeight.w500),
          ),
          leading: Icon(
            Icons.translate,
            size: 20,
            color: Theme.of(context).colorScheme.onSurfaceVariant,
          ),
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
          actions: actionsContext != null
              ? EntryActionBuilder.buildActions(
                  context: context,
                  ref: ref,
                  onCopy: () {
                    final formatted = '${widget.item.entryType}\n${widget.item.toFormattedString()}';
                    Clipboard.setData(ClipboardData(text: formatted));
                    showOverlaySnackBar(
                      context,
                      content: 'Copied to clipboard',
                      type: SnackBarType.success,
                    );
                  },
                  onEdit: actionsContext.onEdit ?? widget.onEdit,
                  onDelete: actionsContext.onDelete ?? widget.onDelete,
                  config: EntryActionsConfig(
                    showCopy: true,
                    showEdit: true,
                    showDelete: true,
                    showHistory: true,
                  ),
                  hasHistory: hasHistory,
                  historyExpanded: _historyExpanded,
                  onHistoryToggle: () =>
                      setState(() => _historyExpanded = !_historyExpanded),
                  isSensitive: fields.any((f) => f.isSensitive),
                )
              : [
                  IconButton(
                    icon: const Icon(Icons.edit_outlined, size: 20),
                    tooltip: 'Edit',
                    onPressed: widget.onEdit,
                  ),
                  IconButton(
                    icon: const Icon(Icons.delete_outline, size: 20),
                    tooltip: 'Delete',
                    onPressed: widget.onDelete,
                  ),
                ],
        ),
        if (hasHistory && _historyExpanded)
          Padding(
            padding: const EdgeInsets.only(left: 32, bottom: 8),
            child: FieldHistoryView(
              fieldName: 'language',
              history: history,
            ),
          ),
      ],
    );
  }
}

// ============ Award Section ============

class _AwardSection extends ConsumerStatefulWidget {
  @override
  ConsumerState<_AwardSection> createState() => _AwardSectionState();
}

class _AwardSectionState extends ConsumerState<_AwardSection> {
  late List<AwardData> _items;

  @override
  void initState() {
    super.initState();
    _loadData();
  }

  void _loadData() {
    final professional = ref.read(profileNotifierProvider)?.professional;
    _items = [...?(professional?.activeAwards)];
  }

  AwardData _createFromValues(Map<String, String> values, {String? id}) {
    return AwardData(
      id: id ?? generateEntryId(),
      title: values['award.title']?.isEmpty == true
          ? null
          : values['award.title'],
      issuer: values['award.issuer']?.isEmpty == true
          ? null
          : values['award.issuer'],
      date: values['award.date']?.isEmpty == true ? null : values['award.date'],
      description: values['award.description']?.isEmpty == true
          ? null
          : values['award.description'],
    );
  }

  Map<String, String> _awardToMap(AwardData award) {
    return {
      'award.title': award.title ?? '',
      'award.issuer': award.issuer ?? '',
      'award.date': award.date ?? '',
      'award.description': award.description ?? '',
    };
  }

  Future<void> _onDelete(AwardData item) async {
    final index = _items.indexOf(item);
    if (index == -1) return;
    final isPrivacyMode =
        ref.read(sensitivitySettingsProvider).displayMode ==
        SensitivityDisplayMode.hidePrivate;
    final deletedId = item.id;

    setState(() {
      _items = List.from(_items)..removeAt(index);
    });

    try {
      await ref
          .read(profileNotifierProvider.notifier)
          .softDelete(
            section: 'professional',
            itemType: 'award',
            index: index,
            deletedItem: item,
          );
    } catch (e) {
      setState(() {
        _items = List.from(_items)..insert(index, item);
      });
      if (mounted) {
        showOverlaySnackBar(
          context,
          content: 'Failed to delete award',
          type: SnackBarType.error,
        );
      }
      return;
    }

    if (mounted) {
      OperationNotification.show(
        context,
        message: OperationLogger.createNotification(
          section: LogSection.professional,
          action: LogAction.delete,
          itemName: item.title ?? 'Award',
          isPrivacyModeActive: isPrivacyMode,
        ),
        duration: const Duration(seconds: 5),
        onUndo: () async {
          await ref
              .read(profileNotifierProvider.notifier)
              .restore(
                section: 'professional',
                itemType: 'award',
                id: deletedId,
              );
        },
      );
    }
  }

  Future<void> _onSave(
    AwardData? newItem,
    Map<String, String> values,
    AwardData? editingItem,
  ) async {
    final wasAdding = editingItem == null;
    final AwardData itemToSave;
    if (wasAdding) {
      itemToSave = newItem!;
    } else {
      itemToSave = _createFromValues(values, id: editingItem!.id);
    }
    if (itemToSave.title == null || itemToSave.title!.isEmpty) return;
    if (wasAdding) {
      _items = List.from(_items)..add(itemToSave);
    } else {
      final index = _items.indexOf(editingItem);
      if (index != -1) {
        _items = List.from(_items)..[index] = itemToSave;
      }
    }
    final professional = ProfessionalData(
      education:
          ref.read(profileNotifierProvider)?.professional?.education ?? [],
      employment:
          ref.read(profileNotifierProvider)?.professional?.employment ?? [],
      skills: ref.read(profileNotifierProvider)?.professional?.skills ?? [],
      languages:
          ref.read(profileNotifierProvider)?.professional?.languages ?? [],
      awards: _items,
    );
    await ref
        .read(profileNotifierProvider.notifier)
        .updateProfessionalImmediate(professional);
    if (mounted) {
      final isPrivacyMode =
          ref.read(sensitivitySettingsProvider).displayMode ==
          SensitivityDisplayMode.hidePrivate;
      OperationNotification.show(
        context,
        message: OperationLogger.createNotification(
          section: LogSection.professional,
          action: wasAdding ? LogAction.create : LogAction.update,
          itemName: itemToSave.title ?? 'Award',
          isPrivacyModeActive: isPrivacyMode,
        ),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    return UnifiedFormSection<AwardData>(
      title: 'Awards',
      icon: Icons.emoji_events_outlined,
      items: _items,
      maxVisibleItems: 3,
      itemFactory: _createFromValues,
      fieldDefs: const [
        FormFieldDef(
          fieldId: 'award.title',
          label: 'Title',
          sensitivity: SensitivityLevel.public,
        ),
        FormFieldDef(
          fieldId: 'award.issuer',
          label: 'Issuer',
          sensitivity: SensitivityLevel.public,
        ),
        FormFieldDef(
          fieldId: 'award.date',
          label: 'Date',
          sensitivity: SensitivityLevel.public,
        ),
        FormFieldDef(
          fieldId: 'award.description',
          label: 'Description',
          sensitivity: SensitivityLevel.public,
        ),
      ],
      displayItemBuilder: (item) =>
          _AwardItem(item: item, onEdit: () {}, onDelete: () {}),
      onDelete: _onDelete,
      onSave: _onSave,
      itemToMap: _awardToMap,
      historyConfig: HistoryRecordingConfig<AwardData>(
        itemIdExtractor: (a) => a.id,
        fieldIdPrefix: 'award',
      ),
      historyAwareOnSave: (newItem, values, editingItem, [oldValues]) async {
        if (editingItem == null) return;
        final accountId = ref.read(authNotifierProvider.notifier).selectedAccountId;
        if (accountId == null) return;
        await ref.read(fieldHistoriesProvider.notifier).recordSnapshot(
          accountId: accountId,
          itemId: editingItem.id,
          fieldIdPrefix: 'award',
          allFieldValues: values,
        );
      },
    );
  }
}

class _AwardItem extends ConsumerStatefulWidget {
  final AwardData item;
  final VoidCallback onEdit;
  final VoidCallback onDelete;

  const _AwardItem({
    required this.item,
    required this.onEdit,
    required this.onDelete,
  });

  @override
  ConsumerState<_AwardItem> createState() => _AwardItemState();
}

class _AwardItemState extends ConsumerState<_AwardItem> {
  bool _historyExpanded = false;

  @override
  Widget build(BuildContext context) {
    final fields = <LabelValueField>[
      if (widget.item.date != null && widget.item.date!.isNotEmpty)
        LabelValueField(label: 'Date', value: widget.item.date!),
      if (widget.item.description != null && widget.item.description!.isNotEmpty)
        LabelValueField(label: 'Description', value: widget.item.description!),
    ];

    final history = ref
        .watch(fieldHistoriesProvider.notifier)
        .getHistory(widget.item.id, 'award');
    final hasHistory = history != null;

    final actionsContext = EntryActionsContext.of(context);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        UniversalEntryCard(
          title: SelectableText(
            widget.item.title ?? 'Award',
            style: Theme.of(
              context,
            ).textTheme.bodyLarge?.copyWith(fontWeight: FontWeight.w500),
          ),
          subtitle: widget.item.issuer != null
              ? SelectableText(
                  widget.item.issuer!,
                  style: Theme.of(context).textTheme.bodySmall?.copyWith(
                    color: Theme.of(context).colorScheme.onSurfaceVariant,
                  ),
                )
              : null,
          leading: Icon(
            Icons.emoji_events,
            size: 20,
            color: Theme.of(context).colorScheme.onSurfaceVariant,
          ),
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
          actions: actionsContext != null
              ? EntryActionBuilder.buildActions(
                  context: context,
                  ref: ref,
                  onCopy: () {
                    final formatted = '${widget.item.entryType}\n${widget.item.toFormattedString()}';
                    Clipboard.setData(ClipboardData(text: formatted));
                    showOverlaySnackBar(
                      context,
                      content: 'Copied to clipboard',
                      type: SnackBarType.success,
                    );
                  },
                  onEdit: actionsContext.onEdit ?? widget.onEdit,
                  onDelete: actionsContext.onDelete ?? widget.onDelete,
                  config: EntryActionsConfig(
                    showCopy: true,
                    showEdit: true,
                    showDelete: true,
                    showHistory: true,
                  ),
                  hasHistory: hasHistory,
                  historyExpanded: _historyExpanded,
                  onHistoryToggle: () =>
                      setState(() => _historyExpanded = !_historyExpanded),
                  isSensitive: fields.any((f) => f.isSensitive),
                )
              : [
                  IconButton(
                    icon: const Icon(Icons.edit_outlined, size: 20),
                    tooltip: 'Edit',
                    onPressed: widget.onEdit,
                  ),
                  IconButton(
                    icon: const Icon(Icons.delete_outline, size: 20),
                    tooltip: 'Delete',
                    onPressed: widget.onDelete,
                  ),
                ],
        ),
        if (hasHistory && _historyExpanded)
          Padding(
            padding: const EdgeInsets.only(left: 32, bottom: 8),
            child: FieldHistoryView(
              fieldName: 'award',
              history: history,
            ),
          ),
      ],
    );
  }
}
