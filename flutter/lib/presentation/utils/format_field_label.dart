/// 将字段 key（camelCase / snake_case）格式化为人类可读的 Title Case。
///
/// 例如：
/// - "givenName" → "Given Name"
/// - "dateOfBirth" → "Date Of Birth"
/// - "visa_type" → "Visa Type"
String formatFieldLabel(String key) {
  final spaced = key.replaceAllMapped(
    RegExp(r'([a-z])([A-Z])'),
    (m) => '${m[1]} ${m[2]}',
  );
  return spaced.replaceAll('_', ' ').split(' ').map((word) {
    if (word.isEmpty) return word;
    return word[0].toUpperCase() + word.substring(1).toLowerCase();
  }).join(' ');
}
