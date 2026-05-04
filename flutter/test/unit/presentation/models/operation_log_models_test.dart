import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/presentation/models/operation_log_models.dart';

void main() {
  group('LogSection', () {
    test('has correct string values', () {
      expect(LogSection.identity.value, 'identity');
      expect(LogSection.contactInformation.value, 'contact information');
      expect(LogSection.passport.value, 'passport');
      expect(LogSection.bankAccount.value, 'bank account');
    });
  });

  group('LogAction', () {
    test('has correct string values', () {
      expect(LogAction.create.value, 'create');
      expect(LogAction.update.value, 'update');
      expect(LogAction.delete.value, 'delete');
      expect(LogAction.restore.value, 'restore');
      expect(LogAction.purge.value, 'purge');
    });
  });

  group('LogDevice', () {
    test('fromString handles known platforms', () {
      expect(LogDevice.fromString('macos'), LogDevice.macos);
      expect(LogDevice.fromString('macOS'), LogDevice.macos);
      expect(LogDevice.fromString('ios'), LogDevice.ios);
      expect(LogDevice.fromString('android'), LogDevice.android);
      expect(LogDevice.fromString('windows'), LogDevice.windows);
      expect(LogDevice.fromString('linux'), LogDevice.linux);
      expect(LogDevice.fromString('web'), LogDevice.web);
    });

    test('fromString returns unknown for unrecognized input', () {
      expect(LogDevice.fromString('unknown'), LogDevice.unknown);
      expect(LogDevice.fromString(''), LogDevice.unknown);
      expect(LogDevice.fromString('tablet'), LogDevice.unknown);
    });

    test('has correct string values', () {
      expect(LogDevice.macos.value, 'macOS');
      expect(LogDevice.ios.value, 'iOS');
      expect(LogDevice.android.value, 'Android');
    });
  });

  group('OperationEntry', () {
    final timestamp = DateTime(2024, 6, 15, 10, 30);

    test('creates with required fields', () {
      final entry = OperationEntry(
        timestamp: timestamp,
        action: 'create',
        section: 'identity',
        description: 'Created identity entry',
      );
      expect(entry.timestamp, timestamp);
      expect(entry.action, 'create');
      expect(entry.section, 'identity');
      expect(entry.description, 'Created identity entry');
      expect(entry.device, 'unknown');
      expect(entry.sensitivityLevel, SensitivityLevel.public);
      expect(entry.fieldPath, isNull);
      expect(entry.properties, isNull);
      expect(entry.propertyLevels, isNull);
    });

    test('creates with all fields', () {
      final entry = OperationEntry(
        timestamp: timestamp,
        action: 'update',
        section: 'travel',
        description: 'Updated passport',
        fieldPath: 'passport.number',
        device: 'macOS',
        sensitivityLevel: SensitivityLevel.critical,
        properties: {'number': 'AB123456'},
        propertyLevels: {'number': 'critical'},
      );
      expect(entry.fieldPath, 'passport.number');
      expect(entry.device, 'macOS');
      expect(entry.sensitivityLevel, SensitivityLevel.critical);
      expect(entry.properties, {'number': 'AB123456'});
      expect(entry.propertyLevels, {'number': 'critical'});
    });

    group('JSON serialization', () {
      test('toJson produces correct map', () {
        final entry = OperationEntry(
          timestamp: timestamp,
          action: 'create',
          section: 'identity',
          description: 'Test',
          device: 'macOS',
          sensitivityLevel: SensitivityLevel.sensitive,
        );
        final json = entry.toJson();
        expect(json['timestamp'], timestamp.toIso8601String());
        expect(json['action'], 'create');
        expect(json['section'], 'identity');
        expect(json['description'], 'Test');
        expect(json['device'], 'macOS');
        expect(json['sensitivityLevel'], 'sensitive');
      });

      test('toJson omits null optional fields', () {
        final entry = OperationEntry(
          timestamp: timestamp,
          action: 'create',
          section: 'identity',
          description: 'Test',
        );
        final json = entry.toJson();
        expect(json.containsKey('fieldPath'), isFalse);
        expect(json.containsKey('properties'), isFalse);
        expect(json.containsKey('propertyLevels'), isFalse);
      });

      test('toJson includes properties when non-empty', () {
        final entry = OperationEntry(
          timestamp: timestamp,
          action: 'delete',
          section: 'financial',
          description: 'Purged',
          properties: {'name': 'Savings'},
          propertyLevels: {'name': 'private'},
        );
        final json = entry.toJson();
        expect(json['properties'], {'name': 'Savings'});
        expect(json['propertyLevels'], {'name': 'private'});
      });

      test('fromJson round-trips correctly', () {
        final original = OperationEntry(
          timestamp: timestamp,
          action: 'update',
          section: 'travel',
          description: 'Updated visa',
          fieldPath: 'visa.expiry',
          device: 'iOS',
          sensitivityLevel: SensitivityLevel.critical,
          properties: {'expiry': '2025-12-31'},
          propertyLevels: {'expiry': 'critical'},
        );
        final json = original.toJson();
        final restored = OperationEntry.fromJson(json);

        expect(restored.timestamp, original.timestamp);
        expect(restored.action, original.action);
        expect(restored.section, original.section);
        expect(restored.description, original.description);
        expect(restored.fieldPath, original.fieldPath);
        expect(restored.device, original.device);
        expect(restored.sensitivityLevel, original.sensitivityLevel);
        expect(restored.properties, original.properties);
        expect(restored.propertyLevels, original.propertyLevels);
      });

      test('fromJson handles missing optional fields', () {
        final json = {
          'timestamp': '2024-01-01T00:00:00.000',
          'action': 'create',
          'section': 'identity',
          'description': 'Test',
        };
        final entry = OperationEntry.fromJson(json);
        expect(entry.device, 'unknown');
        expect(entry.sensitivityLevel, SensitivityLevel.public);
        expect(entry.fieldPath, isNull);
      });

      test('fromJson defaults unknown sensitivityLevel to public', () {
        final json = {
          'timestamp': '2024-01-01T00:00:00.000',
          'action': 'create',
          'section': 'identity',
          'description': 'Test',
          'sensitivityLevel': 'nonexistent',
        };
        final entry = OperationEntry.fromJson(json);
        expect(entry.sensitivityLevel, SensitivityLevel.public);
      });
    });
  });
}
