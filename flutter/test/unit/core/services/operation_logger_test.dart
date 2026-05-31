import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/core/services/operation_logger.dart';
import 'package:solosoul_flutter/core/services/operation_notification.dart';
import 'package:solosoul_flutter/presentation/models/operation_log_models.dart';

void main() {
  group('OperationLogger.detectAction', () {
    test('returns create when old is null and new is not', () {
      expect(OperationLogger.detectAction<String>(null, 'value'), LogAction.create);
    });

    test('returns delete when old is not null and new is null', () {
      expect(OperationLogger.detectAction<String>('value', null), LogAction.delete);
    });

    test('returns update when both are not null', () {
      expect(OperationLogger.detectAction<String>('old', 'new'), LogAction.update);
    });

    test('returns update when both are null', () {
      expect(OperationLogger.detectAction<String>(null, null), LogAction.update);
    });
  });

  group('OperationLogger.toOperationType', () {
    test('maps all LogAction values correctly', () {
      expect(OperationLogger.toOperationType(LogAction.create), OperationType.create);
      expect(OperationLogger.toOperationType(LogAction.update), OperationType.update);
      expect(OperationLogger.toOperationType(LogAction.delete), OperationType.delete);
      expect(OperationLogger.toOperationType(LogAction.restore), OperationType.restore);
      expect(OperationLogger.toOperationType(LogAction.purge), OperationType.purge);
    });
  });

  group('OperationLogger log methods', () {
    test('logIdentity creates entry with correct section', () {
      final entry = OperationLogger.logIdentity(
        action: LogAction.create,
        description: 'Added name',
      );
      expect(entry.section, 'identity');
      expect(entry.action, 'create');
      expect(entry.description, 'Added name');
      expect(entry.sensitivityLevel, SensitivityLevel.public);
    });

    test('logIdCard defaults to critical sensitivity', () {
      final entry = OperationLogger.logIdCard(
        action: LogAction.update,
        description: 'Updated ID',
      );
      expect(entry.section, 'ID card');
      expect(entry.sensitivityLevel, SensitivityLevel.critical);
    });

    test('logPassport defaults to critical sensitivity', () {
      final entry = OperationLogger.logPassport(
        action: LogAction.create,
        description: 'Added passport',
      );
      expect(entry.section, 'passport');
      expect(entry.sensitivityLevel, SensitivityLevel.critical);
    });

    test('logBankAccount defaults to critical sensitivity', () {
      final entry = OperationLogger.logBankAccount(
        action: LogAction.create,
        description: 'Added account',
      );
      expect(entry.section, 'bank account');
      expect(entry.sensitivityLevel, SensitivityLevel.critical);
    });

    test('logEducation defaults to sensitive sensitivity', () {
      final entry = OperationLogger.logEducation(
        action: LogAction.create,
        description: 'Added degree',
      );
      expect(entry.section, 'education');
      expect(entry.sensitivityLevel, SensitivityLevel.sensitive);
    });

    test('logSkill defaults to public sensitivity', () {
      final entry = OperationLogger.logSkill(
        action: LogAction.create,
        description: 'Added skill',
      );
      expect(entry.section, 'skill');
      expect(entry.sensitivityLevel, SensitivityLevel.public);
    });

    test('logCustomSection includes properties', () {
      final entry = OperationLogger.logCustomSection(
        section: 'custom',
        action: LogAction.update,
        description: 'Updated custom',
        properties: {'key': 'value'},
        propertyLevels: {'key': 'public'},
      );
      expect(entry.section, 'custom');
      expect(entry.properties, {'key': 'value'});
      expect(entry.propertyLevels, {'key': 'public'});
    });

    test('log methods include fieldPath when provided', () {
      final entry = OperationLogger.logIdentity(
        action: LogAction.update,
        description: 'Updated name',
        fieldPath: 'identity.firstName',
      );
      expect(entry.fieldPath, 'identity.firstName');
    });
  });

  group('OperationLogger description generators', () {
    test('generateIdentityDescription for each action', () {
      expect(
        OperationLogger.generateIdentityDescription(
          action: LogAction.create,
          itemName: 'John',
        ),
        'Added John',
      );
      expect(
        OperationLogger.generateIdentityDescription(
          action: LogAction.update,
          itemName: 'John',
        ),
        'Updated John',
      );
      expect(
        OperationLogger.generateIdentityDescription(
          action: LogAction.delete,
          itemName: 'John',
        ),
        'Deleted John',
      );
      expect(
        OperationLogger.generateIdentityDescription(
          action: LogAction.restore,
          itemName: 'John',
        ),
        'Restored John',
      );
      expect(
        OperationLogger.generateIdentityDescription(
          action: LogAction.purge,
          itemName: 'John',
        ),
        'Permanently deleted John',
      );
    });

    test('generateIdentityDescription uses fieldPath when no itemName', () {
      expect(
        OperationLogger.generateIdentityDescription(
          action: LogAction.create,
          fieldPath: 'identity.firstName',
        ),
        'Added First Name',
      );
    });

    test('generateTravelDescription includes itemType and itemName', () {
      expect(
        OperationLogger.generateTravelDescription(
          action: LogAction.create,
          itemType: 'passport',
          itemName: 'US Passport',
        ),
        'Added passport: US Passport',
      );
    });

    test('generateTravelDescription without itemName', () {
      expect(
        OperationLogger.generateTravelDescription(
          action: LogAction.update,
          itemType: 'visa',
        ),
        'Updated visa',
      );
    });

    test('generateFinancialDescription for each action', () {
      expect(
        OperationLogger.generateFinancialDescription(
          action: LogAction.create,
          itemType: 'bank account',
        ),
        'Added bank account',
      );
      expect(
        OperationLogger.generateFinancialDescription(
          action: LogAction.delete,
          itemType: 'card',
          itemName: 'Visa',
        ),
        'Deleted card: Visa',
      );
    });

    test('generateProfessionalDescription for each action', () {
      expect(
        OperationLogger.generateProfessionalDescription(
          action: LogAction.create,
          itemType: 'education',
        ),
        'Added education',
      );
    });
  });

  group('OperationLogger.createNotification', () {
    test('creates notification with correct fields', () {
      final msg = OperationLogger.createNotification(
        section: LogSection.identity,
        action: LogAction.create,
        itemName: 'John',
      );
      expect(msg.type, OperationType.create);
      expect(msg.section, 'identity');
      expect(msg.itemName, 'John');
    });

    test('createNotificationForSection uses string section', () {
      final msg = OperationLogger.createNotificationForSection(
        section: 'custom',
        action: LogAction.update,
        itemName: 'Item',
      );
      expect(msg.section, 'custom');
    });
  });
}
