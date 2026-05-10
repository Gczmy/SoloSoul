import 'package:solosoul_flutter/presentation/models/operation_log_models.dart';
import 'package:solosoul_flutter/core/services/operation_notification.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';

/// Utility class for auto-generating operation log entries
/// This provides a clean API for creating descriptive log messages
class OperationLogger {
  /// Log an identity operation
  static OperationEntry logIdentity({
    required LogAction action,
    required String description,
    String? fieldPath,
    SensitivityLevel sensitivityLevel = SensitivityLevel.public,
  }) {
    return OperationEntry(
      timestamp: DateTime.now(),
      action: action.value,
      section: LogSection.identity.value,
      description: description,
      fieldPath: fieldPath,
      sensitivityLevel: sensitivityLevel,
    );
  }

  /// Log a contact information operation
  static OperationEntry logContactInformation({
    required LogAction action,
    required String description,
    String? fieldPath,
    SensitivityLevel sensitivityLevel = SensitivityLevel.public,
  }) {
    return OperationEntry(
      timestamp: DateTime.now(),
      action: action.value,
      section: LogSection.contactInformation.value,
      description: description,
      fieldPath: fieldPath,
      sensitivityLevel: sensitivityLevel,
    );
  }

  /// Log an address operation
  static OperationEntry logAddress({
    required LogAction action,
    required String description,
    String? fieldPath,
    SensitivityLevel sensitivityLevel = SensitivityLevel.public,
  }) {
    return OperationEntry(
      timestamp: DateTime.now(),
      action: action.value,
      section: LogSection.address.value,
      description: description,
      fieldPath: fieldPath,
      sensitivityLevel: sensitivityLevel,
    );
  }

  /// Log an ID card operation
  static OperationEntry logIdCard({
    required LogAction action,
    required String description,
    String? fieldPath,
    SensitivityLevel sensitivityLevel = SensitivityLevel.critical,
  }) {
    return OperationEntry(
      timestamp: DateTime.now(),
      action: action.value,
      section: LogSection.idCard.value,
      description: description,
      fieldPath: fieldPath,
      sensitivityLevel: sensitivityLevel,
    );
  }

  /// Log a passport operation
  static OperationEntry logPassport({
    required LogAction action,
    required String description,
    String? fieldPath,
    SensitivityLevel sensitivityLevel = SensitivityLevel.critical,
  }) {
    return OperationEntry(
      timestamp: DateTime.now(),
      action: action.value,
      section: LogSection.passport.value,
      description: description,
      fieldPath: fieldPath,
      sensitivityLevel: sensitivityLevel,
    );
  }

  /// Log a visa operation
  static OperationEntry logVisa({
    required LogAction action,
    required String description,
    String? fieldPath,
    SensitivityLevel sensitivityLevel = SensitivityLevel.critical,
  }) {
    return OperationEntry(
      timestamp: DateTime.now(),
      action: action.value,
      section: LogSection.visa.value,
      description: description,
      fieldPath: fieldPath,
      sensitivityLevel: sensitivityLevel,
    );
  }

  /// Log a travel history operation
  static OperationEntry logTravelHistory({
    required LogAction action,
    required String description,
    String? fieldPath,
    SensitivityLevel sensitivityLevel = SensitivityLevel.internal,
  }) {
    return OperationEntry(
      timestamp: DateTime.now(),
      action: action.value,
      section: LogSection.travelHistory.value,
      description: description,
      fieldPath: fieldPath,
      sensitivityLevel: sensitivityLevel,
    );
  }

  /// Log a bank account operation
  static OperationEntry logBankAccount({
    required LogAction action,
    required String description,
    String? fieldPath,
    SensitivityLevel sensitivityLevel = SensitivityLevel.critical,
  }) {
    return OperationEntry(
      timestamp: DateTime.now(),
      action: action.value,
      section: LogSection.bankAccount.value,
      description: description,
      fieldPath: fieldPath,
      sensitivityLevel: sensitivityLevel,
    );
  }

  /// Log a card operation
  static OperationEntry logCard({
    required LogAction action,
    required String description,
    String? fieldPath,
    SensitivityLevel sensitivityLevel = SensitivityLevel.critical,
  }) {
    return OperationEntry(
      timestamp: DateTime.now(),
      action: action.value,
      section: LogSection.card.value,
      description: description,
      fieldPath: fieldPath,
      sensitivityLevel: sensitivityLevel,
    );
  }

  /// Log an education operation
  static OperationEntry logEducation({
    required LogAction action,
    required String description,
    String? fieldPath,
    SensitivityLevel sensitivityLevel = SensitivityLevel.sensitive,
  }) {
    return OperationEntry(
      timestamp: DateTime.now(),
      action: action.value,
      section: LogSection.education.value,
      description: description,
      fieldPath: fieldPath,
      sensitivityLevel: sensitivityLevel,
    );
  }

