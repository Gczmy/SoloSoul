import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/presentation/widgets/home/delete_badge.dart';

void main() {
  group('DeleteBadge', () {
    testWidgets('renders close icon in circular container', (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: DeleteBadge(onTap: () {}),
          ),
        ),
      );

      expect(find.byIcon(Icons.close), findsOneWidget);

      final container = tester.widget<Container>(find.byType(Container));
      final decoration = container.decoration as BoxDecoration;
      expect(decoration.shape, BoxShape.circle);
    });

    testWidgets('calls onTap when tapped', (tester) async {
      var tapped = false;
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: DeleteBadge(onTap: () => tapped = true),
          ),
        ),
      );

      await tester.tap(find.byType(DeleteBadge));
      await tester.pump();
      expect(tapped, true);
    });

    testWidgets('has initial scale of 1.0', (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: DeleteBadge(onTap: () {}),
          ),
        ),
      );

      final animatedScale = tester.widget<AnimatedScale>(find.byType(AnimatedScale));
      expect(animatedScale.scale, 1.0);
    });

    testWidgets('uses error color for background', (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: DeleteBadge(onTap: () {}),
          ),
        ),
      );

      final container = tester.widget<Container>(find.byType(Container));
      final decoration = container.decoration as BoxDecoration;
      final theme = Theme.of(tester.element(find.byType(DeleteBadge)));
      expect(decoration.color, theme.colorScheme.error);
    });

    testWidgets('icon is white and size 12', (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: DeleteBadge(onTap: () {}),
          ),
        ),
      );

      final icon = tester.widget<Icon>(find.byIcon(Icons.close));
      expect(icon.color, Colors.white);
      expect(icon.size, 12);
    });
  });
}
