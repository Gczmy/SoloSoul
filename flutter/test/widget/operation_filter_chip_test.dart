import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/presentation/widgets/operation_filter_chip.dart';

void main() {
  group('OperationFilterChip', () {
    testWidgets('renders label and icon when unselected', (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: OperationFilterChip(
              label: 'Create',
              icon: Icons.add,
              isSelected: false,
              color: Colors.blue,
              onSelected: (_) {},
            ),
          ),
        ),
      );

      expect(find.text('Create'), findsOneWidget);
      expect(find.byIcon(Icons.add), findsOneWidget);
    });

    testWidgets('sets backgroundColor and selectedColor', (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: OperationFilterChip(
              label: 'Delete',
              icon: Icons.delete,
              isSelected: false,
              color: Colors.red,
              onSelected: (_) {},
            ),
          ),
        ),
      );

      final chip = tester.widget<FilterChip>(find.byType(FilterChip));
      expect(chip.backgroundColor, Colors.red.withValues(alpha: 0.1));
      expect(chip.selectedColor, Colors.red);
      expect(chip.checkmarkColor, Colors.white);
    });

    testWidgets('selected chip uses white checkmark', (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: OperationFilterChip(
              label: 'Update',
              icon: Icons.edit,
              isSelected: true,
              color: Colors.green,
              onSelected: (_) {},
            ),
          ),
        ),
      );

      final chip = tester.widget<FilterChip>(find.byType(FilterChip));
      expect(chip.selectedColor, Colors.green);
      expect(chip.checkmarkColor, Colors.white);
    });

    testWidgets('calls onSelected with true when tapped', (tester) async {
      bool? selectedValue;
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: OperationFilterChip(
              label: 'Toggle',
              icon: Icons.check,
              isSelected: false,
              color: Colors.purple,
              onSelected: (v) => selectedValue = v,
            ),
          ),
        ),
      );

      await tester.tap(find.byType(FilterChip));
      await tester.pump();
      expect(selectedValue, true);
    });

    testWidgets('calls onSelected with false when deselected',
        (tester) async {
      bool? selectedValue;
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: OperationFilterChip(
              label: 'Toggle',
              icon: Icons.check,
              isSelected: true,
              color: Colors.purple,
              onSelected: (v) => selectedValue = v,
            ),
          ),
        ),
      );

      await tester.tap(find.byType(FilterChip));
      await tester.pump();
      expect(selectedValue, false);
    });

    testWidgets('uses compact visual density', (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: OperationFilterChip(
              label: 'Test',
              icon: Icons.label,
              isSelected: false,
              color: Colors.orange,
              onSelected: (_) {},
            ),
          ),
        ),
      );

      final chip = tester.widget<FilterChip>(find.byType(FilterChip));
      expect(chip.visualDensity, VisualDensity.compact);
    });
  });
}
