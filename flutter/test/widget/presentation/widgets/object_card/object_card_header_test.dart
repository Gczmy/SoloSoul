import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/widgets/object_card/object_card_header.dart';

void main() {
  group('ObjectCardHeader', () {
    final object = UnifiedObject(
      id: 'o1',
      typeId: 'item',
      name: 'Test Object',
      iconName: 'folder',
      parentId: null,
      childrenIds: const [],
      properties: const {},
      isDeleted: false,
      deletedAt: null,
      createdAt: 0,
      updatedAt: 0,
    );

    testWidgets('renders icon and name', (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          home: Scaffold(
            body: ObjectCardHeader(
              object: object,
              onChangeIcon: () {},
              onEdit: () {},
              onDelete: () {},
              onAddItem: () {},
              showEditActions: false,
              showAddButton: false,
            ),
          ),
        ),
      );

      expect(find.text('Test Object'), findsOneWidget);
      expect(find.byType(InkWell), findsOneWidget); // Icon tap target
    });

    testWidgets('shows edit and delete buttons when showEditActions is true', (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          home: Scaffold(
            body: ObjectCardHeader(
              object: object,
              onChangeIcon: () {},
              onEdit: () {},
              onDelete: () {},
              onAddItem: () {},
              showEditActions: true,
              showAddButton: false,
            ),
          ),
        ),
      );

      expect(find.byIcon(Icons.edit_outlined), findsOneWidget);
      expect(find.byIcon(Icons.delete_outline), findsOneWidget);
    });

    testWidgets('hides edit button when showEditSection is true', (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          home: Scaffold(
            body: ObjectCardHeader(
              object: object,
              onChangeIcon: () {},
              onEdit: () {},
              onDelete: () {},
              onAddItem: () {},
              showEditActions: true,
              showAddButton: false,
              showEditSection: true,
            ),
          ),
        ),
      );

      expect(find.byIcon(Icons.edit_outlined), findsNothing);
      expect(find.byIcon(Icons.delete_outline), findsOneWidget);
    });

    testWidgets('shows add button when showAddButton is true', (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          home: Scaffold(
            body: ObjectCardHeader(
              object: object,
              onChangeIcon: () {},
              onEdit: () {},
              onDelete: () {},
              onAddItem: () {},
              showEditActions: false,
              showAddButton: true,
            ),
          ),
        ),
      );

      expect(find.byIcon(Icons.add), findsOneWidget);
    });

    testWidgets('shows edit_note button when showAddButton and showEditSection', (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          home: Scaffold(
            body: ObjectCardHeader(
              object: object,
              onChangeIcon: () {},
              onEdit: () {},
              onDelete: () {},
              onAddItem: () {},
              showEditActions: false,
              showAddButton: true,
              showEditSection: true,
            ),
          ),
        ),
      );

      expect(find.byIcon(Icons.edit_note), findsOneWidget);
      expect(find.byIcon(Icons.add), findsNothing);
    });

    testWidgets('calls onChangeIcon when icon tapped', (tester) async {
      var called = false;
      await tester.pumpWidget(
        MaterialApp(
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          home: Scaffold(
            body: ObjectCardHeader(
              object: object,
              onChangeIcon: () => called = true,
              onEdit: () {},
              onDelete: () {},
              onAddItem: () {},
              showEditActions: false,
              showAddButton: false,
            ),
          ),
        ),
      );

      await tester.tap(find.byType(InkWell));
      expect(called, isTrue);
    });

    testWidgets('calls onEdit when edit button tapped', (tester) async {
      var called = false;
      await tester.pumpWidget(
        MaterialApp(
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          home: Scaffold(
            body: ObjectCardHeader(
              object: object,
              onChangeIcon: () {},
              onEdit: () => called = true,
              onDelete: () {},
              onAddItem: () {},
              showEditActions: true,
              showAddButton: false,
            ),
          ),
        ),
      );

      await tester.tap(find.byIcon(Icons.edit_outlined));
      expect(called, isTrue);
    });

    testWidgets('calls onDelete when delete button tapped', (tester) async {
      var called = false;
      await tester.pumpWidget(
        MaterialApp(
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          home: Scaffold(
            body: ObjectCardHeader(
              object: object,
              onChangeIcon: () {},
              onEdit: () {},
              onDelete: () => called = true,
              onAddItem: () {},
              showEditActions: true,
              showAddButton: false,
            ),
          ),
        ),
      );

      await tester.tap(find.byIcon(Icons.delete_outline));
      expect(called, isTrue);
    });

    testWidgets('calls onAddItem when add button tapped', (tester) async {
      var called = false;
      await tester.pumpWidget(
        MaterialApp(
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          home: Scaffold(
            body: ObjectCardHeader(
              object: object,
              onChangeIcon: () {},
              onEdit: () {},
              onDelete: () {},
              onAddItem: () => called = true,
              showEditActions: false,
              showAddButton: true,
            ),
          ),
        ),
      );

      await tester.tap(find.byIcon(Icons.add));
      expect(called, isTrue);
    });
  });
}
