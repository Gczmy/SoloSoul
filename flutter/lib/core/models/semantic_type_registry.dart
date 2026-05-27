import 'package:flutter/material.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';

/// 预定义的语义类型（Semantic Field Type）。
///
/// 语义类型是插件与用户自定义字段之间的"通用语言"。
/// 它描述"这个字段代表什么含义"，而不是"这个字段叫什么名字"。
///
/// 插件通过语义类型请求数据，不依赖用户的具体字段名或语言。
class SemanticFieldType {
  /// 标准化标识符
  /// 官方类型：短 ID，如 "pet.name"、"person.birth_date"
  /// 第三方扩展：命名空间格式，如 "com.example.veterinarian.license"
  final String id;

  /// 多语言显示名称
  /// {"zh": "宠物名字", "en": "Pet Name", "ja": "ペットの名前"}
  final Map<String, String> labels;

  /// 多语言说明（用于 UI 引导）
  final Map<String, String> descriptions;

  /// 所属分类
  final String category;

  /// 建议的属性类型
  final String suggestedPropertyType;

  /// 默认敏感度（用户可覆盖）
  final SensitivityLevel defaultSensitivity;

  /// 图标名称（Material Icons）
  final IconData icon;

  /// 该语义类型首次引入的 App 版本（用于兼容性检查）
  final String minAppVersion;

  const SemanticFieldType({
    required this.id,
    required this.labels,
    required this.descriptions,
    required this.category,
    required this.suggestedPropertyType,
    required this.defaultSensitivity,
    required this.icon,
    this.minAppVersion = '1.0.0',
  });

  /// 获取当前语言下的显示标签
  ///
  /// 回退逻辑：指定语言 → 英文 → 中文 → 机械格式化最后一段
  String getLabel(String languageCode) {
    final label = labels[languageCode] ??
        labels['en'] ??
        labels['zh'];
    if (label != null && label.isNotEmpty) return label;
    // 最终降级：使用 id 最后一段的机械格式化
    return _formatFieldLabel(id.split('.').last);
  }

  static String _formatFieldLabel(String key) {
    final spaced = key.replaceAllMapped(
      RegExp(r'([a-z])([A-Z])'),
      (m) => '${m[1]} ${m[2]}',
    );
    return spaced.replaceAll('_', ' ').split(' ').map((word) {
      if (word.isEmpty) return word;
      return word[0].toUpperCase() + word.substring(1).toLowerCase();
    }).join(' ');
  }

  /// 获取当前语言下的说明
  String getDescription(String languageCode) {
    return descriptions[languageCode] ??
        descriptions['en'] ??
        descriptions['zh'] ??
        '';
  }
}

