import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/core/models/field_history_models.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/providers/account_style_provider.dart';
import 'package:solosoul_flutter/presentation/widgets/field_history_dialog.dart';
import 'package:solosoul_flutter/presentation/widgets/form_field_def.dart';

Widget wrap(Widget child) {
  return ProviderScope(
    overrides: [
      effectiveSensitivityProvider('test.name').overrideWith((ref) => SensitivityLevel.public),
      effectiveSensitivityProvider('test.email').overrideWith((ref) => SensitivityLevel.public),
    ],
    child: MaterialApp(
      localizationsDelegates: AppLocalizations.localizationsDelegates,
      supportedLocales: AppLocalizations.supportedLocales,
      home: Scaffold(body: child),
    ),
  );
}

void main() {
  group('FieldHistoryDialog', () {
    testWidgets('shows empty state when no history', (tester) async {
      await tester.pumpWidget(wrap(const FieldHistoryDialog(
        title: 'Test History',
        icon: Icons.history,
        fieldDefs: [],
        history: null,
      )));

      expect(find.byType(AlertDialog), findsOneWidget);
    });

    testWidgets('renders history entries', (tester) async {
      final history = FieldHistory(
        fieldId: 'test',
        itemId: 'item1',
        entries: [
          FieldHistoryEntry(
            values: {'name': 'Alice', 'email': 'a@b.com'},
            timestamp: DateTime(2024, 6, 15, 10, 30),
          ),
          FieldHistoryEntry(
            values: {'name': 'Bob', 'email': 'b@c.com'},
            timestamp: DateTime(2024, 6, 14, 9, 0),
          ),
        ],
      );

      await tester.pumpWidget(wrap(FieldHistoryDialog(
        title: 'Test History',
        icon: Icons.history,
        fieldDefs: [
          const FormFieldDef(fieldId: 'name', label: 'Name'),
          const FormFieldDef(fieldId: 'email', label: 'Email'),
        ],
        history: history,
        fieldPrefix: 'test',
      )));

      expect(find.text('Alice'), findsOneWidget);
      expect(find.text('a@b.com'), findsOneWidget);
      expect(find.text('Bob'), findsOneWidget);
    });

    testWidgets('marks latest entry with badge', (tester) async {
      final history = FieldHistory(
        fieldId: 'test',
        itemId: 'item1',
        entries: [
          FieldHistoryEntry(
            values: {'name': 'Alice'},
            timestamp: DateTime(2024, 6, 15),
          ),
        ],
      );

      await tester.pumpWidget(wrap(FieldHistoryDialog(
        title: 'Test History',
        icon: Icons.history,
        fieldDefs: [const FormFieldDef(fieldId: 'name', label: 'Name')],
        history: history,
      )));

      expect(find.byType(AlertDialog), findsOneWidget);
    });

    testWidgets('close button dismisses dialog', (tester) async {
      await tester.pumpWidget(wrap(const FieldHistoryDialog(
        title: 'Test History',
        icon: Icons.history,
        fieldDefs: [],
        history: null,
      )));

      await tester.tap(find.text('Close'));
      await tester.pump();
    });
  });
}
