import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/models/base_models.dart';

class _TestEntry with FormattableEntry {
  @override
  final String entryType;
  final Map<String, dynamic> _data;

  _TestEntry({required this.entryType, required Map<String, dynamic> data})
      : _data = data;

  @override
  Map<String, dynamic> toMap() => _data;
}

void main() {
  group('FormattableEntry', () {
    test('toFormattedString includes non-ignored keys', () {
      final entry = _TestEntry(
        entryType: 'test',
        data: {
          'name': 'John',
          'email': 'john@example.com',
          'id': 'should-be-ignored',
          'createdAt': 'should-be-ignored',
        },
      );
      final result = entry.toFormattedString();
      expect(result, contains('Name: John'));
      expect(result, contains('Email: john@example.com'));
      expect(result, isNot(contains('id')));
      expect(result, isNot(contains('createdAt')));
    });

    test('toFormattedString skips null and empty values', () {
      final entry = _TestEntry(
        entryType: 'test',
        data: {
          'name': 'John',
          'empty': '',
          'nulled': null,
        },
      );
      final result = entry.toFormattedString();
      expect(result, contains('Name: John'));
      expect(result, isNot(contains('empty')));
      expect(result, isNot(contains('nulled')));
    });

    test('toFormattedString capitalizes keys', () {
      final entry = _TestEntry(
        entryType: 'test',
        data: {'firstName': 'Jane'},
      );
      expect(entry.toFormattedString(), 'FirstName: Jane');
    });

    test('toFormattedString returns empty for all-ignored keys', () {
      final entry = _TestEntry(
        entryType: 'test',
        data: {
          'id': '1',
          'createdAt': '2024-01-01',
          'updatedAt': '2024-01-01',
          'fieldHistories': [],
          'typeId': 'x',
          'iconName': 'icon',
          'parentId': null,
          'childrenIds': [],
          'isDeleted': false,
          'deletedAt': null,
        },
      );
      expect(entry.toFormattedString(), isEmpty);
    });
  });
}