/// 语义类型注册表。
///
/// 维护所有预定义语义类型的集合，支持按分类检索和关键词搜索。
class SemanticTypeRegistry {
  static const List<SemanticFieldType> _allTypes = [
    // ========== person ==========
    SemanticFieldType(
      id: 'person.name',
      labels: {'zh': '姓名', 'en': 'Full Name', 'ja': '氏名'},
      descriptions: {
        'zh': '个人的完整姓名',
        'en': 'The full legal name of a person',
      },
      category: 'person',
      suggestedPropertyType: 'text',
      defaultSensitivity: SensitivityLevel.public,
      icon: Icons.person,
    ),
    SemanticFieldType(
      id: 'person.nickname',
      labels: {'zh': '昵称', 'en': 'Nickname', 'ja': 'ニックネーム'},
      descriptions: {
        'zh': '常用称呼或小名',
        'en': 'A familiar or informal name',
      },
      category: 'person',
      suggestedPropertyType: 'text',
      defaultSensitivity: SensitivityLevel.public,
      icon: Icons.face,
    ),
    SemanticFieldType(
      id: 'person.given_name',
      labels: {'zh': '名', 'en': 'Given Name', 'ja': '名'},
      descriptions: {
        'zh': '名字（不包括姓氏）',
        'en': 'First or given name',
      },
      category: 'person',
      suggestedPropertyType: 'text',
      defaultSensitivity: SensitivityLevel.public,
      icon: Icons.person_outline,
    ),
    SemanticFieldType(
      id: 'person.family_name',
      labels: {'zh': '姓', 'en': 'Family Name', 'ja': '姓'},
      descriptions: {
        'zh': '姓氏',
        'en': 'Last name or family name',
      },
      category: 'person',
      suggestedPropertyType: 'text',
      defaultSensitivity: SensitivityLevel.public,
      icon: Icons.person_outline,
    ),
    SemanticFieldType(
      id: 'person.birth_date',
      labels: {'zh': '出生日期', 'en': 'Date of Birth', 'ja': '生年月日'},
      descriptions: {
        'zh': '出生年月日',
        'en': 'Date of birth',
      },
      category: 'person',
      suggestedPropertyType: 'date',
      defaultSensitivity: SensitivityLevel.sensitive,
      icon: Icons.cake,
    ),
    SemanticFieldType(
      id: 'person.gender',
      labels: {'zh': '性别', 'en': 'Gender', 'ja': '性別'},
      descriptions: {
        'zh': '生理性别或社会性别',
        'en': 'Gender identity',
      },
      category: 'person',
      suggestedPropertyType: 'select',
      defaultSensitivity: SensitivityLevel.public,
      icon: Icons.wc,
    ),
    SemanticFieldType(
      id: 'person.nationality',
      labels: {'zh': '国籍', 'en': 'Nationality', 'ja': '国籍'},
      descriptions: {
        'zh': '所持国籍',
        'en': 'Country of nationality',
      },
      category: 'person',
      suggestedPropertyType: 'text',
      defaultSensitivity: SensitivityLevel.public,
      icon: Icons.public,
    ),

    // ========== pet ==========
    SemanticFieldType(
      id: 'pet.name',
      labels: {'zh': '宠物名字', 'en': 'Pet Name', 'ja': 'ペットの名前'},
      descriptions: {
        'zh': '宠物的名字或昵称',
        'en': 'The name of a pet',
      },
      category: 'pet',
      suggestedPropertyType: 'text',
      defaultSensitivity: SensitivityLevel.public,
      icon: Icons.pets,
    ),
    SemanticFieldType(
      id: 'pet.breed',
      labels: {'zh': '品种', 'en': 'Breed', 'ja': '品種'},
      descriptions: {
        'zh': '宠物的品种',
        'en': 'The breed of a pet',
      },
      category: 'pet',
      suggestedPropertyType: 'text',
      defaultSensitivity: SensitivityLevel.public,
      icon: Icons.pets,
    ),
    SemanticFieldType(
      id: 'pet.species',
      labels: {'zh': '物种', 'en': 'Species', 'ja': '種'},
      descriptions: {
        'zh': '动物物种（如犬、猫、鸟）',
        'en': 'Animal species (e.g. dog, cat, bird)',
      },
      category: 'pet',
      suggestedPropertyType: 'select',
      defaultSensitivity: SensitivityLevel.public,
      icon: Icons.pets,
    ),
    SemanticFieldType(
      id: 'pet.birth_date',
      labels: {'zh': '出生日期', 'en': 'Birth Date', 'ja': '生年月日'},
      descriptions: {
        'zh': '宠物的出生日期',
        'en': 'Pet date of birth',
      },
      category: 'pet',
      suggestedPropertyType: 'date',
      defaultSensitivity: SensitivityLevel.public,
      icon: Icons.cake,
    ),
    SemanticFieldType(
      id: 'pet.weight',
      labels: {'zh': '体重', 'en': 'Weight', 'ja': '体重'},
      descriptions: {
        'zh': '宠物体重（千克）',
        'en': 'Pet weight in kilograms',
      },
      category: 'pet',
      suggestedPropertyType: 'number',
      defaultSensitivity: SensitivityLevel.public,
      icon: Icons.monitor_weight,
    ),
    SemanticFieldType(
      id: 'pet.color',
      labels: {'zh': '毛色', 'en': 'Color', 'ja': '毛色'},
      descriptions: {
        'zh': '宠物毛色或羽毛颜色',
        'en': 'Fur or feather color',
      },
      category: 'pet',
      suggestedPropertyType: 'text',
      defaultSensitivity: SensitivityLevel.public,
      icon: Icons.color_lens,
    ),
    SemanticFieldType(
      id: 'pet.vet_name',
      labels: {'zh': '兽医姓名', 'en': 'Veterinarian', 'ja': '獣医師'},
      descriptions: {
        'zh': '负责兽医的姓名',
        'en': 'Name of the veterinarian',
      },
      category: 'pet',
      suggestedPropertyType: 'text',
      defaultSensitivity: SensitivityLevel.public,
      icon: Icons.local_hospital,
    ),
    SemanticFieldType(
      id: 'pet.vet_phone',
      labels: {'zh': '兽医电话', 'en': 'Vet Phone', 'ja': '獣医師電話'},
      descriptions: {
        'zh': '兽医诊所联系电话',
        'en': 'Veterinarian clinic phone number',
      },
      category: 'pet',
      suggestedPropertyType: 'text',
      defaultSensitivity: SensitivityLevel.sensitive,
      icon: Icons.phone,
    ),

    // ========== financial ==========
    SemanticFieldType(
      id: 'financial.account_number',
      labels: {'zh': '账号', 'en': 'Account Number', 'ja': '口座番号'},
      descriptions: {
        'zh': '银行账户号码',
        'en': 'Bank account number',
      },
      category: 'financial',
      suggestedPropertyType: 'text',
      defaultSensitivity: SensitivityLevel.critical,
      icon: Icons.account_balance,
    ),
    SemanticFieldType(
      id: 'financial.bank_name',
      labels: {'zh': '银行名称', 'en': 'Bank Name', 'ja': '銀行名'},
      descriptions: {
        'zh': '开户银行名称',
        'en': 'Name of the bank',
      },
      category: 'financial',
      suggestedPropertyType: 'text',
      defaultSensitivity: SensitivityLevel.public,
      icon: Icons.account_balance,
    ),
    SemanticFieldType(
      id: 'financial.swift_code',
      labels: {'zh': 'SWIFT 代码', 'en': 'SWIFT Code', 'ja': 'SWIFTコード'},
      descriptions: {
        'zh': '银行 SWIFT/BIC 代码',
        'en': 'Bank SWIFT/BIC code',
      },
      category: 'financial',
      suggestedPropertyType: 'text',
      defaultSensitivity: SensitivityLevel.sensitive,
      icon: Icons.code,
    ),
    SemanticFieldType(
      id: 'financial.iban',
      labels: {'zh': 'IBAN', 'en': 'IBAN', 'ja': 'IBAN'},
      descriptions: {
        'zh': '国际银行账户号码',
        'en': 'International Bank Account Number',
      },
      category: 'financial',
      suggestedPropertyType: 'text',
      defaultSensitivity: SensitivityLevel.critical,
      icon: Icons.account_balance_wallet,
    ),
    SemanticFieldType(
      id: 'financial.card_number',
      labels: {'zh': '卡号', 'en': 'Card Number', 'ja': 'カード番号'},
      descriptions: {
        'zh': '银行卡号',
        'en': 'Payment card number',
      },
      category: 'financial',
      suggestedPropertyType: 'text',
      defaultSensitivity: SensitivityLevel.critical,
      icon: Icons.credit_card,
    ),
    SemanticFieldType(
      id: 'financial.card_cvv',
      labels: {'zh': 'CVV', 'en': 'CVV', 'ja': 'CVV'},
      descriptions: {
        'zh': '卡片安全码',
        'en': 'Card verification value',
      },
      category: 'financial',
      suggestedPropertyType: 'text',
      defaultSensitivity: SensitivityLevel.critical,
      icon: Icons.security,
    ),
    SemanticFieldType(
      id: 'financial.card_expiry',
      labels: {'zh': '有效期', 'en': 'Expiry Date', 'ja': '有効期限'},
      descriptions: {
        'zh': '卡片到期日期',
        'en': 'Card expiration date',
      },
      category: 'financial',
      suggestedPropertyType: 'date',
      defaultSensitivity: SensitivityLevel.critical,
      icon: Icons.date_range,
    ),
    SemanticFieldType(
      id: 'financial.tax_id',
      labels: {'zh': '税号', 'en': 'Tax ID', 'ja': '税務ID'},
      descriptions: {
        'zh': '纳税人识别号',
        'en': 'Tax identification number',
      },
      category: 'financial',
      suggestedPropertyType: 'text',
      defaultSensitivity: SensitivityLevel.critical,
      icon: Icons.receipt,
    ),

    // ========== contact ==========
    SemanticFieldType(
      id: 'contact.phone',
      labels: {'zh': '电话号码', 'en': 'Phone Number', 'ja': '電話番号'},
      descriptions: {
        'zh': '联系电话号码',
        'en': 'Contact phone number',
      },
      category: 'contact',
      suggestedPropertyType: 'text',
      defaultSensitivity: SensitivityLevel.sensitive,
      icon: Icons.phone,
    ),
    SemanticFieldType(
      id: 'contact.email',
      labels: {'zh': '邮箱', 'en': 'Email', 'ja': 'メール'},
      descriptions: {
        'zh': '电子邮箱地址',
        'en': 'Email address',
      },
      category: 'contact',
      suggestedPropertyType: 'text',
      defaultSensitivity: SensitivityLevel.internal,
      icon: Icons.email,
    ),
    SemanticFieldType(
      id: 'contact.address',
      labels: {'zh': '地址', 'en': 'Address', 'ja': '住所'},
      descriptions: {
        'zh': '居住或工作地址',
        'en': 'Physical address',
      },
      category: 'contact',
      suggestedPropertyType: 'text',
      defaultSensitivity: SensitivityLevel.sensitive,
      icon: Icons.location_on,
    ),
    SemanticFieldType(
      id: 'contact.emergency_contact',
      labels: {'zh': '紧急联系人', 'en': 'Emergency Contact', 'ja': '緊急連絡先'},
      descriptions: {
        'zh': '紧急情况下的联系人信息',
        'en': 'Person to contact in emergencies',
      },
      category: 'contact',
      suggestedPropertyType: 'text',
      defaultSensitivity: SensitivityLevel.sensitive,
      icon: Icons.contact_phone,
    ),

    // ========== travel ==========
    SemanticFieldType(
      id: 'travel.passport_number',
      labels: {'zh': '护照号码', 'en': 'Passport Number', 'ja': 'パスポート番号'},
      descriptions: {
        'zh': '护照号码',
        'en': 'Passport document number',
      },
      category: 'travel',
      suggestedPropertyType: 'text',
      defaultSensitivity: SensitivityLevel.critical,
      icon: Icons.book,
    ),
    SemanticFieldType(
      id: 'travel.visa_number',
      labels: {'zh': '签证号码', 'en': 'Visa Number', 'ja': 'ビザ番号'},
      descriptions: {
        'zh': '签证号码',
        'en': 'Visa document number',
      },
      category: 'travel',
      suggestedPropertyType: 'text',
      defaultSensitivity: SensitivityLevel.critical,
      icon: Icons.bookmark,
    ),
    SemanticFieldType(
      id: 'travel.flight_number',
      labels: {'zh': '航班号', 'en': 'Flight Number', 'ja': '便名'},
      descriptions: {
        'zh': '航班编号',
        'en': 'Airline flight number',
      },
      category: 'travel',
      suggestedPropertyType: 'text',
      defaultSensitivity: SensitivityLevel.public,
      icon: Icons.flight,
    ),
    SemanticFieldType(
      id: 'travel.hotel_name',
      labels: {'zh': '酒店名称', 'en': 'Hotel Name', 'ja': 'ホテル名'},
      descriptions: {
        'zh': '预订酒店名称',
        'en': 'Hotel or accommodation name',
      },
      category: 'travel',
      suggestedPropertyType: 'text',
      defaultSensitivity: SensitivityLevel.public,
      icon: Icons.hotel,
    ),
    SemanticFieldType(
      id: 'travel.check_in_date',
      labels: {'zh': '入住日期', 'en': 'Check-in Date', 'ja': 'チェックイン日'},
      descriptions: {
        'zh': '酒店入住日期',
        'en': 'Hotel check-in date',
      },
      category: 'travel',
      suggestedPropertyType: 'date',
      defaultSensitivity: SensitivityLevel.public,
      icon: Icons.calendar_today,
    ),

    // ========== professional ==========
    SemanticFieldType(
      id: 'professional.company',
      labels: {'zh': '公司', 'en': 'Company', 'ja': '会社'},
      descriptions: {
        'zh': '工作单位名称',
        'en': 'Employer or company name',
      },
      category: 'professional',
      suggestedPropertyType: 'text',
      defaultSensitivity: SensitivityLevel.public,
      icon: Icons.business,
    ),
    SemanticFieldType(
      id: 'professional.position',
      labels: {'zh': '职位', 'en': 'Position', 'ja': '役職'},
      descriptions: {
        'zh': '职位或岗位名称',
        'en': 'Job title or position',
      },
      category: 'professional',
      suggestedPropertyType: 'text',
      defaultSensitivity: SensitivityLevel.public,
      icon: Icons.work,
    ),
    SemanticFieldType(
      id: 'professional.department',
      labels: {'zh': '部门', 'en': 'Department', 'ja': '部署'},
      descriptions: {
        'zh': '所属部门',
        'en': 'Department or division',
      },
      category: 'professional',
      suggestedPropertyType: 'text',
      defaultSensitivity: SensitivityLevel.public,
      icon: Icons.account_tree,
    ),
    SemanticFieldType(
      id: 'professional.start_date',
      labels: {'zh': '入职日期', 'en': 'Start Date', 'ja': '入社日'},
      descriptions: {
        'zh': '工作开始日期',
        'en': 'Employment start date',
      },
      category: 'professional',
      suggestedPropertyType: 'date',
      defaultSensitivity: SensitivityLevel.public,
      icon: Icons.play_circle,
    ),
    SemanticFieldType(
      id: 'professional.end_date',
      labels: {'zh': '离职日期', 'en': 'End Date', 'ja': '退社日'},
      descriptions: {
        'zh': '工作结束日期',
        'en': 'Employment end date',
      },
      category: 'professional',
      suggestedPropertyType: 'date',
      defaultSensitivity: SensitivityLevel.public,
      icon: Icons.stop_circle,
    ),

    // ========== education ==========
    SemanticFieldType(
      id: 'education.institution',
      labels: {'zh': '学校', 'en': 'Institution', 'ja': '学校'},
      descriptions: {
        'zh': '教育机构名称',
        'en': 'School or university name',
      },
      category: 'education',
      suggestedPropertyType: 'text',
      defaultSensitivity: SensitivityLevel.public,
      icon: Icons.school,
    ),
    SemanticFieldType(
      id: 'education.degree',
      labels: {'zh': '学位', 'en': 'Degree', 'ja': '学位'},
      descriptions: {
        'zh': '所获学位（学士/硕士/博士）',
        'en': 'Academic degree earned',
      },
      category: 'education',
      suggestedPropertyType: 'select',
      defaultSensitivity: SensitivityLevel.public,
      icon: Icons.emoji_events,
    ),
    SemanticFieldType(
      id: 'education.major',
      labels: {'zh': '专业', 'en': 'Major', 'ja': '専攻'},
      descriptions: {
        'zh': '所学专业',
        'en': 'Field of study',
      },
      category: 'education',
      suggestedPropertyType: 'text',
      defaultSensitivity: SensitivityLevel.public,
      icon: Icons.menu_book,
    ),
    SemanticFieldType(
      id: 'education.graduation_date',
      labels: {'zh': '毕业日期', 'en': 'Graduation Date', 'ja': '卒業日'},
      descriptions: {
        'zh': '毕业日期',
        'en': 'Date of graduation',
      },
      category: 'education',
      suggestedPropertyType: 'date',
      defaultSensitivity: SensitivityLevel.public,
      icon: Icons.celebration,
    ),

    // ========== generic ==========
    SemanticFieldType(
      id: 'generic.note',
      labels: {'zh': '备注', 'en': 'Note', 'ja': 'メモ'},
      descriptions: {
        'zh': '任意文本备注',
        'en': 'Free-form text note',
      },
      category: 'generic',
      suggestedPropertyType: 'text',
      defaultSensitivity: SensitivityLevel.public,
      icon: Icons.notes,
    ),
    SemanticFieldType(
      id: 'generic.url',
      labels: {'zh': '网址', 'en': 'URL', 'ja': 'URL'},
      descriptions: {
        'zh': '网页链接地址',
        'en': 'Web URL or link',
      },
      category: 'generic',
      suggestedPropertyType: 'url',
      defaultSensitivity: SensitivityLevel.public,
      icon: Icons.link,
    ),
    SemanticFieldType(
      id: 'generic.date',
      labels: {'zh': '日期', 'en': 'Date', 'ja': '日付'},
      descriptions: {
        'zh': '任意日期',
        'en': 'Generic date field',
      },
      category: 'generic',
      suggestedPropertyType: 'date',
      defaultSensitivity: SensitivityLevel.public,
      icon: Icons.event,
    ),
    SemanticFieldType(
      id: 'generic.number',
      labels: {'zh': '数字', 'en': 'Number', 'ja': '数値'},
      descriptions: {
        'zh': '任意数值',
        'en': 'Generic numeric field',
      },
      category: 'generic',
      suggestedPropertyType: 'number',
      defaultSensitivity: SensitivityLevel.public,
      icon: Icons.numbers,
    ),
    SemanticFieldType(
      id: 'generic.tag',
      labels: {'zh': '标签', 'en': 'Tag', 'ja': 'タグ'},
      descriptions: {
        'zh': '分类标签',
        'en': 'Categorical tag or label',
      },
      category: 'generic',
      suggestedPropertyType: 'text',
      defaultSensitivity: SensitivityLevel.public,
      icon: Icons.label,
    ),
    SemanticFieldType(
      id: 'generic.attachment',
      labels: {'zh': '附件', 'en': 'Attachment', 'ja': '添付ファイル'},
      descriptions: {
        'zh': '文件附件',
        'en': 'File attachment',
      },
      category: 'generic',
      suggestedPropertyType: 'relation',
      defaultSensitivity: SensitivityLevel.public,
      icon: Icons.attach_file,
    ),

    // ========== custom ==========
    SemanticFieldType(
      id: 'custom.untyped',
      labels: {'zh': '其他', 'en': 'Other', 'ja': 'その他'},
      descriptions: {
        'zh': '未分类的其他字段',
        'en': 'Uncategorized or custom field',
      },
      category: 'custom',
      suggestedPropertyType: 'text',
      defaultSensitivity: SensitivityLevel.public,
      icon: Icons.help_outline,
    ),
  ];

