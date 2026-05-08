import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';

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

/// Translate a field key using l10n, falling back to [formatFieldLabel]
/// for unknown keys (e.g. user-defined custom fields).
String translateFieldLabel(String key, AppLocalizations l10n) {
  return switch (key) {
    'fullName' => l10n.fieldFullName,
    'givenName' => l10n.fieldGivenName,
    'familyName' => l10n.fieldFamilyName,
    'dateOfBirth' => l10n.fieldDateOfBirth,
    'gender' => l10n.fieldGender,
    'nationality' => l10n.fieldNationality,
    'title' => l10n.fieldTitle,
    'type' => l10n.fieldType,
    'value' => l10n.fieldValue,
    'number' => l10n.fieldNumber,
    'issueDate' => l10n.fieldIssueDate,
    'expiryDate' => l10n.fieldExpiryDate,
    'holderName' => l10n.fieldHolderName,
    'country' => l10n.fieldCountry,
    'street' => l10n.fieldStreet,
    'city' => l10n.fieldCity,
    'state' => l10n.fieldState,
    'postalCode' => l10n.fieldPostalCode,
    'passportNumber' => l10n.fieldPassportNumber,
    'issuingCountry' => l10n.fieldIssuingCountry,
    'visaNumber' => l10n.fieldVisaNumber,
    'entryDate' => l10n.fieldEntryDate,
    'exitDate' => l10n.fieldExitDate,
    'bankName' => l10n.fieldBankName,
    'accountNumber' => l10n.fieldAccountNumber,
    'swiftCode' => l10n.fieldSwiftCode,
    'iban' => l10n.fieldIban,
    'cardNumber' => l10n.fieldCardNumber,
    'cardholderName' => l10n.fieldCardholderName,
    'cvv' => l10n.fieldCvv,
    'taxIdNumber' => l10n.fieldTaxIdNumber,
    'institution' => l10n.fieldInstitution,
    'degree' => l10n.fieldDegree,
    'fieldOfStudy' => l10n.fieldFieldOfStudy,
    'startDate' => l10n.fieldStartDate,
    'endDate' => l10n.fieldEndDate,
    'company' => l10n.fieldCompany,
    'position' => l10n.fieldPosition,
    'category' => l10n.fieldCategory,
    'level' => l10n.fieldLevel,
    'language' => l10n.fieldLanguage,
    'proficiency' => l10n.fieldProficiency,
    'organization' => l10n.fieldOrganization,
    'phone' => l10n.fieldPhone,
    'email' => l10n.fieldEmail,
    'content' => l10n.fieldContent,
    'done' => l10n.fieldDone,
    'dueDate' => l10n.fieldDueDate,
    _ => formatFieldLabel(key),
  };
}
