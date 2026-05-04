import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';

void main() {
  group('SensitivityLevel', () {
    test('has correct order from least to most restrictive', () {
      expect(SensitivityLevel.public.index, 0);
      expect(SensitivityLevel.internal.index, 1);
      expect(SensitivityLevel.sensitive.index, 2);
      expect(SensitivityLevel.critical.index, 3);
    });

    test('rank matches index', () {
      expect(SensitivityLevel.public.rank, 0);
      expect(SensitivityLevel.internal.rank, 1);
      expect(SensitivityLevel.sensitive.rank, 2);
      expect(SensitivityLevel.critical.rank, 3);
    });
  });

  group('SensitivityLevelExtension.isAtLeast', () {
    test('public is at least public', () {
      expect(SensitivityLevel.public.isAtLeast(SensitivityLevel.public), isTrue);
    });

    test('public is not at least sensitive', () {
      expect(
        SensitivityLevel.public.isAtLeast(SensitivityLevel.sensitive),
        isFalse,
      );
    });

    test('critical is at least all levels', () {
      expect(
        SensitivityLevel.critical.isAtLeast(SensitivityLevel.public),
        isTrue,
      );
      expect(
        SensitivityLevel.critical.isAtLeast(SensitivityLevel.internal),
        isTrue,
      );
      expect(
        SensitivityLevel.critical.isAtLeast(SensitivityLevel.sensitive),
        isTrue,
      );
      expect(
        SensitivityLevel.critical.isAtLeast(SensitivityLevel.critical),
        isTrue,
      );
    });

    test('sensitive is at least internal', () {
      expect(
        SensitivityLevel.sensitive.isAtLeast(SensitivityLevel.internal),
        isTrue,
      );
    });

    test('internal is not at least critical', () {
      expect(
        SensitivityLevel.internal.isAtLeast(SensitivityLevel.critical),
        isFalse,
      );
    });
  });

  group('SensitivityLevelExtension.label', () {
    test('returns correct labels', () {
      expect(SensitivityLevel.public.label, 'Public');
      expect(SensitivityLevel.internal.label, 'Internal');
      expect(SensitivityLevel.sensitive.label, 'Sensitive');
      expect(SensitivityLevel.critical.label, 'Critical');
    });
  });

  group('SensitivityLevelExtension.description', () {
    test('returns non-empty descriptions for all levels', () {
      for (final level in SensitivityLevel.values) {
        expect(level.description, isNotEmpty);
      }
    });

    test('returns distinct descriptions', () {
      final descriptions =
          SensitivityLevel.values.map((l) => l.description).toSet();
      expect(descriptions.length, SensitivityLevel.values.length);
    });
  });
}
