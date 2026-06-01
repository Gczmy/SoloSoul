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

    test('logContactInformation creates correct entry', () {
      final entry = OperationLogger.logContactInformation(
        action: LogAction.create,
        description: 'Added email',
      );
      expect(entry.section, 'contact information');
    });

    test('logAddress creates correct entry', () {
      final entry = OperationLogger.logAddress(
        action: LogAction.create,
        description: 'Added address',
      );
      expect(entry.section, 'address');
    });

    test('logVisa defaults to critical sensitivity', () {
      final entry = OperationLogger.logVisa(
        action: LogAction.create,
        description: 'Added visa',
      );
      expect(entry.section, 'visa');
      expect(entry.sensitivityLevel, SensitivityLevel.critical);
    });

    test('logTravelHistory creates correct entry', () {
      final entry = OperationLogger.logTravelHistory(
        action: LogAction.create,
        description: 'Added trip',
      );
      expect(entry.section, 'travel history');
    });

    test('logCard defaults to critical sensitivity', () {
      final entry = OperationLogger.logCard(
        action: LogAction.create,
        description: 'Added card',
      );
      expect(entry.section, 'card');
      expect(entry.sensitivityLevel, SensitivityLevel.critical);
    });

    test('logEmployment defaults to sensitive sensitivity', () {
      final entry = OperationLogger.logEmployment(
        action: LogAction.create,
        description: 'Added job',
      );
      expect(entry.section, 'employment');
      expect(entry.sensitivityLevel, SensitivityLevel.sensitive);
    });

    test('logLanguage defaults to public sensitivity', () {
      final entry = OperationLogger.logLanguage(
        action: LogAction.create,
        description: 'Added language',
      );
      expect(entry.section, 'language');
      expect(entry.sensitivityLevel, SensitivityLevel.public);
    });

    test('logTravel creates correct entry', () {
      final entry = OperationLogger.logTravel(
        action: LogAction.create,
        description: 'Added travel',
      );
      expect(entry.section, 'travel');
    });

    test('logFinancial creates correct entry', () {
      final entry = OperationLogger.logFinancial(
        action: LogAction.create,
        description: 'Added financial',
      );
      expect(entry.section, 'financial');
    });

    test('logProfessional creates correct entry', () {
      final entry = OperationLogger.logProfessional(
        action: LogAction.create,
        description: 'Added professional',
      );
      expect(entry.section, 'professional');
    });

    test('logSensitivitySettings creates correct entry', () {
      final entry = OperationLogger.logSensitivitySettings(
        action: LogAction.update,
        description: 'Updated sensitivity',
        fieldPath: 'identity.fullName',
      );
      expect(entry.section, 'sensitivity settings');
      expect(entry.fieldPath, 'identity.fullName');
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

  group('generateTravelDescription all actions', () {
    test('update', () {
      expect(
        OperationLogger.generateTravelDescription(
          action: LogAction.update,
          itemType: 'passport',
          itemName: 'P1',
        ),
        'Updated passport: P1',
      );
    });

    test('delete', () {
      expect(
        OperationLogger.generateTravelDescription(
          action: LogAction.delete,
          itemType: 'visa',
        ),
        'Deleted visa',
      );
    });

    test('restore', () {
      expect(
        OperationLogger.generateTravelDescription(
          action: LogAction.restore,
          itemType: 'ticket',
          itemName: 'T1',
        ),
        'Restored ticket: T1',
      );
    });

    test('purge', () {
      expect(
        OperationLogger.generateTravelDescription(
          action: LogAction.purge,
          itemType: 'history',
        ),
        'Permanently deleted history',
      );
    });
  });

  group('generateFinancialDescription all actions', () {
    test('update', () {
      expect(
        OperationLogger.generateFinancialDescription(
          action: LogAction.update,
          itemType: 'card',
          itemName: 'C1',
        ),
        'Updated card: C1',
      );
    });

    test('delete', () {
      expect(
        OperationLogger.generateFinancialDescription(
          action: LogAction.delete,
          itemType: 'account',
        ),
        'Deleted account',
      );
    });

    test('restore', () {
      expect(
        OperationLogger.generateFinancialDescription(
          action: LogAction.restore,
          itemType: 'tax',
          itemName: 'T1',
        ),
        'Restored tax: T1',
      );
    });

    test('purge', () {
      expect(
        OperationLogger.generateFinancialDescription(
          action: LogAction.purge,
          itemType: 'record',
        ),
        'Permanently deleted record',
      );
    });
  });

  group('generateProfessionalDescription all actions', () {
    test('update', () {
      expect(
        OperationLogger.generateProfessionalDescription(
          action: LogAction.update,
          itemType: 'skill',
          itemName: 'S1',
        ),
        'Updated skill: S1',
      );
    });

    test('delete', () {
      expect(
        OperationLogger.generateProfessionalDescription(
          action: LogAction.delete,
          itemType: 'job',
        ),
        'Deleted job',
      );
    });

    test('restore', () {
      expect(
        OperationLogger.generateProfessionalDescription(
          action: LogAction.restore,
          itemType: 'award',
          itemName: 'A1',
        ),
        'Restored award: A1',
      );
    });

    test('purge', () {
      expect(
        OperationLogger.generateProfessionalDescription(
          action: LogAction.purge,
          itemType: 'cert',
        ),
        'Permanently deleted cert',
      );
    });
  });
}