  static List<SemanticFieldType> get allTypes => List.unmodifiable(_allTypes);

  static SemanticFieldType? getType(String id) {
    for (final type in _allTypes) {
      if (type.id == id) return type;
    }
    return null;
  }

  static List<SemanticFieldType> getTypesByCategory(String category) {
    return _allTypes.where((t) => t.category == category).toList();
  }

  static List<String> get categories {
    return _allTypes.map((t) => t.category).toSet().toList()
      ..sort();
  }

  static String getCategoryLabel(String category, String languageCode) {
    final labels = {
      'person': {'zh': '人物', 'en': 'Person', 'ja': '人物'},
      'pet': {'zh': '宠物', 'en': 'Pet', 'ja': 'ペット'},
      'financial': {'zh': '财务', 'en': 'Financial', 'ja': '金融'},
      'contact': {'zh': '联系方式', 'en': 'Contact', 'ja': '連絡先'},
      'travel': {'zh': '旅行', 'en': 'Travel', 'ja': '旅行'},
      'professional': {'zh': '职业', 'en': 'Professional', 'ja': '職業'},
      'education': {'zh': '教育', 'en': 'Education', 'ja': '教育'},
      'generic': {'zh': '通用', 'en': 'General', 'ja': '一般'},
      'custom': {'zh': '自定义', 'en': 'Custom', 'ja': 'カスタム'},
    };
    return labels[category]?[languageCode] ??
        labels[category]?['en'] ??
        category;
  }