  /// Log an employment operation
  static OperationEntry logEmployment({
    required LogAction action,
    required String description,
    String? fieldPath,
    SensitivityLevel sensitivityLevel = SensitivityLevel.sensitive,
  }) {
    return OperationEntry(
      timestamp: DateTime.now(),
      action: action.value,
      section: LogSection.employment.value,
      description: description,
      fieldPath: fieldPath,
      sensitivityLevel: sensitivityLevel,
    );
  }

  /// Log a skill operation
  static OperationEntry logSkill({
    required LogAction action,
    required String description,
    String? fieldPath,
    SensitivityLevel sensitivityLevel = SensitivityLevel.public,
  }) {
    return OperationEntry(
      timestamp: DateTime.now(),
      action: action.value,
      section: LogSection.skill.value,
      description: description,
      fieldPath: fieldPath,
      sensitivityLevel: sensitivityLevel,
    );
  }

  /// Log a language operation
  static OperationEntry logLanguage({
    required LogAction action,
    required String description,
    String? fieldPath,
    SensitivityLevel sensitivityLevel = SensitivityLevel.public,
  }) {
    return OperationEntry(
      timestamp: DateTime.now(),
      action: action.value,
      section: LogSection.language.value,
      description: description,
      fieldPath: fieldPath,
      sensitivityLevel: sensitivityLevel,
    );
  }

  /// Log a travel operation
  static OperationEntry logTravel({
    required LogAction action,
    required String description,
    String? fieldPath,
    SensitivityLevel sensitivityLevel = SensitivityLevel.public,
  }) {
    return OperationEntry(
      timestamp: DateTime.now(),
      action: action.value,
      section: LogSection.travel.value,
      description: description,
      fieldPath: fieldPath,
      sensitivityLevel: sensitivityLevel,
    );
  }

  /// Log a financial operation
  static OperationEntry logFinancial({
    required LogAction action,
    required String description,
    String? fieldPath,
    SensitivityLevel sensitivityLevel = SensitivityLevel.sensitive,
  }) {
    return OperationEntry(
      timestamp: DateTime.now(),
      action: action.value,
      section: LogSection.financial.value,
      description: description,
      fieldPath: fieldPath,
      sensitivityLevel: sensitivityLevel,
    );
  }

  /// Log a professional operation
  static OperationEntry logProfessional({
    required LogAction action,
    required String description,
    String? fieldPath,
    SensitivityLevel sensitivityLevel = SensitivityLevel.sensitive,
  }) {
    return OperationEntry(
      timestamp: DateTime.now(),
      action: action.value,
      section: LogSection.professional.value,
      description: description,
      fieldPath: fieldPath,
      sensitivityLevel: sensitivityLevel,
    );
  }

  /// Log a sensitivity settings operation
  static OperationEntry logSensitivitySettings({
    required LogAction action,
    required String description,
    String? fieldPath,
    SensitivityLevel sensitivityLevel = SensitivityLevel.sensitive,
    String? descriptionKey,
    Map<String, String>? descriptionArgs,
  }) {
    return OperationEntry(
      timestamp: DateTime.now(),
      action: action.value,
      section: LogSection.sensitivitySettings.value,
      description: description,
      fieldPath: fieldPath,
      sensitivityLevel: sensitivityLevel,
      descriptionKey: descriptionKey,
      descriptionArgs: descriptionArgs,
    );
  }

  /// Log a custom section operation (for unified objects and dynamic sections).
  static OperationEntry logCustomSection({
    required String section,
    required LogAction action,
    required String description,
    String? fieldPath,
    SensitivityLevel sensitivityLevel = SensitivityLevel.public,
    Map<String, String>? properties,
    Map<String, String>? propertyLevels,
    String? descriptionKey,
    Map<String, String>? descriptionArgs,
  }) {
    return OperationEntry(
      timestamp: DateTime.now(),
      action: action.value,
      section: section,
      description: description,
      fieldPath: fieldPath,
      sensitivityLevel: sensitivityLevel,
      properties: properties,
      propertyLevels: propertyLevels,
      descriptionKey: descriptionKey,
      descriptionArgs: descriptionArgs,
    );
  }

  /// Auto-detect action by comparing old and new values
  /// Returns create if old was null, delete if new is null, otherwise update
  static LogAction detectAction<T>(T? oldValue, T? newValue) {
    if (oldValue == null && newValue != null) {
      return LogAction.create;
    } else if (oldValue != null && newValue == null) {
      return LogAction.delete;
    } else {
      return LogAction.update;
    }
  }

