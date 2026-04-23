import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_animate/flutter_animate.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart'
    show showOverlaySnackBar, SnackBarType;
import 'package:solosoul_flutter/presentation/providers/profile_provider.dart';
import 'package:solosoul_flutter/presentation/providers/account_style_provider.dart';
import 'package:solosoul_flutter/presentation/utils/list_utils.dart';
import 'package:solosoul_flutter/core/services/profile_storage_service.dart';
import 'package:solosoul_flutter/presentation/widgets/unified_form_section.dart'
    show UnifiedFormSection, FormFieldDef, HistoryRecordingConfig;
import 'package:solosoul_flutter/presentation/widgets/header_action_buttons.dart';
import 'package:solosoul_flutter/presentation/widgets/entry_card_widget.dart';
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

class _EducationSectionState extends ConsumerState<_EducationSection>
    with WidgetsBindingObserver {
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
      'institution': edu.institution ?? '',
      'degree': edu.degree ?? '',
      'degreeCustom': edu.degreeCustom ?? '',
      'field': edu.field ?? '',
      'startDate': edu.startDate ?? '',
      'endDate': edu.endDate ?? '',
    };
  }

  Future<void> _onDelete(EducationData item) async {
    final index = _items.indexById(item.id, (x) => x.id);
    if (index == -1) return;
    final isPrivacyMode =
        ref.read(accountStyleProvider).displayMode ==
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
      if (mounted) {
        setState(() {
          _items = List.from(_items)..insert(index, item);
        });
      }
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
          _loadData();
          if (mounted) setState(() {});
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
      itemToSave = _createFromValues(values, id: editingItem.id);
    }

    // Snapshot for rollback on failure
    final originalItems = List<EducationData>.from(_items);

    // Update local state optimistically
    if (wasAdding) {
      _items = List.from(_items)..add(itemToSave);
    } else {
      final index = _items.indexById(editingItem.id, (x) => x.id);
      if (index != -1) {
        _items = List.from(_items)..[index] = itemToSave;
      }
    }

    // Persist via provider with rollback on failure
    try {
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
    } catch (e) {
      // Rollback on failure
      _items = originalItems;
      if (mounted) {
        showOverlaySnackBar(context, content: 'Failed to save education: $e', type: SnackBarType.error);
      }
      return;
    }

    if (mounted) {
      final isPrivacyMode =
          ref.read(accountStyleProvider).displayMode ==
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
          (ctx, theme, controllers, mode, onSubmit, onCancel, sensitivities) {
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
                        initialValue: isCustomSelected
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
      displayItemBuilder: _buildEducationItem,
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
          allFieldValues: oldValues ?? {},
        );
      },
      showHistoryExpansion: true,
      historyFieldIdPrefix: 'education',
      itemIdExtractor: (e) => e.id,
    );
  }
}

Widget _buildEducationItem(EducationData item, Map<String, String> itemMap) {
    return EntryCardWidget<EducationData>(
      item: item,
      title: item.institution ?? 'Institution',
      icon: Icons.school,
      itemId: item.id,
      historyFieldId: 'education',
      formatAllFields: (e) => '${e.entryType}\n${e.toFormattedString()}',
      // Auto-build mode
      itemData: itemMap,
      fieldPrefix: 'education',
      excludeFields: const {'institution'},
    );
  }

// ============ Employment Section ============

class _EmploymentSection extends ConsumerStatefulWidget {
  @override
  ConsumerState<_EmploymentSection> createState() => _EmploymentSectionState();
}

class _EmploymentSectionState extends ConsumerState<_EmploymentSection>
    with WidgetsBindingObserver {
  late List<EmploymentData> _items;

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
      'company': emp.company ?? '',
      'position': emp.position ?? '',
      'responsibilities': emp.responsibilities ?? '',
      'startDate': emp.startDate ?? '',
      'endDate': emp.endDate ?? '',
    };
  }

  Future<void> _onDelete(EmploymentData item) async {
    final index = _items.indexById(item.id, (x) => x.id);
    if (index == -1) return;
    final isPrivacyMode =
        ref.read(accountStyleProvider).displayMode ==
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
      if (mounted) {
        setState(() {
          _items = List.from(_items)..insert(index, item);
        });
      }
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
          _loadData();
          if (mounted) setState(() {});
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
      itemToSave = _createFromValues(values, id: editingItem.id);
    }

    // Snapshot for rollback on failure
    final originalItems = List<EmploymentData>.from(_items);

    // Update local state optimistically
    if (wasAdding) {
      _items = List.from(_items)..add(itemToSave);
    } else {
      final index = _items.indexById(editingItem.id, (x) => x.id);
      if (index != -1) {
        _items = List.from(_items)..[index] = itemToSave;
      }
    }

    // Persist via provider with rollback on failure
    try {
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
    } catch (e) {
      // Rollback on failure
      _items = originalItems;
      if (mounted) {
        showOverlaySnackBar(context, content: 'Failed to save employment: $e', type: SnackBarType.error);
      }
      return;
    }

    if (mounted) {
      final isPrivacyMode =
          ref.read(accountStyleProvider).displayMode ==
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
      fieldDefs: [
        const FormFieldDef(
          fieldId: 'employment.company',
          label: 'Company',
          sensitivity: SensitivityLevel.public,
        ),
        const FormFieldDef(
          fieldId: 'employment.position',
          label: 'Position',
          sensitivity: SensitivityLevel.public,
        ),
        FormFieldDef(
          fieldId: 'employment.responsibilities',
          label: 'Responsibilities',
          sensitivity: ref.watch(effectiveSensitivityProvider('employment.responsibilities')),
        ),
        const FormFieldDef(
          fieldId: 'employment.startDate',
          label: 'Start Date',
          sensitivity: SensitivityLevel.public,
        ),
        const FormFieldDef(
          fieldId: 'employment.endDate',
          label: 'End Date',
          sensitivity: SensitivityLevel.public,
        ),
      ],
      displayItemBuilder: _buildEmploymentItem,
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
          allFieldValues: oldValues ?? {},
        );
      },
      showHistoryExpansion: true,
      historyFieldIdPrefix: 'employment',
      itemIdExtractor: (e) => e.id,
    );
  }
}

