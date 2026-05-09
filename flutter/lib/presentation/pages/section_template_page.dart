import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/core/models/section_template.dart';
import 'package:solosoul_flutter/presentation/widgets/sensitivity_tag.dart';
import 'package:solosoul_flutter/presentation/theme/glass_adapters.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart' show AppTheme;
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/utils/format_field_label.dart';

/// Standalone page for browsing and selecting a section template.
/// Returns the selected [SectionTemplate] via [Navigator.pop] when applied.
class SectionTemplatePage extends ConsumerStatefulWidget {
  const SectionTemplatePage({super.key});

  @override
  ConsumerState<SectionTemplatePage> createState() => _SectionTemplatePageState();
}

class _SectionTemplatePageState extends ConsumerState<SectionTemplatePage> {
  String? _selectedTemplateId;
  final Set<String> _expandedTemplates = {};

  List<SectionTemplate> get _templates => PresetSectionTemplates.templates;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final templates = _templates;

    return Scaffold(
      appBar: SoloGlassAppBar(
        title: Text(AppLocalizations.of(context).sectionTemplateTitle),
      ),
      body: Stack(
        children: [
          SingleChildScrollView(
            padding: AppTheme.kPagePadding.copyWith(bottom: 96),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                // Template list
                if (templates.isEmpty)
                  _buildEmptyState(theme)
                else
                  ...templates.map((template) => _TemplateCard(
                        template: template,
                        isExpanded: _expandedTemplates.contains(template.id),
                        isSelected: _selectedTemplateId == template.id,
                        onToggleExpand: () {
                          setState(() {
                            if (_expandedTemplates.contains(template.id)) {
                              _expandedTemplates.remove(template.id);
                            } else {
                              _expandedTemplates.add(template.id);
                            }
                          });
                        },
                        onSelect: () {
                          setState(() {
                            _selectedTemplateId =
                                _selectedTemplateId == template.id
                                    ? null
                                    : template.id;
                          });
                        },
                      )),
              ],
            ),
          ),
          _BottomApplyBar(
            selectedTemplateId: _selectedTemplateId,
            onApply: _onApplyTemplate,
          ),
        ],
      ),
    );
  }

  Widget _buildEmptyState(ThemeData theme) {
    return Container(
      width: double.infinity,
      padding: const EdgeInsets.all(24),
      decoration: BoxDecoration(
        color: theme.colorScheme.surface.withValues(alpha: 0.5),
        borderRadius: BorderRadius.circular(8),
        border: Border.all(
          color: theme.colorScheme.outline.withValues(alpha: 0.2),
        ),
      ),
      child: Column(
        children: [
          Icon(
            Icons.folder_copy_outlined,
            size: 32,
            color: theme.colorScheme.onSurfaceVariant.withValues(alpha: 0.5),
          ),
          const SizedBox(height: 8),
          Text(
            AppLocalizations.of(context).sectionTemplateEmpty,
            style: theme.textTheme.bodyMedium?.copyWith(
              color: theme.colorScheme.onSurfaceVariant,
            ),
          ),
          const SizedBox(height: 4),
          Text(
            AppLocalizations.of(context).sectionTemplateEmptyHint,
            style: theme.textTheme.bodySmall?.copyWith(
              color: theme.colorScheme.onSurfaceVariant.withValues(alpha: 0.7),
            ),
          ),
        ],
      ),
    );
  }

  void _onApplyTemplate() {
    if (_selectedTemplateId == null) return;
    final template = _templates.firstWhere(
      (t) => t.id == _selectedTemplateId,
    );
    Navigator.of(context).pop(template);
  }
}

/// Fixed bottom apply bar for template selection.
class _BottomApplyBar extends StatelessWidget {
  final String? selectedTemplateId;
  final VoidCallback onApply;

  const _BottomApplyBar({
    required this.selectedTemplateId,
    required this.onApply,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Positioned(
      left: 0,
      right: 0,
      bottom: 0,
      child: Container(
        color: theme.scaffoldBackgroundColor,
        padding: AppTheme.kPagePadding,
        child: SizedBox(
          width: double.infinity,
          child: OutlinedButton.icon(
            onPressed: selectedTemplateId == null ? null : onApply,
            icon: const Icon(Icons.check_circle_outline, size: 18),
            label: Text(
              AppLocalizations.of(context).sectionTemplateSelectButton,
            ),
            style: OutlinedButton.styleFrom(
              padding: const EdgeInsets.symmetric(vertical: 14),
              side: selectedTemplateId == null
                  ? BorderSide(color: theme.colorScheme.outline.withValues(alpha: 0.3))
                  : BorderSide(color: theme.colorScheme.primary),
            ),
          ),
        ),
      ),
    );
  }
}

/// Single template card with expand/collapse and radio selection.
class _TemplateCard extends ConsumerWidget {
  const _TemplateCard({
    required this.template,
    required this.isExpanded,
    required this.isSelected,
    required this.onToggleExpand,
    required this.onSelect,
  });

