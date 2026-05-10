import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/core/models/scan/scan_result_model.dart';
import 'package:solosoul_flutter/core/services/scan/cancel_token.dart';

// =============================================================================
// Section Detector
// =============================================================================

/// Handles section detection and field extraction from text content.
class ScanSectionDetector {
  /// Check if filename hints at personal information.
  static bool filenameHintsPersonal(String filename) {
    final lower = filename.toLowerCase();
    return kFilenameKeywords.any((kw) => lower.contains(kw.toLowerCase()));
  }

  /// Detect sections from filename only.
  static List<ScanSection> detectSectionsFromFilename(String filename) {
    final lower = filename.toLowerCase();
    final sections = <ScanSection>[];

    if (lower.contains('resume') || lower.contains('cv') || lower.contains('简历')) {
      sections.add(const ScanSection(
        section: 'identity',
        display: 'Personal Information',
        fields: [],
      ));
      sections.add(const ScanSection(
        section: 'education',
        display: 'Education',
        fields: [],
      ));
    }
    if (lower.contains('passport') || lower.contains('护照')) {
      sections.add(const ScanSection(
        section: 'passport',
        display: 'Passport',
        fields: [],
      ));
    }
    if (lower.contains('bank') || lower.contains('银行')) {
      sections.add(const ScanSection(
        section: 'bankAccount',
        display: 'Bank Account',
        fields: [],
      ));
    }

    return sections;
  }

  /// Detect sections from full text content.
  static List<ScanSection> detectSections(String text) {
    final sections = <String, ScanSection>{};
    final lowerText = text.toLowerCase();

    // Detect which sections are present based on keywords
    for (final entry in kSectionKeywords.entries) {
      final sectionId = entry.key;
      final keywords = entry.value;

      final matched = keywords.any((kw) => lowerText.contains(kw.toLowerCase()));
      if (!matched) continue;

      final fields = <ScanField>[];

      // Run fingerprints for this section
      if (sectionId == 'identity') {
        fields.addAll(extractIdentityFields(text));
      } else if (sectionId == 'passport') {
        fields.addAll(extractPassportFields(text));
      } else if (sectionId == 'education') {
        fields.addAll(extractEducationFields(text));
      } else if (sectionId == 'bankAccount') {
        fields.addAll(extractBankAccountFields(text));
      } else if (sectionId == 'contact') {
        fields.addAll(extractContactFields(text));
      } else if (sectionId == 'employment') {
        fields.addAll(extractEmploymentFields(text));
      }

      if (fields.isNotEmpty) {
        sections[sectionId] = ScanSection(
          section: sectionId,
          display: sectionDisplayName(sectionId),
          fields: fields,
        );
      }
    }

    return sections.values.toList();
  }

  /// Get display name for a section.
  static String sectionDisplayName(String sectionId) {
    const names = {
      'identity': 'Personal Information',
      'contact': 'Contact',
      'education': 'Education',
      'passport': 'Passport',
      'visa': 'Visa',
      'bankAccount': 'Bank Account',
      'card': 'Card',
      'employment': 'Employment',
    };
    return names[sectionId] ?? sectionId;
  }

  // ---------------------------------------------------------------------------
  // Field Extractors
  // ---------------------------------------------------------------------------

  static List<ScanField> extractIdentityFields(String text) {
    final fields = <ScanField>[];

    // ID Card
    final idMatch = kFingerprints['id_card']!.pattern.firstMatch(text);
    if (idMatch != null) {
      fields.add(ScanField(
        key: 'idCard',
        value: idMatch.group(0)!,
        sensitivity: SensitivityLevel.critical,
        confidence: 0.99,
      ));
    }

    // Phone
    final phoneMatches = kFingerprints['phone']!.pattern.allMatches(text);
    if (phoneMatches.isNotEmpty) {
      fields.add(ScanField(
        key: 'phone',
        value: phoneMatches.first.group(0)!,
        sensitivity: SensitivityLevel.sensitive,
        confidence: 0.98,
      ));
    }

    // Email
    final emailMatches = kFingerprints['email']!.pattern.allMatches(text);
    if (emailMatches.isNotEmpty) {
      fields.add(ScanField(
        key: 'email',
        value: emailMatches.first.group(0)!,
        sensitivity: SensitivityLevel.internal,
        confidence: 0.97,
      ));
    }

    // Try to extract name (heuristic: look for patterns like "Name: Zhang San")
    final nameMatch = RegExp(r'[Nn]ame[\s:：]+([一-龥]{2,4}|[A-Z][a-z]+\s[A-Z][a-z]+)').firstMatch(text);
    if (nameMatch != null) {
      fields.add(ScanField(
        key: 'fullName',
        value: nameMatch.group(1)!,
        sensitivity: SensitivityLevel.public,
        confidence: 0.85,
      ));
    }

    return fields;
  }

