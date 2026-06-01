import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/widgets/object_card/object_card_header.dart';

Widget wrap(Widget child) {
  return MaterialApp(
    localizationsDelegates: AppLocalizations.localizationsDelegates,
    supportedLocales: AppLocalizations.supportedLocales,
    home: Scaffold(body: child),
  );
}

void main() {
  group('ObjectCardHeader', () {
    final object = UnifiedObject(
      id: 'test-1',
      name: 'Test Object',
      iconName: 'folder',
      createdAt: DateTime(2024, 1, 1).millisecondsSinceEpoch,
      updatedAt: DateTime(2024, 1, 1).millisecondsSinceEpoch,
    );

    testWidgets('renders object name and icon', (tester) async {
      await tester.pumpWidget(wrap(ObjectCardHeader(
        object: object,
        onChangeIcon: () {},
        onEdit: () {},
        onDelete: () {},
        onAddItem: () {},
        showEditActions: false,
        showAddButton: false,
      )));

      expect(find.text('Test Object'), findsOneWidget);
      expect(find.byIcon(Icons.folder_outlined), findsOneWidget);
    });

    testWidgets('shows edit and delete buttons when enabled', (tester) async {
      await tester.pumpWidget(wrap(ObjectCardHeader(
        object: object,
        onChangeIcon: () {},
        onEdit: () {},
        onDelete: () {},
        onAddItem: () {},
        showEditActions: true,
        showAddButton: false,
      )));

      expect(find.byIcon(Icons.edit_outlined), findsOneWidget);
      expect(find.byIcon(Icons.delete_outline), findsOneWidget);
    });

    testWidgets('shows add button when enabled', (tester) async {
      await tester.pumpWidget(wrap(ObjectCardHeader(
        object: object,
        onChangeIcon: () {},
        onEdit: () {},
        onDelete: () {},
        onAddItem: () {},
        showEditActions: false,
        showAddButton: true,
      )));

      expect(find.byIcon(Icons.add), findsOneWidget);
    });

    testWidgets('shows edit section button in edit section mode', (tester) async {
      await tester.pumpWidget(wrap(ObjectCardHeader(
        object: object,
        onChangeIcon: () {},
        onEdit: () {},
        onDelete: () {},
        onAddItem: () {},
        showEditActions: true,
        showAddButton: true,
        showEditSection: true,
      )));

      expect(find.byIcon(Icons.edit_note), findsOneWidget);
    });

    testWidgets('taps trigger callbacks', (tester) async {
      bool changedIcon = false;
      bool edited = false;
      bool deleted = false;
      bool added = false;

      await tester.pumpWidget(wrap(ObjectCardHeader(
        object: object,
        onChangeIcon: () => changedIcon = true,
        onEdit: () => edited = true,
        onDelete: () => deleted = true,
        onAddItem: () => added = true,
        showEditActions: true,
        showAddButton: true,
      )));

      await tester.tap(find.byIcon(Icons.folder_outlined));
      await tester.pump();
      expect(changedIcon, isTrue);

      await tester.tap(find.byIcon(Icons.edit_outlined));
      await tester.pump();
      expect(edited, isTrue);

      await tester.tap(find.byIcon(Icons.delete_outline));
      await tester.pump();
      expect(deleted, isTrue);

      await tester.tap(find.byIcon(Icons.add));
      await tester.pump();
      expect(added, isTrue);
    });
  });
}