  final SectionTemplate template;
  final bool isExpanded;
  final bool isSelected;
  final VoidCallback onToggleExpand;
  final VoidCallback onSelect;

  String _getTemplateName(SectionTemplate t, AppLocalizations l) {
    switch (t.nameKey) {
      case 'templateChinaBankAccountName':
        return l.templateChinaBankAccountName;
      case 'templateUkBankAccountName':
        return l.templateUkBankAccountName;
      case 'templateUsBankAccountName':
        return l.templateUsBankAccountName;
      case 'templateProfileIdentityName':
        return l.templateProfileIdentityName;
      case 'templateProfileContactName':
        return l.templateProfileContactName;
      case 'templateProfileIdCardName':
        return l.templateProfileIdCardName;
      case 'templateProfileAddressName':
        return l.templateProfileAddressName;
      case 'templateTravelPassportName':
        return l.templateTravelPassportName;
      case 'templateTravelVisaName':
        return l.templateTravelVisaName;
      case 'templateTravelHistoryName':
        return l.templateTravelHistoryName;
      case 'templateFinancialBankAccountName':
        return l.templateFinancialBankAccountName;
      case 'templateFinancialCardName':
        return l.templateFinancialCardName;
      case 'templateFinancialTaxIdName':
        return l.templateFinancialTaxIdName;
      case 'templateProfessionalEducationName':
        return l.templateProfessionalEducationName;
      case 'templateProfessionalEmploymentName':
        return l.templateProfessionalEmploymentName;
      case 'templateProfessionalSkillName':
        return l.templateProfessionalSkillName;
      case 'templateProfessionalLanguageName':
        return l.templateProfessionalLanguageName;
      case 'templateProfessionalAwardName':
        return l.templateProfessionalAwardName;
      default:
        return t.nameKey;
    }
  }

  String _getTemplateDescription(SectionTemplate t, AppLocalizations l) {
    switch (t.descriptionKey) {
      case 'templateChinaBankAccountDesc':
        return l.templateChinaBankAccountDesc;
      case 'templateUkBankAccountDesc':
        return l.templateUkBankAccountDesc;
      case 'templateUsBankAccountDesc':
        return l.templateUsBankAccountDesc;
      case 'templateProfileIdentityDesc':
        return l.templateProfileIdentityDesc;
      case 'templateProfileContactDesc':
        return l.templateProfileContactDesc;
      case 'templateProfileIdCardDesc':
        return l.templateProfileIdCardDesc;
      case 'templateProfileAddressDesc':
        return l.templateProfileAddressDesc;
      case 'templateTravelPassportDesc':
        return l.templateTravelPassportDesc;
      case 'templateTravelVisaDesc':
        return l.templateTravelVisaDesc;
      case 'templateTravelHistoryDesc':
        return l.templateTravelHistoryDesc;
      case 'templateFinancialBankAccountDesc':
        return l.templateFinancialBankAccountDesc;
      case 'templateFinancialCardDesc':
        return l.templateFinancialCardDesc;
      case 'templateFinancialTaxIdDesc':
        return l.templateFinancialTaxIdDesc;
      case 'templateProfessionalEducationDesc':
        return l.templateProfessionalEducationDesc;
      case 'templateProfessionalEmploymentDesc':
        return l.templateProfessionalEmploymentDesc;
      case 'templateProfessionalSkillDesc':
        return l.templateProfessionalSkillDesc;
      case 'templateProfessionalLanguageDesc':
        return l.templateProfessionalLanguageDesc;
      case 'templateProfessionalAwardDesc':
        return l.templateProfessionalAwardDesc;
      default:
        return t.descriptionKey;
    }
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);
    final l = AppLocalizations.of(context);

