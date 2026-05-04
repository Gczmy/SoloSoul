import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart';
import 'package:solosoul_flutter/presentation/widgets/sidebar/sidebar_header.dart';

void main() {
  group('SidebarHeader', () {
    testWidgets('renders expanded layout with logo and text',
        (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: SidebarHeader(
              expanded: true,
              onToggle: () {},
            ),
          ),
        ),
      );

      expect(find.text('SoloSoul'), findsOneWidget);
      expect(find.byIcon(Icons.auto_awesome), findsOneWidget);
      expect(find.byIcon(Icons.chevron_left), findsOneWidget);
    });

    testWidgets('renders collapsed layout with icon only',
        (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: SidebarHeader(
              expanded: false,
              onToggle: () {},
            ),
          ),
        ),
      );

      expect(find.text('SoloSoul'), findsNothing);
      expect(find.byIcon(Icons.auto_awesome), findsOneWidget);
    });

    testWidgets('calls onToggle when collapse button tapped',
        (tester) async {
      var toggled = false;
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: SidebarHeader(
              expanded: true,
              onToggle: () => toggled = true,
            ),
          ),
        ),
      );

      await tester.tap(find.byIcon(Icons.chevron_left));
      await tester.pump();
      expect(toggled, true);
    });

    testWidgets('calls onToggle when expand button tapped',
        (tester) async {
      var toggled = false;
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: SidebarHeader(
              expanded: false,
              onToggle: () => toggled = true,
            ),
          ),
        ),
      );

      await tester.tap(find.byIcon(Icons.auto_awesome));
      await tester.pump();
      expect(toggled, true);
    });

    testWidgets('has fixed height of 64', (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: SidebarHeader(
              expanded: true,
              onToggle: () {},
            ),
          ),
        ),
      );

      final size = tester.getSize(find.byType(SidebarHeader));
      expect(size.height, 64);
    });

    testWidgets('logo container has decoration',
        (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: SidebarHeader(
              expanded: true,
              onToggle: () {},
            ),
          ),
        ),
      );

      final container = tester.widget<Container>(
        find.descendant(
          of: find.byType(SidebarHeader),
          matching: find.byType(Container),
        ).first,
      );
      final decoration = container.decoration as BoxDecoration;
      expect(decoration.borderRadius, BorderRadius.circular(10));
      expect(decoration.color, isNotNull);
    });

    testWidgets('hides text in narrow expanded mode', (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: Center(
              child: SizedBox(
                width: 120, // Narrow but expanded
                child: SidebarHeader(
                  expanded: true,
                  onToggle: () {},
                ),
              ),
            ),
          ),
        ),
      );

      // At 120px width, inner maxWidth is < 140 after padding,
      // so showText should be false
      expect(find.text('SoloSoul'), findsNothing);
    });
  });
}