Widget _buildEmploymentItem(EmploymentData item, Map<String, String> itemMap) {
    return EntryCardWidget<EmploymentData>(
      item: item,
      title: item.company ?? 'Company',
      icon: Icons.work,
      itemId: item.id,
      historyFieldId: 'employment',
      formatAllFields: (e) => '${e.entryType}\n${e.toFormattedString()}',
      // Auto-build mode
      itemData: itemMap,
      fieldPrefix: 'employment',
      excludeFields: const {'company'},
    );
  }

// ============ Skills Section ============

class _SkillsSection extends ConsumerStatefulWidget {
  @override
  ConsumerState<_SkillsSection> createState() => _SkillsSectionState();
}

class _SkillsSectionState extends ConsumerState<_SkillsSection>
    with WidgetsBindingObserver {
  late List<SkillData> _items;

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
    return {'name': skill.name, 'level': skill.level ?? ''};
  }

  Future<void> _onDelete(SkillData item) async {
    final index = _items.indexById(item.id, (x) => x.id);
    if (index == -1) return;
    final isPrivacyMode =
        ref.read(accountStyleProvider).displayMode ==
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
      if (mounted) {
        setState(() {
          _items = List.from(_items)..insert(index, item);
        });
      }
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
          _loadData();
          if (mounted) setState(() {});
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
      itemToSave = _createFromValues(values, id: editingItem.id);
    }
    if (itemToSave.name.isEmpty) return;

    // Snapshot for rollback on failure
    final originalItems = List<SkillData>.from(_items);

    // Update local state optimistically
    if (wasAdding) {
      _items = List.from(_items)..add(itemToSave);
    } else {
      final index = _items.indexById(editingItem.id, (x) => x.id);
      if (index != -1) {
        _items = List.from(_items)..[index] = itemToSave;
      }
    }

    // Persist via provider with rollback on failure
    try {
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
    } catch (e) {
      // Rollback on failure
      _items = originalItems;
      if (mounted) {
        showOverlaySnackBar(context, content: 'Failed to save skill: $e', type: SnackBarType.error);
      }
      return;
    }

    if (mounted) {
      final isPrivacyMode =
          ref.read(accountStyleProvider).displayMode ==
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
      displayItemBuilder: _buildSkillItem,
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
          allFieldValues: oldValues ?? {},
        );
      },
      showHistoryExpansion: true,
      historyFieldIdPrefix: 'skill',
      itemIdExtractor: (s) => s.id,
    );
  }
}

Widget _buildSkillItem(SkillData item, Map<String, String> itemMap) {
    return EntryCardWidget<SkillData>(
      item: item,
      title: item.name,
      icon: Icons.star,
      itemId: item.id,
      historyFieldId: 'skill',
      formatAllFields: (e) => '${e.entryType}\n${e.toFormattedString()}',
      // Auto-build mode
      itemData: itemMap,
      fieldPrefix: 'skill',
      excludeFields: const {'name'},
    );
  }

// ============ Language Section ============

class _LanguageSection extends ConsumerStatefulWidget {
  @override
  ConsumerState<_LanguageSection> createState() => _LanguageSectionState();
}