  static IconData getCategoryIcon(String category) {
    final icons = {
      'person': Icons.person,
      'pet': Icons.pets,
      'financial': Icons.account_balance,
      'contact': Icons.contact_phone,
      'travel': Icons.flight,
      'professional': Icons.work,
      'education': Icons.school,
      'generic': Icons.category,
      'custom': Icons.settings,
    };
    return icons[category] ?? Icons.label;
  }

  /// 根据关键词搜索语义类型
  static List<SemanticFieldType> search(String query, String languageCode) {
    final lowerQuery = query.toLowerCase();
    return _allTypes.where((type) {
      final label = type.getLabel(languageCode).toLowerCase();
      final desc = type.getDescription(languageCode).toLowerCase();
      final id = type.id.toLowerCase();
      return label.contains(lowerQuery) ||
          desc.contains(lowerQuery) ||
          id.contains(lowerQuery);
    }).toList();
  }

  /// 根据字段名称和 section 上下文推荐语义类型
  static List<SemanticFieldType> recommend(
    String label,
    String sectionName,
    String languageCode,
  ) {
    final results = <SemanticFieldType>[];
    final lowerLabel = label.toLowerCase();
    final lowerSection = sectionName.toLowerCase();

    // 1. 根据字段名称关键词匹配
    final keywordMap = {
      'name': ['person.name', 'pet.name'],
      '昵称': ['person.nickname', 'pet.name'],
      'nickname': ['person.nickname', 'pet.name'],
      'breed': ['pet.breed'],
      '品种': ['pet.breed'],
      'species': ['pet.species'],
      '物种': ['pet.species'],
      'birth': ['person.birth_date', 'pet.birth_date'],
      '出生': ['person.birth_date', 'pet.birth_date'],
      'weight': ['pet.weight'],
      '体重': ['pet.weight'],
      'color': ['pet.color'],
      '颜色': ['pet.color'],
      'phone': ['contact.phone'],
      '电话': ['contact.phone'],
      'email': ['contact.email'],
      '邮箱': ['contact.email'],
      'address': ['contact.address'],
      '地址': ['contact.address'],
      'note': ['generic.note'],
      '备注': ['generic.note'],
      'url': ['generic.url'],
      '网址': ['generic.url'],
      'date': ['generic.date'],
      '日期': ['generic.date'],
      'number': ['generic.number'],
      '数字': ['generic.number'],
      'tag': ['generic.tag'],
      '标签': ['generic.tag'],
      'company': ['professional.company'],
      '公司': ['professional.company'],
      'position': ['professional.position'],
      '职位': ['professional.position'],
      'school': ['education.institution'],
      '学校': ['education.institution'],
      'degree': ['education.degree'],
      '学位': ['education.degree'],
    };

    for (final entry in keywordMap.entries) {
      if (lowerLabel.contains(entry.key)) {
        for (final typeId in entry.value) {
          final type = getType(typeId);
          if (type != null && !results.contains(type)) {
            results.add(type);
          }
        }
      }
    }

    // 2. 根据 section 名称上下文增强推荐优先级
    final sectionCategoryMap = {
      '宠物': 'pet',
      'pet': 'pet',
      '狗': 'pet',
      '猫': 'pet',
      '人': 'person',
      'person': 'person',
      '身份': 'person',
      '银行': 'financial',
      'bank': 'financial',
      '财务': 'financial',
      '金融': 'financial',
      '旅行': 'travel',
      'travel': 'travel',
      '工作': 'professional',
      'work': 'professional',
      '职业': 'professional',
      '教育': 'education',
      'education': 'education',
      '学校': 'education',
    };

    String? targetCategory;
    for (final entry in sectionCategoryMap.entries) {
      if (lowerSection.contains(entry.key)) {
        targetCategory = entry.value;
        break;
      }
    }

    if (targetCategory != null) {
      results.sort((a, b) {
        final aMatch = a.category == targetCategory ? -1 : 0;
        final bMatch = b.category == targetCategory ? -1 : 0;
        return aMatch - bMatch;
      });
    }

    return results.isEmpty ? [getType('custom.untyped')!] : results.take(5).toList();
  }

