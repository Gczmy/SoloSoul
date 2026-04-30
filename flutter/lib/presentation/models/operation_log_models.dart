import 'dart:io';

import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
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
  });

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
      };
}
