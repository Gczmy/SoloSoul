import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/presentation/widgets/universal_entry_card.dart';

void main() {
  group('UniversalEntryCard', () {
    testWidgets('renders title only', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: UniversalEntryCard(
              title: Text('Title'),
            ),
          ),
        ),
      );

      expect(find.text('Title'), findsOneWidget);
    });

    testWidgets('renders subtitle when provided', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: UniversalEntryCard(
              title: Text('Title'),
              subtitle: Text('Subtitle'),
            ),
          ),
        ),
      );

      expect(find.text('Subtitle'), findsOneWidget);
    });

    testWidgets('renders leading widget', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: UniversalEntryCard(
              title: Text('Title'),
              leading: Icon(Icons.star),
            ),
          ),
        ),
      );

      expect(find.byIcon(Icons.star), findsOneWidget);
    });

    testWidgets('renders action buttons in top-right', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: UniversalEntryCard(
              title: Text('Title'),
              actions: [
                Icon(Icons.edit),
                Icon(Icons.delete),
              ],
            ),
          ),
        ),
      );

      expect(find.byIcon(Icons.edit), findsOneWidget);
      expect(find.byIcon(Icons.delete), findsOneWidget);
    });

    testWidgets('renders children widgets', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: UniversalEntryCard(
              title: Text('Title'),
              children: [
                Text('Child 1'),
                Text('Child 2'),
              ],
            ),
          ),
        ),
      );

      expect(find.text('Child 1'), findsOneWidget);
      expect(find.text('Child 2'), findsOneWidget);
    });

    testWidgets('renders bottom actions', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: UniversalEntryCard(
              title: Text('Title'),
              bottomActions: [
                Text('Bottom 1'),
              ],
            ),
          ),
        ),
      );

      expect(find.text('Bottom 1'), findsOneWidget);
    });
  });

  group('UniversalEntryTile', () {
    testWidgets('renders title and leading', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: UniversalEntryTile(
              title: Text('Tile Title'),
              leading: Icon(Icons.folder),
            ),
          ),
        ),
      );

      expect(find.text('Tile Title'), findsOneWidget);
      expect(find.byIcon(Icons.folder), findsOneWidget);
    });

    testWidgets('renders actions on the right', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: UniversalEntryTile(
              title: Text('Tile Title'),
              actions: [
                Icon(Icons.more_vert),
              ],
            ),
          ),
        ),
      );

      expect(find.byIcon(Icons.more_vert), findsOneWidget);
    });

    testWidgets('renders children', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: UniversalEntryTile(
              title: Text('Tile Title'),
              children: [
                Text('Detail'),
              ],
            ),
          ),
        ),
      );

      expect(find.text('Detail'), findsOneWidget);
    });
  });
}