  // ============================================================================
  // 字段路径 → 语义类型映射（供 FieldLabelResolver 使用）
  // ============================================================================

  /// 插件 manifest / Vault 存储字段路径 → 语义类型 ID 的集中映射。
  ///
  /// 这是**预定义字段**的静态映射。用户自定义字段通过 [resolveByFieldPath]
  /// 的 [sectionId] + [machineKey] 参数从运行时数据动态查找。
  static const Map<String, String> _fieldPathToSemanticType = {
    // 映射原则：
    // 1. 仅保留"一对一"或"语义类型标签明显比 ARB 更精确"的映射。
    // 2. 多个字段路径映射到同一语义类型会导致 UI 重复标签（如 address.*
    //    全部显示为"地址"），此类映射删除，让字段 fallback 到 ARB
    //    的独立翻译（street→"街道", city→"城市" 等）。
    // 3. ARB 中无翻译的字段（如 emergencyName）保留映射以保证中文显示。

    // ========== Identity ==========
    // fullName: ARB "真实姓名" vs 语义类型 "姓名" — 语义类型更简洁通用，保留
    'identity.fullName': 'person.name',
    'identity.full_name': 'person.name',
    // 其余 identity 子字段 ARB 翻译完整（dateOfBirth→出生日期, nationality→国籍,
    // sex→性别, title→标题），删除映射避免重复且精确度足够

    // ========== Contact ==========
    'contact.email': 'contact.email',
    'contact.phone': 'contact.phone',
    // emergencyName 无 ARB 翻译，保留映射保证中文"紧急联系人"
    'contact.emergencyName': 'contact.emergency_contact',
    // emergencyPhone 无 ARB 翻译，但映射到 contact.phone 会导致与 contact.phone
    // 显示相同标签（"电话号码"），不如删除让其显示为 "Emergency Phone"

    // ========== Address ==========
    // 所有 address 子字段删除映射：ARB 有独立精确翻译
    // (street→街道, city→城市, state→省/州, postalCode→邮政编码,
    //  country→国家/地区, district→区/县, title→标题)
    // 避免全部显示为重复的 "地址"

    // ========== Passport ==========
    // number: ARB "号码" vs 语义类型 "护照号码" — 语义类型更精确，保留
    'passport.number': 'travel.passport_number',
    // surname / givenNames: 无 ARB 翻译，保留映射保证中文"姓"/"名"
    'passport.surname': 'person.family_name',
    'passport.givenNames': 'person.given_name',
    // 其余 passport 子字段 ARB 翻译完整（expiryDate→到期日, issueDate→签发日,
    // issuingAuthority→签发机关, placeOfBirth→出生地, nationality→国籍,
    // sex→性别, dateOfBirth→出生日期），删除映射

    // ========== Visa ==========
    // number: ARB "号码" vs 语义类型 "签证号码" — 语义类型更精确，保留
    'visa.number': 'travel.visa_number',
    // 其余 visa 子字段 ARB 翻译完整（type→类型, expiryDate→到期日, issueDate→签发日）

    // ========== ID Card / Card ==========
    // 无合适语义类型，ARB 有 number→号码 / expiryDate→到期日

    // ========== Financial ==========
    // accountNumber: ARB "账号" vs 语义类型 "账号" — 相同，保留
    'financial.accountNumber': 'financial.account_number',
    'financial.bankName': 'financial.bank_name',
    'financial.swiftCode': 'financial.swift_code',
    'financial.iban': 'financial.iban',
    'financial.taxIdNumber': 'financial.tax_id',

    // ========== Education ==========
    // institution: ARB "院校" vs 语义类型 "学校" — 语义类型更通用，保留
    'education.institution': 'education.institution',
    'education.degree': 'education.degree',
    // year 无 ARB 翻译时 fallback 到 "Year"，保留映射 "日期" 也无意义，删除
    // field 映射到 major（"专业"）但 ARB 有 field→"领域"，删除

    // ========== Employment ==========
    'employment.company': 'professional.company',
    'employment.position': 'professional.position',
    // startDate / endDate: 语义类型 "入职日期"/"离职日期" 比 ARB "开始日期"/"结束日期" 更精确
    'employment.startDate': 'professional.start_date',
    'employment.endDate': 'professional.end_date',
    // description: ARB "描述" vs 映射到 generic.note（"备注"），ARB 更准确

    // ========== Travel ==========
    // flightNumber: ARB "航班号" vs 语义类型 "航班号" — 相同，保留
    'travel.flightNumber': 'travel.flight_number',
    // destination / hotelBooking 无 ARB 翻译，但映射到 hotel_name（"酒店名称"）
    // 会导致两者都显示为"酒店名称"，不如删除显示原始英文

    // ========== Medical / Security ==========
    // bloodType 映射到 gender（"性别"）明显错误；allergies 映射到 note（"备注"）
    // 也不合适。security.* 全部映射到 note 同样错误。全部删除。

    // ========== Digital Accounts / Tax ==========
    'digitalAccounts.email': 'contact.email',
    'taxId.number': 'financial.tax_id',

    // ========== Legacy nested ==========
    // 全部删除，legacy 路径应逐步淘汰
  };

