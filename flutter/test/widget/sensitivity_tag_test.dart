import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/presentation/widgets/sensitivity_tag.dart';

void main() {
  group('getSensitivityColor', () {
    test('returns correct colors', () {
      expect(
        getSensitivityColor(SensitivityLevel.public),
        Colors.green,
      );
      expect(
        getSensitivityColor(SensitivityLevel.internal),
        Colors.blue,
      );
      expect(
        getSensitivityColor(SensitivityLevel.sensitive),
        Colors.orange,
      );
      expect(
        getSensitivityColor(SensitivityLevel.critical),
        Colors.red.shade900,
      );
    });
  });

  group('getSensitivityLabel', () {
    test('returns correct labels', () {
      expect(getSensitivityLabel(SensitivityLevel.public), 'Public');
      expect(getSensitivityLabel(SensitivityLevel.internal), 'Internal');
      expect(getSensitivityLabel(SensitivityLevel.sensitive), 'Sensitive');
      expect(getSensitivityLabel(SensitivityLevel.critical), 'Critical');
    });
  });

  group('SensitivityTag widget', () {
    testWidgets('renders label for public level', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: SensitivityTag(level: SensitivityLevel.public),
          ),
        ),
      );

      expect(find.text('Public'), findsOneWidget);
      final text = tester.widget<Text>(find.text('Public'));
      expect(text.style?.color, Colors.green);
    });

    testWidgets('renders label for critical level', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: SensitivityTag(level: SensitivityLevel.critical),
          ),
        ),
      );

      expect(find.text('Critical'), findsOneWidget);
      final text = tester.widget<Text>(find.text('Critical'));
      expect(text.style?.color, Colors.red.shade900);
    });

    testWidgets('uses small font and semibold weight', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: SensitivityTag(level: SensitivityLevel.sensitive),
          ),
        ),
      );

      final text = tester.widget<Text>(find.text('Sensitive'));
      expect(text.style?.fontSize, 10);
      expect(text.style?.fontWeight, FontWeight.w600);
    });

    testWidgets('renders container with border and background', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: SensitivityTag(level: SensitivityLevel.internal),
          ),
        ),
      );

      final container = tester.widget<Container>(find.byType(Container));
      final decoration = container.decoration as BoxDecoration;
      expect(decoration.borderRadius, BorderRadius.circular(4));
      expect(decoration.border, isA<Border>());
    });
  });
}
