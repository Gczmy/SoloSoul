import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/presentation/widgets/home/add_button.dart';
import 'package:solosoul_flutter/presentation/widgets/home/dashed_placeholder.dart';

void main() {
  group('AddButton', () {
    testWidgets('renders add icon inside dashed placeholder',
        (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: AddButton(onTap: () {}),
          ),
        ),
      );

      expect(find.byIcon(Icons.add), findsOneWidget);
      expect(find.byType(DashedPlaceholder), findsOneWidget);
    });

    testWidgets('calls onTap when tapped', (tester) async {
      var tapped = false;
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: AddButton(onTap: () => tapped = true),
          ),
        ),
      );

      await tester.tap(find.byType(AddButton));
      await tester.pump();
      expect(tapped, true);
    });

    testWidgets('has fixed 90x90 size', (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: AddButton(onTap: () {}),
          ),
        ),
      );

      final size = tester.getSize(find.byType(AddButton));
      expect(size.width, 90);
      expect(size.height, 90);
    });

    testWidgets('uses primary color for icon by default', (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: AddButton(onTap: () {}),
          ),
        ),
      );

      final icon = tester.widget<Icon>(find.byIcon(Icons.add));
      final theme = Theme.of(tester.element(find.byType(AddButton)));
      expect(icon.color, theme.colorScheme.primary);
    });

    testWidgets('icon size is 28', (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: AddButton(onTap: () {}),
          ),
        ),
      );

      final icon = tester.widget<Icon>(find.byIcon(Icons.add));
      expect(icon.size, 28);
    });
  });
}
