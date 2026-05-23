import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/presentation/widgets/sidebar/nav_tile.dart';

void main() {
  group('NavTile', () {
    testWidgets('renders icon and label when expanded', (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: NavTile(
              icon: Icons.home,
              label: 'Home',
              expanded: true,
              onTap: () {},
            ),
          ),
        ),
      );

      expect(find.byIcon(Icons.home), findsOneWidget);
      expect(find.text('Home'), findsOneWidget);
    });

    testWidgets('wraps with Tooltip when not expanded', (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: NavTile(
              icon: Icons.settings,
              label: 'Settings',
              expanded: false,
              onTap: () {},
            ),
          ),
        ),
      );

      expect(find.byType(Tooltip), findsOneWidget);
      final tooltip = tester.widget<Tooltip>(find.byType(Tooltip));
      expect(tooltip.message, 'Settings');
    });

    testWidgets('shows larger icon when collapsed', (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: NavTile(
              icon: Icons.person,
              label: 'Profile',
              expanded: false,
              onTap: () {},
            ),
          ),
        ),
      );

      final icon = tester.widget<Icon>(find.byIcon(Icons.person));
      expect(icon.size, 22);
    });

    testWidgets('applies selected state colors', (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: NavTile(
              icon: Icons.star,
              label: 'Favorites',
              expanded: true,
              selected: true,
              onTap: () {},
            ),
          ),
        ),
      );

      final text = tester.widget<Text>(find.text('Favorites'));
      expect(text.style?.fontWeight, FontWeight.w600);
    });

    testWidgets('calls onTap when tapped', (tester) async {
      var tapped = false;
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: NavTile(
              icon: Icons.mail,
              label: 'Mail',
              expanded: true,
              onTap: () => tapped = true,
            ),
          ),
        ),
      );

      await tester.tap(find.byType(NavTile));
      await tester.pump();
      expect(tapped, true);
    });

    testWidgets('shows icon with InkWell when onIconTap provided',
        (tester) async {
      var iconTapped = false;
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: NavTile(
              icon: Icons.folder,
              label: 'Folder',
              expanded: true,
              onTap: () {},
              onIconTap: () => iconTapped = true,
            ),
          ),
        ),
      );

      // The icon is wrapped in an InkWell when onIconTap is provided
      expect(find.byType(InkWell), findsWidgets);
      await tester.tap(find.byType(InkWell).first);
      await tester.pump();
      expect(iconTapped, isTrue);
    });

    testWidgets('has fixed height of 40', (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: NavTile(
              icon: Icons.dashboard,
              label: 'Dashboard',
              expanded: true,
              onTap: () {},
            ),
          ),
        ),
      );

      final container = tester.widget<Container>(
        find.descendant(of: find.byType(LayoutBuilder), matching: find.byType(Container)).first,
      );
      expect(container.constraints?.minHeight, 40);
    });
  });
}
