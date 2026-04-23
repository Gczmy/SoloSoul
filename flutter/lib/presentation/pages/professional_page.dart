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

class _EducationSectionState extends ConsumerState<_EducationSection> {
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
    final items = ref.read(educationItemsProvider);
    final index = items.indexById(item.id, (x) => x.id);
    if (index == -1) return;
    final isPrivacyMode =
        ref.read(accountStyleProvider).displayMode ==
        SensitivityDisplayMode.hidePrivate;
    final deletedId = item.id;

    try {
      await ref
          .read(profileNotifierProvider.notifier)
          .softDelete(
            section: 'professional',
            itemType: 'education',
            index: index,
            deletedItem: item,
          );
    } on Exception {
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
      itemToSave = _createFromValues(values, id: editingItem.id);
    }

    // Persist via provider
    try {
      final professional = ProfessionalData(
        education: ref.read(educationItemsProvider),
        employment: ref.read(employmentItemsProvider),
        skills: ref.read(skillItemsProvider),
        languages: ref.read(languageItemsProvider),
      );
      await ref
          .read(profileNotifierProvider.notifier)
          .updateProfessionalImmediate(professional);
    } on Exception catch (e) {
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
    final items = ref.watch(educationItemsProvider);
    return UnifiedFormSection<EducationData>(
      title: 'Education',
      icon: Icons.school_outlined,
      items: items,
      maxVisibleItems: 3,
      itemFactory: _createFromValues,
      fieldDefs: [
        FormFieldDef(
          fieldId: 'education.institution',
          label: 'Institution',
          sensitivity: ref.watch(effectiveSensitivityProvider('education.institution')),
        ),
        FormFieldDef(
          fieldId: 'education.degree',
          label: 'Degree',
          sensitivity: ref.watch(effectiveSensitivityProvider('education.degree')),
        ),
        FormFieldDef(
          fieldId: 'education.degreeCustom',
          label: 'Custom Degree',
          sensitivity: ref.watch(effectiveSensitivityProvider('education.degreeCustom')),
        ),
        FormFieldDef(
          fieldId: 'education.fieldOfStudy',
          label: 'Field of Study',
          sensitivity: ref.watch(effectiveSensitivityProvider('education.fieldOfStudy')),
        ),
        FormFieldDef(
          fieldId: 'education.startDate',
          label: 'Start Date',
          sensitivity: ref.watch(effectiveSensitivityProvider('education.startDate')),
        ),
        FormFieldDef(
          fieldId: 'education.endDate',
          label: 'End Date',
          sensitivity: ref.watch(effectiveSensitivityProvider('education.endDate')),
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

class _EmploymentSectionState extends ConsumerState<_EmploymentSection> {
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
    final items = ref.read(employmentItemsProvider);
    final index = items.indexById(item.id, (x) => x.id);
    if (index == -1) return;
    final isPrivacyMode =
        ref.read(accountStyleProvider).displayMode ==
        SensitivityDisplayMode.hidePrivate;
    final deletedId = item.id;

    try {
      await ref
          .read(profileNotifierProvider.notifier)
          .softDelete(
            section: 'professional',
            itemType: 'employment',
            index: index,
            deletedItem: item,
          );
    } on Exception {
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
      itemToSave = _createFromValues(values, id: editingItem.id);
    }

    // Persist via provider
    try {
      final professional = ProfessionalData(
        education: ref.read(educationItemsProvider),
        employment: ref.read(employmentItemsProvider),
        skills: ref.read(skillItemsProvider),
        languages: ref.read(languageItemsProvider),
        awards: ref.read(awardItemsProvider),
      );
      await ref
          .read(profileNotifierProvider.notifier)
          .updateProfessionalImmediate(professional);
    } on Exception catch (e) {
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
    final items = ref.watch(employmentItemsProvider);
    return UnifiedFormSection<EmploymentData>(
      title: 'Employment',
      icon: Icons.work_outlined,
      items: items,
      maxVisibleItems: 3,
      itemFactory: _createFromValues,
      fieldDefs: [
        FormFieldDef(
          fieldId: 'employment.company',
          label: 'Company',
          sensitivity: ref.watch(effectiveSensitivityProvider('employment.company')),
        ),
        FormFieldDef(
          fieldId: 'employment.position',
          label: 'Position',
          sensitivity: ref.watch(effectiveSensitivityProvider('employment.position')),
        ),
        FormFieldDef(
          fieldId: 'employment.responsibilities',
          label: 'Responsibilities',
          sensitivity: ref.watch(effectiveSensitivityProvider('employment.responsibilities')),
        ),
        FormFieldDef(
          fieldId: 'employment.startDate',
          label: 'Start Date',
          sensitivity: ref.watch(effectiveSensitivityProvider('employment.startDate')),
        ),
        FormFieldDef(
          fieldId: 'employment.endDate',
          label: 'End Date',
          sensitivity: ref.watch(effectiveSensitivityProvider('employment.endDate')),
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

class _SkillsSectionState extends ConsumerState<_SkillsSection> {
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
    final items = ref.watch(skillItemsProvider);
    final index = items.indexById(item.id, (x) => x.id);
    if (index == -1) return;
    final isPrivacyMode =
        ref.read(accountStyleProvider).displayMode ==
        SensitivityDisplayMode.hidePrivate;
    final deletedId = item.id;

    try {
      await ref
          .read(profileNotifierProvider.notifier)
          .softDelete(
            section: 'professional',
            itemType: 'skill',
            index: index,
            deletedItem: item,
          );
    } on Exception {
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
      itemToSave = _createFromValues(values, id: editingItem.id);
    }
    if (itemToSave.name.isEmpty) return;

    // Persist via provider - UI derives state from skillItemsProvider
    try {
      final professional = ProfessionalData(
        education: ref.read(educationItemsProvider),
        employment: ref.read(employmentItemsProvider),
        skills: [...ref.read(skillItemsProvider)],
        languages: ref.read(languageItemsProvider),
      );
      await ref
          .read(profileNotifierProvider.notifier)
          .updateProfessionalImmediate(professional);
    } on Exception catch (e) {
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
    final items = ref.watch(skillItemsProvider);
    return UnifiedFormSection<SkillData>(
      title: 'Skills',
      icon: Icons.star_outline,
      items: items,
      maxVisibleItems: 3,
      itemFactory: _createFromValues,
      fieldDefs: [
        FormFieldDef(
          fieldId: 'skill.name',
          label: 'Skill Name',
          sensitivity: ref.watch(effectiveSensitivityProvider('skill.name')),
        ),
        FormFieldDef(
          fieldId: 'skill.level',
          label: 'Level (e.g. Beginner, Intermediate, Expert)',
          sensitivity: ref.watch(effectiveSensitivityProvider('skill.level')),
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

class _LanguageSectionState extends ConsumerState<_LanguageSection> {
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
    final items = ref.read(languageItemsProvider);
    final index = items.indexById(item.id, (x) => x.id);
    if (index == -1) return;
    final isPrivacyMode =
        ref.read(accountStyleProvider).displayMode ==
        SensitivityDisplayMode.hidePrivate;
    final deletedId = item.id;

    try {
      await ref
          .read(profileNotifierProvider.notifier)
          .softDelete(
            section: 'professional',
            itemType: 'language',
            index: index,
            deletedItem: item,
          );
    } on Exception {
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
      itemToSave = _createFromValues(values, id: editingItem.id);
    }
    if (itemToSave.name.isEmpty) return;

    // Persist via provider
    try {
      final professional = ProfessionalData(
        education: ref.read(educationItemsProvider),
        employment: ref.read(employmentItemsProvider),
        skills: ref.read(skillItemsProvider),
        languages: ref.read(languageItemsProvider),
      );
      await ref
          .read(profileNotifierProvider.notifier)
          .updateProfessionalImmediate(professional);
    } on Exception catch (e) {
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
    final items = ref.watch(languageItemsProvider);
    return UnifiedFormSection<LanguageData>(
      title: 'Languages',
      icon: Icons.translate,
      items: items,
      maxVisibleItems: 3,
      itemFactory: _createFromValues,
      fieldDefs: [
        FormFieldDef(
          fieldId: 'language.name',
          label: 'Language',
          sensitivity: ref.watch(effectiveSensitivityProvider('language.name')),
        ),
        FormFieldDef(
          fieldId: 'language.proficiency',
          label: 'Proficiency (e.g. Native, Fluent, Intermediate)',
          sensitivity: ref.watch(effectiveSensitivityProvider('language.proficiency')),
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

class _AwardSectionState extends ConsumerState<_AwardSection> {
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
    final items = ref.read(awardItemsProvider);
    final index = items.indexById(item.id, (x) => x.id);
    if (index == -1) return;
    final isPrivacyMode =
        ref.read(accountStyleProvider).displayMode ==
        SensitivityDisplayMode.hidePrivate;
    final deletedId = item.id;

    try {
      await ref
          .read(profileNotifierProvider.notifier)
          .softDelete(
            section: 'professional',
            itemType: 'award',
            index: index,
            deletedItem: item,
          );
    } on Exception {
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
      itemToSave = _createFromValues(values, id: editingItem.id);
    }
    if (itemToSave.title == null || itemToSave.title!.isEmpty) return;

    // Persist via provider
    try {
      final professional = ProfessionalData(
        education: ref.read(educationItemsProvider),
        employment: ref.read(employmentItemsProvider),
        skills: ref.read(skillItemsProvider),
        languages: ref.read(languageItemsProvider),
        awards: ref.read(awardItemsProvider),
      );
      await ref
          .read(profileNotifierProvider.notifier)
          .updateProfessionalImmediate(professional);
    } on Exception catch (e) {
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
    final items = ref.watch(awardItemsProvider);
    return UnifiedFormSection<AwardData>(
      title: 'Awards',
      icon: Icons.emoji_events_outlined,
      items: items,
      maxVisibleItems: 3,
      itemFactory: _createFromValues,
      fieldDefs: [
        FormFieldDef(
          fieldId: 'award.title',
          label: 'Title',
          sensitivity: ref.watch(effectiveSensitivityProvider('award.title')),
        ),
        FormFieldDef(
          fieldId: 'award.issuer',
          label: 'Issuer',
          sensitivity: ref.watch(effectiveSensitivityProvider('award.issuer')),
        ),
        FormFieldDef(
          fieldId: 'award.date',
          label: 'Date',
          sensitivity: ref.watch(effectiveSensitivityProvider('award.date')),
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
