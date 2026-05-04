import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/services/operation_notification.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart';

void main() {
  group('OperationType', () {
    test('has expected values', () {
      expect(OperationType.values, hasLength(5));
      expect(OperationType.values, contains(OperationType.create));
      expect(OperationType.values, contains(OperationType.update));
      expect(OperationType.values, contains(OperationType.delete));
      expect(OperationType.values, contains(OperationType.restore));
      expect(OperationType.values, contains(OperationType.purge));
    });
  });

  group('OperationMessage', () {
    group('message getter', () {
      group('create', () {
        test('with fieldName', () {
          const msg = OperationMessage(
            type: OperationType.create,
            section: 'identity',
            fieldName: 'email - work',
          );
          expect(msg.message, 'Added email - work to Identity');
        });

        test('with itemName fallback', () {
          const msg = OperationMessage(
            type: OperationType.create,
            section: 'travel',
            itemName: 'Paris Trip',
          );
          expect(msg.message, 'Added Paris Trip to Travel');
        });

        test('with neither fieldName nor itemName', () {
          const msg = OperationMessage(
            type: OperationType.create,
            section: 'education',
          );
          expect(msg.message, 'Added new item to Education');
        });

        test('fieldName takes priority over itemName', () {
          const msg = OperationMessage(
            type: OperationType.create,
            section: 'identity',
            fieldName: 'email - work',
            itemName: 'Contact Info',
          );
          expect(msg.message, contains('email - work'));
        });
      });

      group('update', () {
        test('with fieldName', () {
          const msg = OperationMessage(
            type: OperationType.update,
            section: 'bank account',
            fieldName: 'Account Number',
          );
          expect(msg.message, 'Updated Account Number in Bank Account');
        });

        test('with itemName', () {
          const msg = OperationMessage(
            type: OperationType.update,
            section: 'card',
            itemName: 'Visa Card',
          );
          expect(msg.message, 'Updated Visa Card in Card');
        });

        test('with neither', () {
          const msg = OperationMessage(
            type: OperationType.update,
            section: 'employment',
          );
          expect(msg.message, 'Updated Employment');
        });
      });

      group('delete', () {
        test('with fieldName', () {
          const msg = OperationMessage(
            type: OperationType.delete,
            section: 'address',
            fieldName: 'Home Address',
          );
          expect(msg.message, 'Deleted Home Address from Address');
        });

        test('with itemName', () {
          const msg = OperationMessage(
            type: OperationType.delete,
            section: 'visa',
            itemName: 'US Visa',
          );
          expect(msg.message, 'Deleted US Visa from Visa');
        });

        test('with neither', () {
          const msg = OperationMessage(
            type: OperationType.delete,
            section: 'passport',
          );
          expect(msg.message, 'Deleted from Passport');
        });
      });

      group('restore', () {
        test('with fieldName', () {
          const msg = OperationMessage(
            type: OperationType.restore,
            section: 'skill',
            fieldName: 'Flutter',
          );
          expect(msg.message, 'Restored Flutter to Skill');
        });

        test('with itemName', () {
          const msg = OperationMessage(
            type: OperationType.restore,
            section: 'language',
            itemName: 'English',
          );
          expect(msg.message, 'Restored English to Language');
        });

        test('with neither', () {
          const msg = OperationMessage(
            type: OperationType.restore,
            section: 'education',
          );
          expect(msg.message, 'Restored Education');
        });
      });

      group('purge', () {
        test('with fieldName', () {
          const msg = OperationMessage(
            type: OperationType.purge,
            section: 'identity',
            fieldName: 'Old Name',
          );
          expect(msg.message, 'Permanently deleted Old Name');
        });

        test('with itemName', () {
          const msg = OperationMessage(
            type: OperationType.purge,
            section: 'card',
            itemName: 'Expired Card',
          );
          expect(msg.message, 'Permanently deleted Expired Card');
        });

        test('with neither', () {
          const msg = OperationMessage(
            type: OperationType.purge,
            section: 'travel',
          );
          expect(msg.message, 'Permanently deleted from Travel');
        });
      });

      group('customMessage', () {
        test('returns custom message when set', () {
          const msg = OperationMessage(
            type: OperationType.create,
            section: 'identity',
            customMessage: 'Custom override',
          );
          expect(msg.message, 'Custom override');
        });

        test('ignores other fields when customMessage set', () {
          const msg = OperationMessage(
            type: OperationType.create,
            section: 'identity',
            fieldName: 'email',
            itemName: 'item',
            customMessage: 'Custom',
          );
          expect(msg.message, 'Custom');
        });
      });

      group('section display names', () {
        test('maps known sections', () {
          final expected = {
            'identity': 'Identity',
            'contact information': 'Contact Information',
            'address': 'Address',
            'id card': 'ID Card',
            'passport': 'Passport',
            'visa': 'Visa',
            'travel history': 'Travel History',
            'bank account': 'Bank Account',
            'card': 'Card',
            'education': 'Education',
            'employment': 'Employment',
            'skill': 'Skill',
            'language': 'Language',
            'travel': 'Travel',
            'financial': 'Financial',
            'professional': 'Professional',
          };

          for (final entry in expected.entries) {
            final msg = OperationMessage(
              type: OperationType.create,
              section: entry.key,
            );
            expect(msg.message, contains(entry.value),
                reason: 'Section "${entry.key}" should map to "${entry.value}"');
          }
        });

        test('uses raw section for unknown', () {
          const msg = OperationMessage(
            type: OperationType.create,
            section: 'customSection',
          );
          expect(msg.message, contains('customSection'));
        });
      });
    });

    group('snackBarType', () {
      test('create maps to success', () {
        const msg = OperationMessage(
          type: OperationType.create,
          section: 'identity',
        );
        expect(msg.snackBarType, SnackBarType.success);
      });

      test('update maps to info', () {
        const msg = OperationMessage(
          type: OperationType.update,
          section: 'identity',
        );
        expect(msg.snackBarType, SnackBarType.info);
      });

      test('delete maps to warning', () {
        const msg = OperationMessage(
          type: OperationType.delete,
          section: 'identity',
        );
        expect(msg.snackBarType, SnackBarType.warning);
      });

      test('restore maps to info', () {
        const msg = OperationMessage(
          type: OperationType.restore,
          section: 'identity',
        );
        expect(msg.snackBarType, SnackBarType.info);
      });

      test('purge maps to error', () {
        const msg = OperationMessage(
          type: OperationType.purge,
          section: 'identity',
        );
        expect(msg.snackBarType, SnackBarType.error);
      });
    });
  });
}