  /// Convert LogAction to OperationType
  static OperationType toOperationType(LogAction action) {
    switch (action) {
      case LogAction.create:
        return OperationType.create;
      case LogAction.update:
        return OperationType.update;
      case LogAction.delete:
        return OperationType.delete;
      case LogAction.restore:
        return OperationType.restore;
      case LogAction.purge:
        return OperationType.purge;
    }
  }

  /// Generate an OperationMessage for notification
  static OperationMessage createNotification({
    required LogSection section,
    required LogAction action,
    required String itemName,
    String? fieldName,
    bool isPrivacyModeActive = false,
  }) {
    return OperationMessage(
      type: toOperationType(action),
      section: section.value,
      itemName: itemName,
      fieldName: fieldName,
      isPrivacyModeActive: isPrivacyModeActive,
    );
  }

  static OperationMessage createNotificationForSection({
    required String section,
    required LogAction action,
    required String itemName,
    String? fieldName,
    bool isPrivacyModeActive = false,
  }) {
    return OperationMessage(
      type: toOperationType(action),
      section: section,
      itemName: itemName,
      fieldName: fieldName,
      isPrivacyModeActive: isPrivacyModeActive,
    );
  }

  /// Generate a human-readable description for an identity operation
  static String generateIdentityDescription({
    required LogAction action,
    String? fieldPath,
    String? itemName,
  }) {
    final fieldLabel = _getFieldLabel(fieldPath);
    switch (action) {
      case LogAction.create:
        return 'Added ${itemName ?? fieldLabel}';
      case LogAction.update:
        return 'Updated ${itemName ?? fieldLabel}';
      case LogAction.delete:
        return 'Deleted ${itemName ?? fieldLabel}';
      case LogAction.restore:
        return 'Restored ${itemName ?? fieldLabel}';
      case LogAction.purge:
        return 'Permanently deleted ${itemName ?? fieldLabel}';
    }
  }

  /// Generate a human-readable description for a travel operation
  static String generateTravelDescription({
    required LogAction action,
    required String itemType,
    String? itemName,
  }) {
    switch (action) {
      case LogAction.create:
        return 'Added $itemType${itemName != null ? ': $itemName' : ''}';
      case LogAction.update:
        return 'Updated $itemType${itemName != null ? ': $itemName' : ''}';
      case LogAction.delete:
        return 'Deleted $itemType${itemName != null ? ': $itemName' : ''}';
      case LogAction.restore:
        return 'Restored $itemType${itemName != null ? ': $itemName' : ''}';
      case LogAction.purge:
        return 'Permanently deleted $itemType${itemName != null ? ': $itemName' : ''}';
    }
  }

  /// Generate a human-readable description for a financial operation
  static String generateFinancialDescription({
    required LogAction action,
    required String itemType,
    String? itemName,
  }) {
    switch (action) {
      case LogAction.create:
        return 'Added $itemType${itemName != null ? ': $itemName' : ''}';
      case LogAction.update:
        return 'Updated $itemType${itemName != null ? ': $itemName' : ''}';
      case LogAction.delete:
        return 'Deleted $itemType${itemName != null ? ': $itemName' : ''}';
      case LogAction.restore:
        return 'Restored $itemType${itemName != null ? ': $itemName' : ''}';
      case LogAction.purge:
        return 'Permanently deleted $itemType${itemName != null ? ': $itemName' : ''}';
    }
  }

  /// Generate a human-readable description for a professional operation
  static String generateProfessionalDescription({
    required LogAction action,
    required String itemType,
    String? itemName,
  }) {
    switch (action) {
      case LogAction.create:
        return 'Added $itemType${itemName != null ? ': $itemName' : ''}';
      case LogAction.update:
        return 'Updated $itemType${itemName != null ? ': $itemName' : ''}';
      case LogAction.delete:
        return 'Deleted $itemType${itemName != null ? ': $itemName' : ''}';
      case LogAction.restore:
        return 'Restored $itemType${itemName != null ? ': $itemName' : ''}';
      case LogAction.purge:
        return 'Permanently deleted $itemType${itemName != null ? ': $itemName' : ''}';
    }
  }

  /// Get a readable field label from field path
  static String _getFieldLabel(String? fieldPath) {
    if (fieldPath == null || fieldPath.isEmpty) return 'field';
    return fieldPath
        .split('.')
        .last
        .replaceAllMapped(
          RegExp(r'([a-z])([A-Z])'),
          (match) => '${match.group(1)} ${match.group(2)}',
        )
        .toLowerCase();
  }
}