  static List<ScanField> extractPassportFields(String text) {
    final fields = <ScanField>[];

    final passportMatch = kFingerprints['passport']!.pattern.firstMatch(text);
    if (passportMatch != null) {
      fields.add(ScanField(
        key: 'number',
        value: passportMatch.group(0)!,
        sensitivity: SensitivityLevel.critical,
        confidence: 0.95,
      ));
    }

    // Country (heuristic)
    final countryMatch = RegExp(r'[Cc]ountry[\s:：]+([A-Za-z一-龥 ]{2,30})').firstMatch(text);
    if (countryMatch != null) {
      fields.add(ScanField(
        key: 'country',
        value: countryMatch.group(1)!.trim(),
        sensitivity: SensitivityLevel.public,
        confidence: 0.80,
      ));
    }

    // Holder name
    final holderMatch = RegExp(r'[Nn]ame[\s:：]+([一-龥]{2,4}|[A-Z][a-z]+\s[A-Z][a-z]+)').firstMatch(text);
    if (holderMatch != null) {
      fields.add(ScanField(
        key: 'holderName',
        value: holderMatch.group(1)!,
        sensitivity: SensitivityLevel.sensitive,
        confidence: 0.80,
      ));
    }

    return fields;
  }

  static List<ScanField> extractEducationFields(String text) {
    final fields = <ScanField>[];

    // Institution (heuristic)
    final instMatch = RegExp(
      r'([一-龥]{2,10}(?:大学|学院|学校)|[A-Z][a-zA-Z\s]+(?:University|College|Institute|School))',
    ).firstMatch(text);
    if (instMatch != null) {
      fields.add(ScanField(
        key: 'institution',
        value: instMatch.group(1)!,
        sensitivity: SensitivityLevel.public,
        confidence: 0.75,
      ));
    }

    // Degree
    final degreeMatch = RegExp(
      r'(Bachelor|Master|Ph\.?D|博士|硕士|学士|本科|研究生)',
      caseSensitive: false,
    ).firstMatch(text);
    if (degreeMatch != null) {
      fields.add(ScanField(
        key: 'degree',
        value: degreeMatch.group(1)!,
        sensitivity: SensitivityLevel.public,
        confidence: 0.70,
      ));
    }

    return fields;
  }

  static List<ScanField> extractBankAccountFields(String text) {
    final fields = <ScanField>[];

    // Bank name heuristic
    final bankMatch = RegExp(
      r'([一-龥]{2,8}银行|[A-Z][a-zA-Z\s]+Bank)',
    ).firstMatch(text);
    if (bankMatch != null) {
      fields.add(ScanField(
        key: 'bankName',
        value: bankMatch.group(1)!,
        sensitivity: SensitivityLevel.sensitive,
        confidence: 0.80,
      ));
    }

    // Account number (generic 16-19 digit pattern)
    final acctMatch = RegExp(r'\b\d{16,19}\b').firstMatch(text);
    if (acctMatch != null) {
      fields.add(ScanField(
        key: 'accountNumber',
        value: acctMatch.group(0)!,
        sensitivity: SensitivityLevel.critical,
        confidence: 0.90,
      ));
    }

    // SWIFT/BIC
    final swiftMatch = RegExp(r'\b[A-Z]{6}[A-Z0-9]{2}([A-Z0-9]{3})?\b').firstMatch(text);
    if (swiftMatch != null) {
      fields.add(ScanField(
        key: 'swiftBic',
        value: swiftMatch.group(0)!,
        sensitivity: SensitivityLevel.critical,
        confidence: 0.85,
      ));
    }

    return fields;
  }

  static List<ScanField> extractContactFields(String text) {
    final fields = <ScanField>[];

    final phoneMatches = kFingerprints['phone']!.pattern.allMatches(text);
    if (phoneMatches.isNotEmpty) {
      fields.add(ScanField(
        key: 'value',
        value: phoneMatches.first.group(0)!,
        sensitivity: SensitivityLevel.internal,
        confidence: 0.95,
      ));
    }

    final emailMatches = kFingerprints['email']!.pattern.allMatches(text);
    if (emailMatches.isNotEmpty) {
      fields.add(ScanField(
        key: 'value',
        value: emailMatches.first.group(0)!,
        sensitivity: SensitivityLevel.internal,
        confidence: 0.95,
      ));
    }

    return fields;
  }

  static List<ScanField> extractEmploymentFields(String text) {
    final fields = <ScanField>[];

    final companyMatch = RegExp(
      r'([一-龥]{2,20}(?:公司|集团|企业)|[A-Z][a-zA-Z0-9\s&]+(?:Inc|Ltd|LLC|Corp|Company))',
    ).firstMatch(text);
    if (companyMatch != null) {
      fields.add(ScanField(
        key: 'company',
        value: companyMatch.group(1)!,
        sensitivity: SensitivityLevel.public,
        confidence: 0.70,
      ));
    }

    return fields;
  }
}
