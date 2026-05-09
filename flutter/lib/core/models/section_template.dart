import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';

/// SectionTemplate - 分区模板模型
/// 用于快速创建具有预定义字段的分区
/// [nameKey] 和 [descriptionKey] 是 i18n key，会在 UI 层通过 AppLocalizations 解析
class SectionTemplate {
  final String id;
  final String nameKey;
  final String descriptionKey;
  final List<TemplateField> fields;
  final String icon;

  const SectionTemplate({
    required this.id,
    required this.nameKey,
    required this.descriptionKey,
    required this.fields,
    required this.icon,
  });
}

/// TemplateField - 模板字段定义
class TemplateField {
  final String key;
  final String type;
  final SensitivityLevel sensitivity;
  final String? config;

  const TemplateField({
    required this.key,
    required this.type,
    required this.sensitivity,
    this.config,
  });
}

/// 预设分区模板
class PresetSectionTemplates {
  PresetSectionTemplates._();

  static const List<SectionTemplate> templates = [
    // 中国银行账户模板
    SectionTemplate(
      id: 'china_bank_account',
      nameKey: 'templateChinaBankAccountName',
      descriptionKey: 'templateChinaBankAccountDesc',
      icon: '🏦',
      fields: [
        TemplateField(
          key: 'bank_name',
          type: 'text',
          sensitivity: SensitivityLevel.public,
        ),
        TemplateField(
          key: 'account_number',
          type: 'text',
          sensitivity: SensitivityLevel.critical,
        ),
        TemplateField(
          key: 'account_holder',
          type: 'text',
          sensitivity: SensitivityLevel.sensitive,
        ),
        TemplateField(
          key: 'branch_name',
          type: 'text',
          sensitivity: SensitivityLevel.public,
        ),
      ],
    ),
    // 英国银行账户模板
    SectionTemplate(
      id: 'uk_bank_account',
      nameKey: 'templateUkBankAccountName',
      descriptionKey: 'templateUkBankAccountDesc',
      icon: '🏛️',
      fields: [
        TemplateField(
          key: 'bank_name',
          type: 'text',
          sensitivity: SensitivityLevel.public,
        ),
        TemplateField(
          key: 'account_number',
          type: 'text',
          sensitivity: SensitivityLevel.critical,
        ),
        TemplateField(
          key: 'sort_code',
          type: 'text',
          sensitivity: SensitivityLevel.critical,
        ),
        TemplateField(
          key: 'iban',
          type: 'text',
          sensitivity: SensitivityLevel.critical,
        ),
        TemplateField(
          key: 'account_holder',
          type: 'text',
          sensitivity: SensitivityLevel.sensitive,
        ),
      ],
    ),
    // 美国银行账户模板
    SectionTemplate(
      id: 'us_bank_account',
      nameKey: 'templateUsBankAccountName',
      descriptionKey: 'templateUsBankAccountDesc',
      icon: '🗽',
      fields: [
        TemplateField(
          key: 'bank_name',
          type: 'text',
          sensitivity: SensitivityLevel.public,
        ),
        TemplateField(
          key: 'account_number',
          type: 'text',
          sensitivity: SensitivityLevel.critical,
        ),
        TemplateField(
          key: 'routing_number',
          type: 'text',
          sensitivity: SensitivityLevel.critical,
        ),
        TemplateField(
          key: 'account_type',
          type: 'select',
          sensitivity: SensitivityLevel.public,
          config: 'checking,savings',
        ),
        TemplateField(
          key: 'account_holder',
          type: 'text',
          sensitivity: SensitivityLevel.sensitive,
        ),
      ],
    ),
  ];
}
