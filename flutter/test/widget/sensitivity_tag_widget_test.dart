import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/presentation/widgets/sensitivity_tag.dart';

void main() {
  group('SensitivityTag', () {
    testWidgets('renders public tag with green color', (tester) async {
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

    testWidgets('renders internal tag with blue color', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: SensitivityTag(level: SensitivityLevel.internal),
          ),
        ),
      );

      expect(find.text('Internal'), findsOneWidget);
      final text = tester.widget<Text>(find.text('Internal'));
      expect(text.style?.color, Colors.blue);
    });

    testWidgets('renders sensitive tag with orange color', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: SensitivityTag(level: SensitivityLevel.sensitive),
          ),
        ),
      );

      expect(find.text('Sensitive'), findsOneWidget);
      final text = tester.widget<Text>(find.text('Sensitive'));
      expect(text.style?.color, Colors.orange);
    });

    testWidgets('renders critical tag with red color', (tester) async {
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

    testWidgets('has container decoration', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: SensitivityTag(level: SensitivityLevel.public),
          ),
        ),
      );

      expect(find.byType(Container), findsOneWidget);
      final container = tester.widget<Container>(find.byType(Container));
      expect(container.decoration, isA<BoxDecoration>());
    });

    testWidgets('text has correct style attributes', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: SensitivityTag(level: SensitivityLevel.public),
          ),
        ),
      );

      final text = tester.widget<Text>(find.text('Public'));
      expect(text.style?.fontSize, 10);
      expect(text.style?.fontWeight, FontWeight.w600);
    });
  });
}
