import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/presentation/providers/account_style_provider.dart';

void main() {
  group('SensitivityDisplayMode', () {
    test('has expected values', () {
      expect(SensitivityDisplayMode.values, hasLength(3));
      expect(SensitivityDisplayMode.values,
          contains(SensitivityDisplayMode.showAll));
      expect(SensitivityDisplayMode.values,
          contains(SensitivityDisplayMode.hidePrivate));
      expect(SensitivityDisplayMode.values,
          contains(SensitivityDisplayMode.hideAll));
    });
  });

  group('SensitivityResolver', () {
    const resolver = SensitivityResolver();

    test('revealed field returns public', () {
      final result = resolver.resolve(
        fieldId: 'card.cvv',
        fieldSettings: {},
        revealedFields: {'card.cvv'},
      );
      expect(result, SensitivityLevel.public);
    });

    test('user override takes priority over tag default', () {
      final result = resolver.resolve(
        fieldId: 'some.field',
        fieldSettings: {'some.field': SensitivityLevel.sensitive},
        revealedFields: {},
        tags: ['financial'],
      );
      expect(result, SensitivityLevel.sensitive);
    });

    test('tag default applies when no user override', () {
      final result = resolver.resolve(
        fieldId: 'unknown.field',
        fieldSettings: {},
        revealedFields: {},
        tags: ['financial'],
      );
      expect(result, SensitivityLevel.critical);
    });

    test('first matching tag wins', () {
      final result = resolver.resolve(
        fieldId: 'unknown.field',
        fieldSettings: {},
        revealedFields: {},
        tags: ['work', 'financial'],
      );
      expect(result, SensitivityLevel.internal);
    });

    test('registry default applies when no tag match', () {
      final result = resolver.resolve(
        fieldId: 'identity.fullName',
        fieldSettings: {},
        revealedFields: {},
        tags: ['unknownTag'],
      );
      expect(result, SensitivityLevel.public);
    });

    test('fallback to public for completely unknown field', () {
      final result = resolver.resolve(
        fieldId: 'nonexistent.field',
        fieldSettings: {},
        revealedFields: {},
      );
      expect(result, SensitivityLevel.public);
    });

    test('tag defaults have expected mappings', () {
      // 'work' -> internal
      final work = resolver.resolve(
        fieldId: 'x',
        fieldSettings: {},
        revealedFields: {},
        tags: ['work'],
      );
      expect(work, SensitivityLevel.internal);

      // 'personal' -> sensitive
      final personal = resolver.resolve(
        fieldId: 'x',
        fieldSettings: {},
        revealedFields: {},
        tags: ['personal'],
      );
      expect(personal, SensitivityLevel.sensitive);

      // 'financial' -> critical
      final financial = resolver.resolve(
        fieldId: 'x',
        fieldSettings: {},
        revealedFields: {},
        tags: ['financial'],
      );
      expect(financial, SensitivityLevel.critical);

      // 'health' -> critical
      final health = resolver.resolve(
        fieldId: 'x',
        fieldSettings: {},
        revealedFields: {},
        tags: ['health'],
      );
      expect(health, SensitivityLevel.critical);
    });

    test('empty tags falls through to registry', () {
      final result = resolver.resolve(
        fieldId: 'card.cvv',
        fieldSettings: {},
        revealedFields: {},
        tags: [],
      );
      expect(result, SensitivityLevel.critical);
    });

    test('reveal takes highest priority over user override', () {
      final result = resolver.resolve(
        fieldId: 'card.cvv',
        fieldSettings: {'card.cvv': SensitivityLevel.critical},
        revealedFields: {'card.cvv'},
      );
      expect(result, SensitivityLevel.public);
    });
  });

  group('AccountStyle', () {
    test('default constructor has correct defaults', () {
      const style = AccountStyle();
      expect(style.fieldSettings, isEmpty);
      expect(style.tagDefaults, isEmpty);
      expect(style.lastModified, isNull);
      expect(style.displayMode, SensitivityDisplayMode.hidePrivate);
      expect(style.revealedFields, isEmpty);
    });

    group('copyWith', () {
      test('copies with no changes', () {
        const style = AccountStyle(
          displayMode: SensitivityDisplayMode.showAll,
        );
        final copy = style.copyWith();
        expect(copy.displayMode, SensitivityDisplayMode.showAll);
        expect(copy.fieldSettings, isEmpty);
      });

      test('copies with changes', () {
        const style = AccountStyle();
        final copy = style.copyWith(
          displayMode: SensitivityDisplayMode.hideAll,
          revealedFields: {'field1'},
        );
        expect(copy.displayMode, SensitivityDisplayMode.hideAll);
        expect(copy.revealedFields, {'field1'});
        expect(copy.fieldSettings, isEmpty);
      });
    });

    group('toJson / fromJson', () {
      test('round-trip with defaults', () {
        const style = AccountStyle();
        final json = style.toJson();
        final restored = AccountStyle.fromJson(json);
        expect(restored.displayMode, style.displayMode);
        expect(restored.fieldSettings, isEmpty);
        expect(restored.tagDefaults, isEmpty);
        expect(restored.lastModified, isNull);
        expect(restored.revealedFields, isEmpty);
      });

      test('round-trip with field settings', () {
        final style = AccountStyle(
          fieldSettings: {
            'card.cvv': SensitivityLevel.critical,
            'identity.fullName': SensitivityLevel.public,
          },
          displayMode: SensitivityDisplayMode.showAll,
          revealedFields: {'field1', 'field2'},
          lastModified: DateTime(2024, 6, 15),
        );
        final json = style.toJson();
        final restored = AccountStyle.fromJson(json);
        expect(restored.fieldSettings['card.cvv'], SensitivityLevel.critical);
        expect(restored.fieldSettings['identity.fullName'],
            SensitivityLevel.public);
        expect(restored.displayMode, SensitivityDisplayMode.showAll);
        expect(restored.revealedFields, {'field1', 'field2'});
        expect(restored.lastModified, DateTime(2024, 6, 15));
      });

      test('round-trip with tag defaults', () {
        const style = AccountStyle(
          tagDefaults: {
            'work': SensitivityLevel.internal,
            'personal': SensitivityLevel.sensitive,
          },
        );
        final json = style.toJson();
        final restored = AccountStyle.fromJson(json);
        expect(restored.tagDefaults['work'], SensitivityLevel.internal);
        expect(restored.tagDefaults['personal'], SensitivityLevel.sensitive);
      });

      test('fromJson handles missing fields', () {
        final restored = AccountStyle.fromJson({});
        expect(restored.displayMode, SensitivityDisplayMode.hidePrivate);
        expect(restored.fieldSettings, isEmpty);
        expect(restored.tagDefaults, isEmpty);
        expect(restored.lastModified, isNull);
        expect(restored.revealedFields, isEmpty);
      });

      test('fromJson handles invalid display mode index', () {
        final restored = AccountStyle.fromJson({'display_mode': 99});
        expect(restored.displayMode, SensitivityDisplayMode.hidePrivate);
      });

      test('fromJson handles invalid sensitivity level name', () {
        final restored = AccountStyle.fromJson({
          'field_settings': {'field1': 'nonexistent'},
        });
        expect(
            restored.fieldSettings['field1'], SensitivityLevel.public);
      });

      test('fromJson handles null last_modified', () {
        final restored = AccountStyle.fromJson({'last_modified': null});
        expect(restored.lastModified, isNull);
      });

      test('fromJson handles invalid last_modified string', () {
        final restored =
            AccountStyle.fromJson({'last_modified': 'not-a-date'});
        expect(restored.lastModified, isNull);
      });
    });
  });
}
