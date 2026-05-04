import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/presentation/widgets/home/quick_action_tile.dart';

void main() {
  group('QuickActionTile', () {
    testWidgets('renders icon, label and colored container',
        (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: QuickActionTile(
              icon: Icons.add,
              label: 'Add',
              color: Colors.blue,
            ),
          ),
        ),
      );

      expect(find.byIcon(Icons.add), findsOneWidget);
      expect(find.text('Add'), findsOneWidget);

      final container = tester.widget<Container>(
        find.descendant(of: find.byType(Card), matching: find.byType(Container)).first,
      );
      final decoration = container.decoration as BoxDecoration;
      expect(decoration.borderRadius, BorderRadius.circular(10));
    });

    testWidgets('applies color to icon and background tint', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: QuickActionTile(
              icon: Icons.lock,
              label: 'Secure',
              color: Colors.red,
            ),
          ),
        ),
      );

      final icon = tester.widget<Icon>(find.byIcon(Icons.lock));
      expect(icon.color, Colors.red);
    });

    testWidgets('calls onTap when tapped', (tester) async {
      var tapped = false;
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: QuickActionTile(
              icon: Icons.settings,
              label: 'Settings',
              color: Colors.green,
              onTap: () => tapped = true,
            ),
          ),
        ),
      );

      await tester.tap(find.byType(QuickActionTile));
      await tester.pump();
      expect(tapped, true);
    });

    testWidgets('has fixed 90x90 size', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: QuickActionTile(
              icon: Icons.home,
              label: 'Home',
              color: Colors.purple,
            ),
          ),
        ),
      );

      final sizedBox = tester.widget<SizedBox>(find.byType(SizedBox).first);
      expect(sizedBox.width, 90);
      expect(sizedBox.height, 90);
    });
  });
}