    return Container(
      margin: const EdgeInsets.only(bottom: 8),
      decoration: BoxDecoration(
        color: isSelected
            ? theme.colorScheme.primary.withValues(alpha: 0.05)
            : theme.colorScheme.surface,
        borderRadius: BorderRadius.circular(8),
        border: Border.all(
          color: isSelected
              ? theme.colorScheme.primary.withValues(alpha: 0.4)
              : theme.colorScheme.outline.withValues(alpha: 0.2),
          width: isSelected ? 1.5 : 1,
        ),
      ),
      child: Column(
        children: [
          // Header row
          InkWell(
            onTap: onToggleExpand,
            borderRadius: BorderRadius.circular(8),
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
              child: Row(
                children: [
                  // Expand/collapse icon
                  AnimatedRotation(
                    turns: isExpanded ? 0.25 : 0,
                    duration: const Duration(milliseconds: 200),
                    child: Icon(
                      Icons.chevron_right,
                      size: 20,
                      color: theme.colorScheme.onSurfaceVariant,
                    ),
                  ),
                  const SizedBox(width: 8),

                  // Template icon
                  Container(
                    width: 32,
                    height: 32,
                    decoration: BoxDecoration(
                      color: theme.colorScheme.primary.withValues(alpha: 0.1),
                      borderRadius: BorderRadius.circular(6),
                    ),
                    child: Icon(
                      Icons.description_outlined,
                      size: 16,
                      color: theme.colorScheme.primary,
                    ),
                  ),
                  const SizedBox(width: 12),

                  // Template info
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
                          _getTemplateName(template, l),
                          style: theme.textTheme.bodyMedium?.copyWith(
                            fontWeight: FontWeight.w600,
                            color: isSelected
                                ? theme.colorScheme.primary
                                : theme.colorScheme.onSurface,
                          ),
                        ),
                        Text(
                          _getTemplateDescription(template, l),
                          style: theme.textTheme.bodySmall?.copyWith(
                            color: theme.colorScheme.onSurfaceVariant,
                          ),
                        ),
                      ],
                    ),
                  ),

                  // Field count badge
                  Container(
                    padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                    decoration: BoxDecoration(
                      color: theme.colorScheme.surfaceContainerHighest,
                      borderRadius: BorderRadius.circular(4),
                    ),
                    child: Text(
                      l.sectionTemplateFieldCount(template.fields.length),
                      style: theme.textTheme.bodySmall?.copyWith(
                        color: theme.colorScheme.onSurfaceVariant,
                        fontSize: 10,
                      ),
                    ),
                  ),
                  const SizedBox(width: 12),

                  // Radio button for selection
                  Radio<String>(
                    value: template.id,
                    groupValue: isSelected ? template.id : null,
                    onChanged: (_) => onSelect(),
                    activeColor: theme.colorScheme.primary,
                  ),
                ],
              ),
            ),
          ),

          // Expanded field list
          AnimatedCrossFade(
            firstChild: const SizedBox.shrink(),
            secondChild: _buildExpandedContent(theme),
            crossFadeState:
                isExpanded ? CrossFadeState.showSecond : CrossFadeState.showFirst,
            duration: const Duration(milliseconds: 200),
          ),
        ],
      ),
    );
  }

  Widget _buildExpandedContent(ThemeData theme) {
    return Container(
      width: double.infinity,
      padding: const EdgeInsets.fromLTRB(12, 0, 12, 12),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Divider(
            height: 1,
            color: theme.colorScheme.outline.withValues(alpha: 0.15),
          ),
          const SizedBox(height: 8),
          ...template.fields.map((field) => _TemplateFieldRow(field: field)),
        ],
      ),
    );
  }
}

/// Single field row inside expanded template card.
class _TemplateFieldRow extends ConsumerWidget {
  const _TemplateFieldRow({required this.field});

  final TemplateField field;

  String _getFieldKeyLabel(String key, AppLocalizations l) {
    return translateFieldLabel(key, l);
  }

  String _getFieldTypeLabel(String type, AppLocalizations l) {
    switch (type) {
      case 'text':
        return l.objectEditorPropertyTypeText;
      case 'date':
        return l.objectEditorPropertyTypeDate;
      case 'number':
        return l.objectEditorPropertyTypeNumber;
      case 'checkbox':
        return l.objectEditorPropertyTypeCheckbox;
      case 'select':
        return l.objectEditorPropertyTypeSelect;
      case 'multiSelect':
        return l.objectEditorPropertyTypeMultiSelect;
      default:
        return type;
    }
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);
    final l = AppLocalizations.of(context);

    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 4),
      child: Row(
        children: [
          // Field type icon
          Container(
            width: 24,
            height: 24,
            decoration: BoxDecoration(
              color: theme.colorScheme.surfaceContainerHighest,
              borderRadius: BorderRadius.circular(4),
            ),
            child: Icon(
              _getFieldTypeIcon(field.type),
              size: 12,
              color: theme.colorScheme.onSurfaceVariant,
            ),
          ),
          const SizedBox(width: 8),

          // Field name
          Expanded(
            child: Text(
              _getFieldKeyLabel(field.key, l),
              style: theme.textTheme.bodySmall?.copyWith(
                fontWeight: FontWeight.w500,
              ),
            ),
          ),

          // Field type label
          Container(
            padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
            decoration: BoxDecoration(
              color: theme.colorScheme.surfaceContainerHighest,
              borderRadius: BorderRadius.circular(4),
            ),
            child: Text(
              _getFieldTypeLabel(field.type, l),
              style: theme.textTheme.bodySmall?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
                fontSize: 10,
              ),
            ),
          ),
          const SizedBox(width: 8),

          // Sensitivity tag
          SensitivityTag(level: field.sensitivity),
        ],
      ),
    );
  }

  IconData _getFieldTypeIcon(String type) {
    switch (type) {
      case 'text':
        return Icons.text_fields;
      case 'number':
        return Icons.numbers;
      case 'date':
        return Icons.calendar_today;
      case 'checkbox':
        return Icons.check_box_outlined;
      case 'select':
        return Icons.arrow_drop_down_circle_outlined;
      case 'multiSelect':
        return Icons.list;
      default:
        return Icons.text_fields;
    }
  }
}
