import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/models/entry_configs.dart';

void main() {
  group('EntryActionsConfig', () {
    test('default constructor enables all actions', () {
      const config = EntryActionsConfig();
      expect(config.showCopy, isTrue);
      expect(config.showEdit, isTrue);
      expect(config.showDelete, isTrue);
      expect(config.showHistory, isTrue);
    });

    test('all preset enables all actions', () {
      const config = EntryActionsConfig.all;
      expect(config.showCopy, isTrue);
      expect(config.showEdit, isTrue);
      expect(config.showDelete, isTrue);
      expect(config.showHistory, isTrue);
    });

    test('readOnly preset disables edit and delete', () {
      const config = EntryActionsConfig.readOnly;
      expect(config.showCopy, isTrue);
      expect(config.showEdit, isFalse);
      expect(config.showDelete, isFalse);
      expect(config.showHistory, isTrue);
    });

    test('noHistory preset disables history', () {
      const config = EntryActionsConfig.noHistory;
      expect(config.showCopy, isTrue);
      expect(config.showEdit, isTrue);
      expect(config.showDelete, isTrue);
      expect(config.showHistory, isFalse);
    });

    test('can customize individual flags', () {
      const config = EntryActionsConfig(showCopy: false, showDelete: false);
      expect(config.showCopy, isFalse);
      expect(config.showEdit, isTrue);
      expect(config.showDelete, isFalse);
      expect(config.showHistory, isTrue);
    });
  });
}
