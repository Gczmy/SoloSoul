import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/presentation/widgets/sensitivity_tag.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';

void main() {
  group('SensitivityTag Widget Tests', () {
    testWidgets('renders correctly for public level', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: SensitivityTag(level: SensitivityLevel.public),
          ),
        ),
      );

      expect(find.text('Public'), findsOneWidget);
    });

    testWidgets('renders correctly for sensitive level', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: SensitivityTag(level: SensitivityLevel.sensitive),
          ),
        ),
      );

      expect(find.text('Sensitive'), findsOneWidget);
    });

    testWidgets('renders correctly for internal level', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: SensitivityTag(level: SensitivityLevel.internal),
          ),
        ),
      );

      expect(find.text('Internal'), findsOneWidget);
    });

    testWidgets('renders correctly for critical level', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: SensitivityTag(level: SensitivityLevel.critical),
          ),
        ),
      );

      expect(find.text('Critical'), findsOneWidget);
    });

    testWidgets('has correct container styling', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: SensitivityTag(level: SensitivityLevel.public),
          ),
        ),
      );

      final container = tester.widget<Container>(find.byType(Container).first);
      final decoration = container.decoration as BoxDecoration;

      expect(decoration.borderRadius, equals(BorderRadius.circular(4)));
      expect(container.padding, equals(const EdgeInsets.symmetric(horizontal: 6, vertical: 2)));
    });

    testWidgets('text has correct styling properties', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: SensitivityTag(level: SensitivityLevel.critical),
          ),
        ),
      );

      final text = tester.widget<Text>(find.byType(Text).first);
      expect(text.style?.fontSize, equals(10));
      expect(text.style?.fontWeight, equals(FontWeight.w600));
    });
  });

  group('Helper Function Tests', () {
    test('getSensitivityColor returns correct colors', () {
      expect(getSensitivityColor(SensitivityLevel.public), equals(Colors.green));
      expect(getSensitivityColor(SensitivityLevel.internal), equals(Colors.blue));
      expect(getSensitivityColor(SensitivityLevel.sensitive), equals(Colors.orange));
      expect(getSensitivityColor(SensitivityLevel.critical), equals(Colors.red.shade900));
    });

    test('getSensitivityLabel returns correct labels', () {
      expect(getSensitivityLabel(SensitivityLevel.public), equals('Public'));
      expect(getSensitivityLabel(SensitivityLevel.internal), equals('Internal'));
      expect(getSensitivityLabel(SensitivityLevel.sensitive), equals('Sensitive'));
      expect(getSensitivityLabel(SensitivityLevel.critical), equals('Critical'));
    });
  });
}
