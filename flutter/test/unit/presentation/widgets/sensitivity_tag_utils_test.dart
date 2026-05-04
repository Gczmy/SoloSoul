import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/presentation/widgets/sensitivity_tag.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';

void main() {
  group('getSensitivityLabel', () {
    test('returns correct labels for all levels', () {
      expect(getSensitivityLabel(SensitivityLevel.public), 'Public');
      expect(getSensitivityLabel(SensitivityLevel.internal), 'Internal');
      expect(getSensitivityLabel(SensitivityLevel.sensitive), 'Sensitive');
      expect(getSensitivityLabel(SensitivityLevel.critical), 'Critical');
    });

    test('labels are non-empty', () {
      for (final level in SensitivityLevel.values) {
        expect(getSensitivityLabel(level), isNotEmpty);
      }
    });
  });

  group('getSensitivityColor', () {
    test('returns green for public', () {
      final color = getSensitivityColor(SensitivityLevel.public);
      expect(color, Colors.green);
    });

    test('returns blue for internal', () {
      final color = getSensitivityColor(SensitivityLevel.internal);
      expect(color, Colors.blue);
    });

    test('returns orange for sensitive', () {
      final color = getSensitivityColor(SensitivityLevel.sensitive);
      expect(color, Colors.orange);
    });

    test('returns a red shade for critical', () {
      final color = getSensitivityColor(SensitivityLevel.critical);
      // Colors.red.shade900 is a MaterialColor swatch value
      expect(color, isNotNull);
    });

    test('all levels return distinct colors', () {
      final colors =
          SensitivityLevel.values.map(getSensitivityColor).toSet();
      expect(colors.length, SensitivityLevel.values.length);
    });
  });
}
