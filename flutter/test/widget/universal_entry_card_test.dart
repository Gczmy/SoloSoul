import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/presentation/widgets/universal_entry_card.dart';

Widget wrap(Widget child) {
  return MaterialApp(
    home: Scaffold(body: child),
  );
}

void main() {
  group('UniversalEntryCard', () {
    testWidgets('renders title only', (tester) async {
      await tester.pumpWidget(wrap(const UniversalEntryCard(
        title: Text('Hello'),
      )));

      expect(find.text('Hello'), findsOneWidget);
    });

    testWidgets('renders subtitle and children', (tester) async {
      await tester.pumpWidget(wrap(const UniversalEntryCard(
        title: Text('Title'),
        subtitle: Text('Subtitle'),
        children: [Text('Child 1'), Text('Child 2')],
      )));

      expect(find.text('Title'), findsOneWidget);
      expect(find.text('Subtitle'), findsOneWidget);
      expect(find.text('Child 1'), findsOneWidget);
      expect(find.text('Child 2'), findsOneWidget);
    });

    testWidgets('renders leading widget', (tester) async {
      await tester.pumpWidget(wrap(const UniversalEntryCard(
        title: Text('Title'),
        leading: Icon(Icons.person),
      )));

      expect(find.byIcon(Icons.person), findsOneWidget);
    });

    testWidgets('renders action buttons', (tester) async {
      await tester.pumpWidget(wrap(UniversalEntryCard(
        title: const Text('Title'),
        actions: [
          IconButton(icon: const Icon(Icons.edit), onPressed: () {}),
        ],
      )));

      expect(find.byIcon(Icons.edit), findsOneWidget);
    });

    testWidgets('renders bottom actions', (tester) async {
      await tester.pumpWidget(wrap(const UniversalEntryCard(
        title: Text('Title'),
        bottomActions: [Text('Bottom')],
      )));

      expect(find.text('Bottom'), findsOneWidget);
    });
  });

  group('UniversalEntryTile', () {
    testWidgets('renders title and actions in row', (tester) async {
      await tester.pumpWidget(wrap(UniversalEntryTile(
        title: const Text('Tile'),
        actions: [
          IconButton(icon: const Icon(Icons.delete), onPressed: () {}),
        ],
      )));

      expect(find.text('Tile'), findsOneWidget);
      expect(find.byIcon(Icons.delete), findsOneWidget);
    });
  });
}
