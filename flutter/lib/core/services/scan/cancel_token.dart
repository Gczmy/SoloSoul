import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';

// =============================================================================
// Cancel Token
// =============================================================================

/// Simple cooperative cancellation token for long-running scan operations.
class CancelToken {
  bool _isCanceled = false;
  bool get isCanceled => _isCanceled;
  void cancel() => _isCanceled = true;
}

// =============================================================================
// Scan Constants
// =============================================================================

/// Target file extensions for scanning.
const List<String> kTargetExtensions = [
  '.pdf', '.docx', '.xlsx', '.csv', '.json', '.txt', '.md',
  '.png', '.jpg', '.jpeg', '.webp', '.bmp', '.tiff',
];

/// Image file extensions (subset of target extensions).
const List<String> kImageExtensions = [
  '.png', '.jpg', '.jpeg', '.webp', '.bmp', '.tiff',
];

/// Filename keywords that suggest personal information.
const List<String> kFilenameKeywords = [
  'resume', 'cv', '简历', 'passport', '护照', 'id_card', '身份证',
  'bank', '银行', 'card', '证书', 'credential', 'profile',
  'contact', 'address', 'tax', 'visa', 'education', 'employment',
];

/// Content fingerprint regexes for personal information.
final Map<String, Fingerprint> kFingerprints = {
  'id_card': Fingerprint(
    pattern: RegExp(r'(?<!\d)[1-9]\d{5}(?:19|20)\d{2}(?:0[1-9]|1[0-2])(?:0[1-9]|[12]\d|3[01])\d{3}[\dXx](?!\d)'),
    sensitivity: SensitivityLevel.critical,
  ),
  'phone': Fingerprint(
    pattern: RegExp(r'(?<!\d)1[3-9]\d{9}(?!\d)'),
    sensitivity: SensitivityLevel.sensitive,
  ),
  'email': Fingerprint(
    pattern: RegExp(r'[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}'),
    sensitivity: SensitivityLevel.internal,
  ),
  'passport': Fingerprint(
    pattern: RegExp(r'(?<![A-Z0-9])[A-Z]\d{7,8}(?![A-Z0-9])'),
    sensitivity: SensitivityLevel.critical,
  ),
  'bank_card': Fingerprint(
    pattern: RegExp(r'(?<!\d)\d{4}[\s-]?\d{4}[\s-]?\d{4}[\s-]?\d{4}(?!\d)'),
    sensitivity: SensitivityLevel.critical,
  ),
};

/// Section detection patterns: map section type to keywords in content.
final Map<String, List<String>> kSectionKeywords = {
  'identity': ['姓名', '性别', '民族', '出生', '身份证', 'name', 'gender', 'nationality', 'date of birth'],
  'contact': ['电话', '手机', '邮箱', '地址', 'phone', 'email', 'address', 'contact'],
  'education': ['学校', '学历', '学位', '专业', 'university', 'college', 'degree', 'major', 'education'],
  'passport': ['护照', 'passport', '国籍', 'nationality', 'place of birth'],
  'visa': ['签证', 'visa', 'visa type', '签证类型'],
  'bankAccount': ['银行', '账户', '开户行', 'bank', 'account number', 'swift', 'sort code'],
  'card': ['信用卡', '借记卡', 'card number', 'cvv', 'expiry'],
  'employment': ['公司', '职位', '工作', 'company', 'employer', 'position', 'job title'],
};

/// Property mapping: section -> key patterns -> propertyId
final Map<String, Map<String, String>> kPropertyMapping = {
  'identity': {
    'fullName|姓名|名字': 'fullName',
    'givenName|given': 'givenName',
    'familyName|姓|姓氏': 'familyName',
    'dateOfBirth|出生日期|birth': 'dateOfBirth',
    'gender|性别|sex': 'gender',
    'nationality|国籍|民族': 'nationality',
  },
  'passport': {
    'country|国家|签发国': 'country',
    'countryCode|代码': 'countryCode',
    'number|号码|编号|passport': 'number',
    'issueDate|签发日期|date of issue': 'issueDate',
    'placeOfIssue|签发地点': 'placeOfIssue',
    'expiryDate|有效期|date of expiry': 'expiryDate',
    'holderName|持有人|姓名': 'holderName',
    'dateOfBirth|出生日期': 'dateOfBirth',
    'placeOfBirth|出生地': 'placeOfBirth',
    'sex|性别': 'sex',
    'nationality|国籍': 'nationality',
    'authority|签发机关': 'authority',
  },
  'education': {
    'institution|学校|大学|院校|university|college|school': 'institution',
    'degree|学位|学历': 'degree',
    'field|专业|领域|major': 'field',
    'startDate|开始日期|入学': 'startDate',
    'endDate|结束日期|毕业': 'endDate',
  },
  'bankAccount': {
    'bankName|银行名称|开户行': 'bankName',
    'accountNumber|账号|账户': 'accountNumber',
    'currency|货币': 'currency',
    'swiftBic|swift|bic': 'swiftBic',
    'sortCode|sort': 'sortCode',
    'accountHolderName|持有人': 'accountHolderName',
    'routingNumber|routing': 'routingNumber',
  },
  'idCard': {
    'number|号码|编号|id': 'number',
    'holderName|持有人|姓名|name': 'holderName',
    'country|国家|签发国': 'country',
    'dateOfBirth|出生日期|birth': 'dateOfBirth',
    'sex|性别': 'sex',
    'expiryDate|有效期|date of expiry': 'expiryDate',
  },
};

/// Max files per path during scanning.
const int kMaxFilesPerPath = 500;

/// Default max file size in MB.
const int kDefaultMaxFileSizeMb = 10;

/// Per-extension default size limits (MB).
const Map<String, int> kDefaultSizeLimits = {
  '.pdf': 5,
  '.docx': 1,
  '.xlsx': 1,
  '.csv': 1,
  '.json': 1,
  '.txt': 1,
  '.md': 1,
  '.png': 5,
  '.jpg': 5,
  '.jpeg': 5,
  '.webp': 5,
  '.bmp': 5,
  '.tiff': 10,
};

const List<String> kHotPaths = ['Documents', 'Desktop', 'Downloads'];

// =============================================================================
// Fingerprint
// =============================================================================

/// Internal class wrapping a regex fingerprint.
class Fingerprint {
  final RegExp pattern;
  final SensitivityLevel sensitivity;

  const Fingerprint({required this.pattern, required this.sensitivity});
}
