import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/presentation/models/operation_log_models.dart';

void main() {
  group('LogDevice', () {
    test('fromString maps platform names', () {
      expect(LogDevice.fromString('macos'), LogDevice.macos);
      expect(LogDevice.fromString('ios'), LogDevice.ios);
      expect(LogDevice.fromString('android'), LogDevice.android);
      expect(LogDevice.fromString('windows'), LogDevice.windows);
      expect(LogDevice.fromString('linux'), LogDevice.linux);
      expect(LogDevice.fromString('web'), LogDevice.web);
    });

    test('fromString is case insensitive', () {
      expect(LogDevice.fromString('MacOS'), LogDevice.macos);
      expect(LogDevice.fromString('ANDROID'), LogDevice.android);
    });

    test('fromString returns unknown for unrecognized', () {
      expect(LogDevice.fromString('unknown_os'), LogDevice.unknown);
    });

    test('values are correct', () {
      expect(LogDevice.macos.value, 'macOS');
      expect(LogDevice.ios.value, 'iOS');
      expect(LogDevice.unknown.value, 'Unknown');
    });
  });

  group('LogAction', () {
    test('values are correct', () {
      expect(LogAction.create.value, 'create');
      expect(LogAction.update.value, 'update');
      expect(LogAction.delete.value, 'delete');
      expect(LogAction.restore.value, 'restore');
      expect(LogAction.purge.value, 'purge');
    });
  });

  group('LogSection', () {
    test('values are correct', () {
      expect(LogSection.identity.value, 'identity');
      expect(LogSection.passport.value, 'passport');
      expect(LogSection.bankAccount.value, 'bank account');
    });
  });

  group('OperationEntry', () {
    test('fromJson and toJson round-trip', () {
      final entry = OperationEntry(
        timestamp: DateTime.parse('2024-01-01T00:00:00Z'),
        action: 'create',
        section: 'identity',
        description: 'Created item',
        device: 'macos',
        sensitivityLevel: SensitivityLevel.sensitive,
      );

      final json = entry.toJson();
      final restored = OperationEntry.fromJson(json);

      expect(restored.action, entry.action);
      expect(restored.section, entry.section);
      expect(restored.description, entry.description);
      expect(restored.device, entry.device);
      expect(restored.sensitivityLevel, entry.sensitivityLevel);
    });

    test('fromJson handles optional fields', () {
      final json = {
        'timestamp': '2024-01-01T00:00:00Z',
        'action': 'update',
        'section': 'passport',
        'description': 'Updated',
        'fieldPath': 'passport.number',
        'properties': {'number': '12345'},
        'descriptionKey': 'updatedUnifiedItem',
        'descriptionArgs': {'name': 'Passport'},
      };

      final entry = OperationEntry.fromJson(json);
      expect(entry.fieldPath, 'passport.number');
      expect(entry.properties, {'number': '12345'});
      expect(entry.descriptionKey, 'updatedUnifiedItem');
      expect(entry.descriptionArgs, {'name': 'Passport'});
    });

    test('fromJson defaults device to unknown', () {
      final json = {
        'timestamp': '2024-01-01T00:00:00Z',
        'action': 'delete',
        'section': 'identity',
        'description': 'Deleted',
      };

      final entry = OperationEntry.fromJson(json);
      expect(entry.device, 'unknown');
      expect(entry.sensitivityLevel, SensitivityLevel.public);
    });

    test('toJson omits null/empty optional fields', () {
      final entry = OperationEntry(
        timestamp: DateTime.parse('2024-01-01T00:00:00Z'),
        action: 'create',
        section: 'identity',
        description: 'Created',
      );

      final json = entry.toJson();
      expect(json.containsKey('fieldPath'), isFalse);
      expect(json.containsKey('properties'), isFalse);
      expect(json.containsKey('propertyLevels'), isFalse);
      expect(json.containsKey('descriptionKey'), isFalse);
      expect(json.containsKey('descriptionArgs'), isFalse);
    });

    test('toJson includes non-empty optional fields', () {
      final entry = OperationEntry(
        timestamp: DateTime.parse('2024-01-01T00:00:00Z'),
        action: 'create',
        section: 'identity',
        description: 'Created',
        fieldPath: 'identity.name',
        properties: {'name': 'John'},
        propertyLevels: {'name': 'public'},
        descriptionKey: 'createdUnifiedItem',
        descriptionArgs: {'name': 'Identity'},
      );

      final json = entry.toJson();
      expect(json['fieldPath'], 'identity.name');
      expect(json['properties'], {'name': 'John'});
      expect(json['propertyLevels'], {'name': 'public'});
      expect(json['descriptionKey'], 'createdUnifiedItem');
      expect(json['descriptionArgs'], {'name': 'Identity'});
    });
  });
}
