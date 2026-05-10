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
  /// 所属页面分类：profile, financial, professional, travel, bank
  final String pageTag;

  const SectionTemplate({
    required this.id,
    required this.nameKey,
    required this.descriptionKey,
    required this.fields,
    required this.icon,
    required this.pageTag,
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
      pageTag: 'bank',
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
      pageTag: 'bank',
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
      pageTag: 'bank',
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
    // 身份信息模板
    SectionTemplate(
      id: 'profile_identity',
      nameKey: 'templateProfileIdentityName',
      descriptionKey: 'templateProfileIdentityDesc',
      icon: '👤',
      pageTag: 'profile',
      fields: [
        TemplateField(
          key: 'full_name',
          type: 'text',
          sensitivity: SensitivityLevel.public,
        ),
        TemplateField(
          key: 'given_name',
          type: 'text',
          sensitivity: SensitivityLevel.public,
        ),
        TemplateField(
          key: 'family_name',
          type: 'text',
          sensitivity: SensitivityLevel.public,
        ),
        TemplateField(
          key: 'date_of_birth',
          type: 'date',
          sensitivity: SensitivityLevel.sensitive,
        ),
        TemplateField(
          key: 'gender',
          type: 'text',
          sensitivity: SensitivityLevel.public,
        ),
        TemplateField(
          key: 'nationality',
          type: 'text',
          sensitivity: SensitivityLevel.sensitive,
        ),
      ],
    ),
    // 联系信息模板
    SectionTemplate(
      id: 'profile_contact',
      nameKey: 'templateProfileContactName',
      descriptionKey: 'templateProfileContactDesc',
      icon: '📞',
      pageTag: 'profile',
      fields: [
        TemplateField(
          key: 'type',
          type: 'text',
          sensitivity: SensitivityLevel.public,
        ),
        TemplateField(
          key: 'value',
          type: 'text',
          sensitivity: SensitivityLevel.internal,
        ),
      ],
    ),
    // 身份证件模板
    SectionTemplate(
      id: 'profile_id_card',
      nameKey: 'templateProfileIdCardName',
      descriptionKey: 'templateProfileIdCardDesc',
      icon: '🪪',
      pageTag: 'profile',
      fields: [
        TemplateField(
          key: 'number',
          type: 'text',
          sensitivity: SensitivityLevel.critical,
        ),
        TemplateField(
          key: 'issue_date',
          type: 'date',
          sensitivity: SensitivityLevel.internal,
        ),
        TemplateField(
          key: 'expiry_date',
          type: 'date',
          sensitivity: SensitivityLevel.internal,
        ),
        TemplateField(
          key: 'holder_name',
          type: 'text',
          sensitivity: SensitivityLevel.sensitive,
        ),
        TemplateField(
          key: 'country',
          type: 'text',
          sensitivity: SensitivityLevel.public,
        ),
      ],
    ),
    // 地址模板
    SectionTemplate(
      id: 'profile_address',
      nameKey: 'templateProfileAddressName',
      descriptionKey: 'templateProfileAddressDesc',
      icon: '🏠',
      pageTag: 'profile',
      fields: [
        TemplateField(
          key: 'street',
          type: 'text',
          sensitivity: SensitivityLevel.sensitive,
        ),
        TemplateField(
          key: 'city',
          type: 'text',
          sensitivity: SensitivityLevel.sensitive,
        ),
        TemplateField(
          key: 'state',
          type: 'text',
          sensitivity: SensitivityLevel.sensitive,
        ),
        TemplateField(
          key: 'postal_code',
          type: 'text',
          sensitivity: SensitivityLevel.sensitive,
        ),
        TemplateField(
          key: 'country',
          type: 'text',
          sensitivity: SensitivityLevel.public,
        ),
      ],
    ),
    // 银行账户模板
    SectionTemplate(
      id: 'financial_bank_account',
      nameKey: 'templateFinancialBankAccountName',
      descriptionKey: 'templateFinancialBankAccountDesc',
      icon: '🏦',
      pageTag: 'financial',
      fields: [
        TemplateField(
          key: 'bank_name',
          type: 'text',
          sensitivity: SensitivityLevel.sensitive,
        ),
        TemplateField(
          key: 'account_number',
          type: 'text',
          sensitivity: SensitivityLevel.critical,
        ),
        TemplateField(
          key: 'currency',
          type: 'text',
          sensitivity: SensitivityLevel.public,
        ),
        TemplateField(
          key: 'swift_bic',
          type: 'text',
          sensitivity: SensitivityLevel.critical,
        ),
        TemplateField(
          key: 'sort_code',
          type: 'text',
          sensitivity: SensitivityLevel.critical,
        ),
      ],
    ),
    // 卡片模板
    SectionTemplate(
      id: 'financial_card',
      nameKey: 'templateFinancialCardName',
      descriptionKey: 'templateFinancialCardDesc',
      icon: '💳',
      pageTag: 'financial',
      fields: [
        TemplateField(
          key: 'card_number',
          type: 'text',
          sensitivity: SensitivityLevel.critical,
        ),
        TemplateField(
          key: 'card_type',
          type: 'text',
          sensitivity: SensitivityLevel.public,
        ),
        TemplateField(
          key: 'expiry_date',
          type: 'date',
          sensitivity: SensitivityLevel.sensitive,
        ),
        TemplateField(
          key: 'holder_name',
          type: 'text',
          sensitivity: SensitivityLevel.sensitive,
        ),
        TemplateField(
          key: 'cvv',
          type: 'text',
          sensitivity: SensitivityLevel.critical,
        ),
      ],
    ),
    // 税务识别号模板
    SectionTemplate(
      id: 'financial_tax_id',
      nameKey: 'templateFinancialTaxIdName',
      descriptionKey: 'templateFinancialTaxIdDesc',
      icon: '🔢',
      pageTag: 'financial',
      fields: [
        TemplateField(
          key: 'tax_id_number',
          type: 'text',
          sensitivity: SensitivityLevel.critical,
        ),
        TemplateField(
          key: 'tax_id_type',
          type: 'text',
          sensitivity: SensitivityLevel.sensitive,
        ),
        TemplateField(
          key: 'issuing_authority',
          type: 'text',
          sensitivity: SensitivityLevel.sensitive,
        ),
        TemplateField(
          key: 'country',
          type: 'text',
          sensitivity: SensitivityLevel.public,
        ),
      ],
    ),
    // 教育模板
    SectionTemplate(
      id: 'professional_education',
      nameKey: 'templateProfessionalEducationName',
      descriptionKey: 'templateProfessionalEducationDesc',
      icon: '🎓',
      pageTag: 'professional',
      fields: [
        TemplateField(
          key: 'institution',
          type: 'text',
          sensitivity: SensitivityLevel.public,
        ),
        TemplateField(
          key: 'degree',
          type: 'text',
          sensitivity: SensitivityLevel.public,
        ),
        TemplateField(
          key: 'degreeCustom',
          type: 'text',
          sensitivity: SensitivityLevel.public,
        ),
        TemplateField(
          key: 'field',
          type: 'text',
          sensitivity: SensitivityLevel.public,
        ),
        TemplateField(
          key: 'startDate',
          type: 'date',
          sensitivity: SensitivityLevel.public,
        ),
        TemplateField(
          key: 'endDate',
          type: 'date',
          sensitivity: SensitivityLevel.public,
        ),
      ],
    ),
    // 就业模板
    SectionTemplate(
      id: 'professional_employment',
      nameKey: 'templateProfessionalEmploymentName',
      descriptionKey: 'templateProfessionalEmploymentDesc',
      icon: '💼',
      pageTag: 'professional',
      fields: [
        TemplateField(
          key: 'company',
          type: 'text',
          sensitivity: SensitivityLevel.public,
        ),
        TemplateField(
          key: 'position',
          type: 'text',
          sensitivity: SensitivityLevel.public,
        ),
        TemplateField(
          key: 'responsibilities',
          type: 'text',
          sensitivity: SensitivityLevel.sensitive,
        ),
        TemplateField(
          key: 'startDate',
          type: 'date',
          sensitivity: SensitivityLevel.public,
        ),
        TemplateField(
          key: 'endDate',
          type: 'date',
          sensitivity: SensitivityLevel.public,
        ),
      ],
    ),
    // 技能模板
    SectionTemplate(
      id: 'professional_skill',
      nameKey: 'templateProfessionalSkillName',
      descriptionKey: 'templateProfessionalSkillDesc',
      icon: '🛠️',
      pageTag: 'professional',
      fields: [
        TemplateField(
          key: 'name',
          type: 'text',
          sensitivity: SensitivityLevel.public,
        ),
        TemplateField(
          key: 'level',
          type: 'text',
          sensitivity: SensitivityLevel.public,
        ),
      ],
    ),
    // 语言模板
    SectionTemplate(
      id: 'professional_language',
      nameKey: 'templateProfessionalLanguageName',
      descriptionKey: 'templateProfessionalLanguageDesc',
      icon: '🌐',
      pageTag: 'professional',
      fields: [
        TemplateField(
          key: 'name',
          type: 'text',
          sensitivity: SensitivityLevel.public,
        ),
        TemplateField(
          key: 'proficiency',
          type: 'text',
          sensitivity: SensitivityLevel.public,
        ),
      ],
    ),
    // 奖项模板
    SectionTemplate(
      id: 'professional_award',
      nameKey: 'templateProfessionalAwardName',
      descriptionKey: 'templateProfessionalAwardDesc',
      icon: '🏆',
      pageTag: 'professional',
      fields: [
        TemplateField(
          key: 'issuer',
          type: 'text',
          sensitivity: SensitivityLevel.public,
        ),
        TemplateField(
          key: 'date',
          type: 'date',
          sensitivity: SensitivityLevel.public,
        ),
        TemplateField(
          key: 'description',
          type: 'text',
          sensitivity: SensitivityLevel.sensitive,
        ),
      ],
    ),
    // 护照模板
    SectionTemplate(
      id: 'travel_passport',
      nameKey: 'templateTravelPassportName',
      descriptionKey: 'templateTravelPassportDesc',
      icon: '🛂',
      pageTag: 'travel',
      fields: [
        TemplateField(
          key: 'country',
          type: 'text',
          sensitivity: SensitivityLevel.public,
        ),
        TemplateField(
          key: 'country_code',
          type: 'text',
          sensitivity: SensitivityLevel.public,
        ),
        TemplateField(
          key: 'number',
          type: 'text',
          sensitivity: SensitivityLevel.critical,
        ),
        TemplateField(
          key: 'issue_date',
          type: 'date',
          sensitivity: SensitivityLevel.internal,
        ),
        TemplateField(
          key: 'place_of_issue',
          type: 'text',
          sensitivity: SensitivityLevel.sensitive,
        ),
        TemplateField(
          key: 'expiry_date',
          type: 'date',
          sensitivity: SensitivityLevel.internal,
        ),
        TemplateField(
          key: 'holder_name',
          type: 'text',
          sensitivity: SensitivityLevel.sensitive,
        ),
        TemplateField(
          key: 'date_of_birth',
          type: 'date',
          sensitivity: SensitivityLevel.sensitive,
        ),
        TemplateField(
          key: 'place_of_birth',
          type: 'text',
          sensitivity: SensitivityLevel.sensitive,
        ),
        TemplateField(
          key: 'sex',
          type: 'text',
          sensitivity: SensitivityLevel.public,
        ),
        TemplateField(
          key: 'nationality',
          type: 'text',
          sensitivity: SensitivityLevel.sensitive,
        ),
        TemplateField(
          key: 'authority',
          type: 'text',
          sensitivity: SensitivityLevel.sensitive,
        ),
      ],
    ),
    // 签证模板
    SectionTemplate(
      id: 'travel_visa',
      nameKey: 'templateTravelVisaName',
      descriptionKey: 'templateTravelVisaDesc',
      icon: '📋',
      pageTag: 'travel',
      fields: [
        TemplateField(
          key: 'country',
          type: 'text',
          sensitivity: SensitivityLevel.public,
        ),
        TemplateField(
          key: 'visa_type',
          type: 'text',
          sensitivity: SensitivityLevel.sensitive,
        ),
        TemplateField(
          key: 'number',
          type: 'text',
          sensitivity: SensitivityLevel.critical,
        ),
        TemplateField(
          key: 'issue_date',
          type: 'date',
          sensitivity: SensitivityLevel.internal,
        ),
        TemplateField(
          key: 'expiry_date',
          type: 'date',
          sensitivity: SensitivityLevel.sensitive,
        ),
      ],
    ),
    // 旅行历史模板
    SectionTemplate(
      id: 'travel_history',
      nameKey: 'templateTravelHistoryName',
      descriptionKey: 'templateTravelHistoryDesc',
      icon: '✈️',
      pageTag: 'travel',
      fields: [
        TemplateField(
          key: 'destination',
          type: 'text',
          sensitivity: SensitivityLevel.public,
        ),
        TemplateField(
          key: 'travel_type',
          type: 'text',
          sensitivity: SensitivityLevel.public,
        ),
        TemplateField(
          key: 'date',
          type: 'date',
          sensitivity: SensitivityLevel.public,
        ),
        TemplateField(
          key: 'departure_city',
          type: 'text',
          sensitivity: SensitivityLevel.public,
        ),
        TemplateField(
          key: 'departure_time',
          type: 'text',
          sensitivity: SensitivityLevel.public,
        ),
        TemplateField(
          key: 'arrival_time',
          type: 'text',
          sensitivity: SensitivityLevel.public,
        ),
        TemplateField(
          key: 'flight_number',
          type: 'text',
          sensitivity: SensitivityLevel.public,
        ),
        TemplateField(
          key: 'ticket_price',
          type: 'text',
          sensitivity: SensitivityLevel.sensitive,
        ),
        TemplateField(
          key: 'airline',
          type: 'text',
          sensitivity: SensitivityLevel.public,
        ),
      ],
    ),
  ];
}
