import 'dart:io';

import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';

enum LogSection {
  identity('identity'),
  contactInformation('contact information'),
  address('address'),
  idCard('ID card'),
  passport('passport'),
  visa('visa'),
  travelHistory('travel history'),
  bankAccount('bank account'),
  card('card'),
  education('education'),
  employment('employment'),
  skill('skill'),
  language('language'),
  travel('travel'),
  financial('financial'),
  professional('professional'),
  sensitivitySettings('sensitivity settings');

  final String value;
  const LogSection(this.value);
}

// Action types for operation logs
enum LogAction {
  create('create'),
  update('update'),
  delete('delete'),
  restore('restore'),
  purge('purge');

  final String value;
  const LogAction(this.value);
}

// Device/Platform types for operation logs
enum LogDevice {
  macos('macOS'),
  ios('iOS'),
  android('Android'),
  windows('Windows'),
  linux('Linux'),
  web('Web'),
  unknown('Unknown');

  final String value;
  const LogDevice(this.value);

  static LogDevice get current {
    return fromString(Platform.operatingSystem);
  }

  static LogDevice fromString(String value) {
    switch (value.toLowerCase()) {
      case 'macos':
        return LogDevice.macos;
      case 'ios':
        return LogDevice.ios;
      case 'android':
        return LogDevice.android;
      case 'windows':
        return LogDevice.windows;
      case 'linux':
        return LogDevice.linux;
      case 'web':
        return LogDevice.web;
      default:
        return LogDevice.unknown;
    }
  }
}

/// Operation log entry model
/// NOTE: The description field should NOT contain sensitive plain text.
/// Example: Use "Modified password field" not "Changed password to 123456"
class OperationEntry {
  final DateTime timestamp;
  final String action; // 'create', 'update', 'delete'
  final String section; // 'identity', 'travel', 'financial', 'professional'
  final String description;
  final String? fieldPath; // Optional field path for more details
  final String device; // Platform: 'macos', 'ios', 'android', etc.
  final SensitivityLevel sensitivityLevel;

  /// Snapshot of property values at the time of the operation (e.g. for purge).
  /// Key: property name, Value: property value string.
  final Map<String, String>? properties;

  /// Snapshot of property sensitivity levels at the time of the operation.
  /// Key: property name, Value: sensitivity level name.
  final Map<String, String>? propertyLevels;

  /// i18n key for rendering description dynamically (enables language switching).
  /// When set, [localizedDescription] uses this key instead of the raw [description].
  final String? descriptionKey;

  /// Arguments for [descriptionKey] interpolation.
  /// Key: param name, Value: param value.
  final Map<String, String>? descriptionArgs;

  const OperationEntry({
    required this.timestamp,
    required this.action,
    required this.section,
    required this.description,
    this.fieldPath,
    this.device = 'unknown',
    this.sensitivityLevel = SensitivityLevel.public,
    this.properties,
    this.propertyLevels,
    this.descriptionKey,
    this.descriptionArgs,
  });

  /// Render the description using the current locale.
  /// Falls back to the stored [description] for backward compatibility with
  /// older log entries that lack structured i18n data.
  String localizedDescription(AppLocalizations l10n) {
    final key = descriptionKey;
    final args = descriptionArgs;
    if (key == null || args == null) return description;

    return _renderLocalized(key, args, l10n, description);
  }

  static String _renderLocalized(
    String key,
    Map<String, String> args,
    AppLocalizations l10n,
    String fallbackDescription,
  ) {
    return switch (key) {
      'createdUnifiedItem' => l10n.operationLogCreatedItem(args['name'] ?? ''),
      'updatedUnifiedItem' => l10n.operationLogUpdatedItem(args['name'] ?? ''),
      'deletedUnifiedItem' => l10n.operationLogDeletedItem(args['name'] ?? ''),
      'deletedPredefinedItem' => l10n.predefinedDeletedItem(args['title'] ?? '', args['name'] ?? ''),
      'restoredUnifiedItem' => l10n.operationLogRestoredItem(args['name'] ?? ''),
      'purgedUnifiedItem' => l10n.trashPermanentDeletedItem(args['name'] ?? ''),
      'restoredTrashItem' => l10n.trashRestoredItem(args['name'] ?? ''),
      'sensitivitySet' => _sensitivityDesc(args, l10n),
      'sensitivityChanged' => _sensitivityDesc(args, l10n),
      'sensitivityReverted' => _sensitivityDesc(args, l10n),
      'sensitivityUpgraded' => _sensitivityDesc(args, l10n),
      'sensitivityDowngraded' => _sensitivityDesc(args, l10n),
      _ => fallbackDescription,
    };
  }

  static String _sensitivityDesc(Map<String, String> args, AppLocalizations l10n) {
    final field = args['field'] ?? '';
    final oldLevel = args['oldLevel'];
    final newLevel = args['newLevel'];
    if (oldLevel != null && newLevel != null) {
      return l10n.operationLogSensitivityChanged(field, oldLevel, newLevel);
    }
    if (newLevel != null) {
      return l10n.operationLogSensitivitySet(field, newLevel);
    }
    if (oldLevel != null) {
      return l10n.operationLogSensitivityReverted(field, oldLevel);
    }
    return field;
  }

  factory OperationEntry.fromJson(Map<String, dynamic> json) {
    return OperationEntry(
      timestamp: DateTime.parse(json['timestamp'] as String),
      action: json['action'] as String,
      section: json['section'] as String,
      description: json['description'] as String,
      fieldPath: json['fieldPath'] as String?,
      device: json['device'] as String? ?? 'unknown',
      sensitivityLevel: SensitivityLevel.values.firstWhere(
        (e) => e.name == json['sensitivityLevel'],
        orElse: () => SensitivityLevel.public,
      ),
      properties: (json['properties'] as Map<String, dynamic>?)?.cast<String, String>(),
      propertyLevels: (json['propertyLevels'] as Map<String, dynamic>?)?.cast<String, String>(),
      descriptionKey: json['descriptionKey'] as String?,
      descriptionArgs: (json['descriptionArgs'] as Map<String, dynamic>?)?.cast<String, String>(),
    );
  }

  Map<String, dynamic> toJson() => {
        'timestamp': timestamp.toIso8601String(),
        'action': action,
        'section': section,
        'description': description,
        if (fieldPath != null) 'fieldPath': fieldPath,
        'device': device,
        'sensitivityLevel': sensitivityLevel.name,
        if (properties != null && properties!.isNotEmpty) 'properties': properties,
        if (propertyLevels != null && propertyLevels!.isNotEmpty) 'propertyLevels': propertyLevels,
        if (descriptionKey != null) 'descriptionKey': descriptionKey,
        if (descriptionArgs != null && descriptionArgs!.isNotEmpty) 'descriptionArgs': descriptionArgs,
      };
}