class _LanguageSectionState extends ConsumerState<_LanguageSection>
    with WidgetsBindingObserver {
  late List<LanguageData> _items;

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
      'name': language.name,
      'proficiency': language.proficiency ?? '',
    };
  }

  Future<void> _onDelete(LanguageData item) async {
    final index = _items.indexById(item.id, (x) => x.id);
    if (index == -1) return;
    final isPrivacyMode =
        ref.read(accountStyleProvider).displayMode ==
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
      if (mounted) {
        setState(() {
          _items = List.from(_items)..insert(index, item);
        });
      }
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
          _loadData();
          if (mounted) setState(() {});
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
      itemToSave = _createFromValues(values, id: editingItem.id);
    }
    if (itemToSave.name.isEmpty) return;

    // Snapshot for rollback on failure
    final originalItems = List<LanguageData>.from(_items);

    // Update local state optimistically
    if (wasAdding) {
      _items = List.from(_items)..add(itemToSave);
    } else {
      final index = _items.indexById(editingItem.id, (x) => x.id);
      if (index != -1) {
        _items = List.from(_items)..[index] = itemToSave;
      }
    }

    // Persist via provider with rollback on failure
    try {
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
    } catch (e) {
      // Rollback on failure
      _items = originalItems;
      if (mounted) {
        showOverlaySnackBar(context, content: 'Failed to save language: $e', type: SnackBarType.error);
      }
      return;
    }

    if (mounted) {
      final isPrivacyMode =
          ref.read(accountStyleProvider).displayMode ==
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
      displayItemBuilder: _buildLanguageItem,
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
          allFieldValues: oldValues ?? {},
        );
      },
      showHistoryExpansion: true,
      historyFieldIdPrefix: 'language',
      itemIdExtractor: (l) => l.id,
    );
  }
}

Widget _buildLanguageItem(LanguageData item, Map<String, String> itemMap) {
    return EntryCardWidget<LanguageData>(
      item: item,
      title: item.name,
      icon: Icons.translate,
      itemId: item.id,
      historyFieldId: 'language',
      formatAllFields: (e) => '${e.entryType}\n${e.toFormattedString()}',
      // Auto-build mode
      itemData: itemMap,
      fieldPrefix: 'language',
      excludeFields: const {'name'},
    );
  }

// ============ Award Section ============

class _AwardSection extends ConsumerStatefulWidget {
  @override
  ConsumerState<_AwardSection> createState() => _AwardSectionState();
}

class _AwardSectionState extends ConsumerState<_AwardSection>
    with WidgetsBindingObserver {
  late List<AwardData> _items;

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
      'title': award.title ?? '',
      'issuer': award.issuer ?? '',
      'date': award.date ?? '',
      'description': award.description ?? '',
    };
  }

  Future<void> _onDelete(AwardData item) async {
    final index = _items.indexById(item.id, (x) => x.id);
    if (index == -1) return;
    final isPrivacyMode =
        ref.read(accountStyleProvider).displayMode ==
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
      if (mounted) {
        setState(() {
          _items = List.from(_items)..insert(index, item);
        });
      }
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
          _loadData();
          if (mounted) setState(() {});
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
      itemToSave = _createFromValues(values, id: editingItem.id);
    }
    if (itemToSave.title == null || itemToSave.title!.isEmpty) return;

    // Snapshot for rollback on failure
    final originalItems = List<AwardData>.from(_items);

    // Update local state optimistically
    if (wasAdding) {
      _items = List.from(_items)..add(itemToSave);
    } else {
      final index = _items.indexById(editingItem.id, (x) => x.id);
      if (index != -1) {
        _items = List.from(_items)..[index] = itemToSave;
      }
    }

    // Persist via provider with rollback on failure
    try {
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
    } catch (e) {
      // Rollback on failure
      _items = originalItems;
      if (mounted) {
        showOverlaySnackBar(context, content: 'Failed to save award: $e', type: SnackBarType.error);
      }
      return;
    }

    if (mounted) {
      final isPrivacyMode =
          ref.read(accountStyleProvider).displayMode ==
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
      fieldDefs: [
        const FormFieldDef(
          fieldId: 'award.title',
          label: 'Title',
          sensitivity: SensitivityLevel.public,
        ),
        const FormFieldDef(
          fieldId: 'award.issuer',
          label: 'Issuer',
          sensitivity: SensitivityLevel.public,
        ),
        const FormFieldDef(
          fieldId: 'award.date',
          label: 'Date',
          sensitivity: SensitivityLevel.public,
        ),
        FormFieldDef(
          fieldId: 'award.description',
          label: 'Description',
          sensitivity: ref.watch(effectiveSensitivityProvider('award.description')),
        ),
      ],
      displayItemBuilder: _buildAwardItem,
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
          allFieldValues: oldValues ?? {},
        );
      },
      showHistoryExpansion: true,
      historyFieldIdPrefix: 'award',
      itemIdExtractor: (a) => a.id,
    );
  }
}

Widget _buildAwardItem(AwardData item, Map<String, String> itemMap) {
    return EntryCardWidget<AwardData>(
      item: item,
      title: item.title ?? 'Award',
      icon: Icons.emoji_events,
      itemId: item.id,
      historyFieldId: 'award',
      formatAllFields: (e) => '${e.entryType}\n${e.toFormattedString()}',
      // Auto-build mode
      itemData: itemMap,
      fieldPrefix: 'award',
      excludeFields: const {'title'},
    );
  }