  /// 根据字段路径解析对应的语义类型。
  ///
  /// 解析优先级：
  /// 1. 静态映射表 [_fieldPathToSemanticType]
  /// 2. 动态查找：如果提供了 [sectionId] 和 [machineKey]，从该 section 的
  ///    `__semanticTypes` 中读取该 machine key 对应的语义类型
  /// 3. 模糊匹配：提取最后一段尝试匹配语义类型 ID
  static SemanticFieldType? resolveByFieldPath(
    String fieldPath, {
    String? sectionId,
    String? machineKey,
  }) {
    // 1. 静态映射表（规范化路径）
    final normalized = _normalizeFieldPath(fieldPath);
    final staticId = _fieldPathToSemanticType[normalized];
    if (staticId != null) return getType(staticId);

    // 2. 动态查找：从用户 section 数据的 __semanticTypes 获取
    if (sectionId != null && machineKey != null) {
      final semanticId = _getSemanticTypeFromSection(sectionId, machineKey);
      if (semanticId != null) return getType(semanticId);
    }

    // 3. 模糊匹配
    return _tryFuzzyMatch(fieldPath);
  }

  /// 规范化字段路径：移除数组索引。
  static String _normalizeFieldPath(String fieldPath) {
    return fieldPath.replaceAll(RegExp(r'\[\d+\]'), '');
  }

