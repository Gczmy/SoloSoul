import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/presentation/widgets/object_tile.dart';

void main() {
  group('ObjectTile', () {
    final testObject = UnifiedObject(
      id: 'obj-1',
      typeId: 'identity',
      name: 'Passport',
      iconName: 'passport',
      childrenIds: ['child-1', 'child-2'],
      createdAt: 1735689600000,
      updatedAt: 1735689600000,
    );

    testWidgets('renders name and type label', (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: ObjectTile(object: testObject),
          ),
        ),
      );

      expect(find.text('Passport'), findsOneWidget);
      expect(find.text('identity'), findsOneWidget);
    });

    testWidgets('shows drag handle by default', (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: ObjectTile(object: testObject),
          ),
        ),
      );

      expect(find.byIcon(Icons.drag_handle), findsOneWidget);
    });

    testWidgets('hides drag handle when showDragHandle is false',
        (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: ObjectTile(
              object: testObject,
              showDragHandle: false,
            ),
          ),
        ),
      );

      expect(find.byIcon(Icons.drag_handle), findsNothing);
    });

    testWidgets('shows children count badge', (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: ObjectTile(object: testObject),
          ),
        ),
      );

      expect(find.text('2'), findsOneWidget);
    });

    testWidgets('hides children count when empty', (tester) async {
      final noChildren = testObject.copyWith(childrenIds: []);
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: ObjectTile(object: noChildren),
          ),
        ),
      );

      expect(find.text('2'), findsNothing);
    });

    testWidgets('shows edit button when onEdit provided', (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: ObjectTile(
              object: testObject,
              onEdit: () {},
            ),
          ),
        ),
      );

      expect(find.byIcon(Icons.edit_outlined), findsOneWidget);
    });

    testWidgets('hides edit button when onEdit is null', (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: ObjectTile(object: testObject),
          ),
        ),
      );

      expect(find.byIcon(Icons.edit_outlined), findsNothing);
    });

    testWidgets('shows delete button when onDelete provided', (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: ObjectTile(
              object: testObject,
              onDelete: () {},
            ),
          ),
        ),
      );

      expect(find.byIcon(Icons.delete_outline), findsOneWidget);
    });

    testWidgets('hides delete button when onDelete is null', (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: ObjectTile(object: testObject),
          ),
        ),
      );

      expect(find.byIcon(Icons.delete_outline), findsNothing);
    });

    testWidgets('calls onTap when tapped', (tester) async {
      var tapped = false;
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: ObjectTile(
              object: testObject,
              onTap: () => tapped = true,
            ),
          ),
        ),
      );

      await tester.tap(find.text('Passport'));
      await tester.pump();
      expect(tapped, true);
    });

    testWidgets('calls onEdit when edit button tapped', (tester) async {
      var edited = false;
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: ObjectTile(
              object: testObject,
              onEdit: () => edited = true,
            ),
          ),
        ),
      );

      await tester.tap(find.byIcon(Icons.edit_outlined));
      await tester.pump();
      expect(edited, true);
    });

    testWidgets('calls onDelete when delete button tapped', (tester) async {
      var deleted = false;
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: ObjectTile(
              object: testObject,
              onDelete: () => deleted = true,
            ),
          ),
        ),
      );

      await tester.tap(find.byIcon(Icons.delete_outline));
      await tester.pump();
      expect(deleted, true);
    });

    testWidgets('renders icon container with primary color tint',
        (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: ObjectTile(object: testObject),
          ),
        ),
      );

      final container = tester.widget<Container>(
        find.descendant(of: find.byType(Card), matching: find.byType(Container)).first,
      );
      final decoration = container.decoration as BoxDecoration;
      expect(decoration.borderRadius, BorderRadius.circular(10));
    });
  });
}
