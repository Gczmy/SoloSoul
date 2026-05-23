import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/presentation/widgets/section_card.dart';

void main() {
  group('SectionCard', () {
    testWidgets('renders title, icon and children', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: SectionCard(
              title: 'Test Section',
              icon: Icons.star,
              children: [Text('Child 1'), Text('Child 2')],
            ),
          ),
        ),
      );

      expect(find.text('Test Section'), findsOneWidget);
      expect(find.byIcon(Icons.star), findsOneWidget);
      expect(find.text('Child 1'), findsOneWidget);
      expect(find.text('Child 2'), findsOneWidget);
    });

    testWidgets('renders action icon button when provided', (tester) async {
      var actionTapped = false;
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: SectionCard(
              title: 'Section',
              icon: Icons.folder,
              actionIcon: Icons.add,
              onAction: () => actionTapped = true,
              children: const [],
            ),
          ),
        ),
      );

      expect(find.byIcon(Icons.add), findsOneWidget);
      await tester.tap(find.byIcon(Icons.add));
      await tester.pump();
      expect(actionTapped, true);
    });

    testWidgets('hides action button when icon or callback missing',
        (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: SectionCard(
              title: 'Section',
              icon: Icons.folder,
              actionIcon: Icons.add,
              children: [],
            ),
          ),
        ),
      );

      expect(find.byIcon(Icons.add), findsNothing);
    });

    testWidgets('applies custom title color', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: SectionCard(
              title: 'Colored',
              icon: Icons.palette,
              titleColor: Colors.purple,
              children: [],
            ),
          ),
        ),
      );

      final icon = tester.widget<Icon>(find.byIcon(Icons.palette));
      expect(icon.color, Colors.purple);
    });
  });

  group('CollapsibleSectionCard', () {
    testWidgets('renders all children when count <= maxVisibleItems',
        (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: CollapsibleSectionCard(
              title: 'Small List',
              icon: Icons.list,
              maxVisibleItems: 3,
              children: [Text('A'), Text('B'), Text('C')],
            ),
          ),
        ),
      );

      expect(find.text('A'), findsOneWidget);
      expect(find.text('B'), findsOneWidget);
      expect(find.text('C'), findsOneWidget);
      expect(find.text('Show less'), findsNothing);
      expect(find.textContaining('more'), findsNothing);
    });

    testWidgets('collapses children when count > maxVisibleItems',
        (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: CollapsibleSectionCard(
              title: 'Big List',
              icon: Icons.list,
              maxVisibleItems: 2,
              children: [Text('A'), Text('B'), Text('C'), Text('D')],
            ),
          ),
        ),
      );

      // Only first 2 visible initially
      expect(find.text('A'), findsOneWidget);
      expect(find.text('B'), findsOneWidget);
      expect(find.text('C'), findsNothing);
      expect(find.text('D'), findsNothing);
      expect(find.text('Show 2 more'), findsOneWidget);
    });

    testWidgets('expands to show all children when tapped', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: CollapsibleSectionCard(
              title: 'Big List',
              icon: Icons.list,
              maxVisibleItems: 2,
              children: [Text('A'), Text('B'), Text('C')],
            ),
          ),
        ),
      );

      await tester.tap(find.text('Show 1 more'));
      await tester.pump();

      expect(find.text('A'), findsOneWidget);
      expect(find.text('B'), findsOneWidget);
      expect(find.text('C'), findsOneWidget);
      expect(find.text('Show less'), findsOneWidget);
    });

    testWidgets('collapses again when Show less tapped', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: CollapsibleSectionCard(
              title: 'Big List',
              icon: Icons.list,
              maxVisibleItems: 1,
              children: [Text('A'), Text('B')],
            ),
          ),
        ),
      );

      // Expand
      await tester.tap(find.text('Show 1 more'));
      await tester.pump();
      expect(find.text('Show less'), findsOneWidget);

      // Collapse
      await tester.tap(find.text('Show less'));
      await tester.pump();
      expect(find.text('Show 1 more'), findsOneWidget);
      expect(find.text('B'), findsNothing);
    });

    testWidgets('shows empty content when children empty and builder provided',
        (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: CollapsibleSectionCard(
              title: 'Empty',
              icon: Icons.inbox,
              children: const [],
              emptyContentBuilder: (theme) => const Text('No items yet'),
            ),
          ),
        ),
      );

      expect(find.text('No items yet'), findsOneWidget);
    });

    testWidgets('renders footer when provided', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: CollapsibleSectionCard(
              title: 'With Footer',
              icon: Icons.info,
              footer: Text('Footer content'),
              children: [],
            ),
          ),
        ),
      );

      expect(find.text('Footer content'), findsOneWidget);
      expect(find.byType(Divider), findsOneWidget);
    });
  });
}