  /// 从 section 数据中动态获取字段的语义类型。
  ///
  /// 读取 UnifiedObject section 的 `__semanticTypes` 映射（machineKey → semanticTypeId）。
  /// TODO: 接入 UnifiedObjectService 或 ProfileStorageService 的实际数据读取。
  static String? _getSemanticTypeFromSection(String sectionId, String machineKey) {
    // 当前为架构接入点，待接入实际数据层。
    // 实现后应通过服务定位器或 Provider 读取 section 的 __semanticTypes 字段。
    return null;
  }

  /// 模糊匹配：提取字段路径最后一段，尝试匹配语义类型 ID。
  static SemanticFieldType? _tryFuzzyMatch(String fieldPath) {
    final last = fieldPath.replaceAll(RegExp(r'\[\d+\]'), '').split('.').last;
    for (final type in _allTypes) {
      if (type.id == last || type.id.endsWith('.$last')) {
        return type;
      }
    }
    return null;
  }

  // ============================================================================

  /// 检查语义类型是否存在
  static bool hasType(String id) {
    return _allTypes.any((t) => t.id == id);
  }

  /// 获取语义类型的默认敏感度
  static SensitivityLevel? getDefaultSensitivity(String semanticTypeId) {
    return getType(semanticTypeId)?.defaultSensitivity;
  }
}
