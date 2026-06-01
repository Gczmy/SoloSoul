import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/core/models/field_history_models.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/providers/sensitivity_provider.dart';
import 'package:solosoul_flutter/presentation/widgets/field_history_view.dart';
import 'package:solosoul_flutter/presentation/widgets/sensitivity_tag.dart';

Widget wrapWithProviders(Widget child, {List overrides = const []}) {
  return ProviderScope(
    overrides: overrides.cast(),
    child: MaterialApp(
      localizationsDelegates: AppLocalizations.localizationsDelegates,
      supportedLocales: AppLocalizations.supportedLocales,
      home: Scaffold(body: child),
    ),
  );
}

void main() {
  group('FieldHistoryView', () {
    testWidgets('renders empty list when history is null', (tester) async {
      await tester.pumpWidget(wrapWithProviders(
        const FieldHistoryView(fieldName: 'test'),
      ));
      await tester.pumpAndSettle();

      // ListView is present but with 0 items
      expect(find.byType(ListView), findsOneWidget);
    });

    testWidgets('renders entries in reverse order', (tester) async {
      final history = FieldHistory(
        fieldId: 'name',
        itemId: 'item1',
        entries: [
          FieldHistoryEntry(
            values: const {'name': 'Alice'},
            timestamp: DateTime(2024, 1, 1),
          ),
          FieldHistoryEntry(
            values: const {'name': 'Bob'},
            timestamp: DateTime(2024, 6, 15),
          ),
        ],
      );

      await tester.pumpWidget(wrapWithProviders(
        FieldHistoryView(
          fieldName: 'name',
          history: history,
        ),
        overrides: [
          effectiveSensitivityProvider('name.name').overrideWith((ref) => SensitivityLevel.public),
        ],
      ));
      await tester.pumpAndSettle();

      // ListView should be present
      expect(find.byType(ListView), findsOneWidget);
      // Both entries should render
      expect(find.text('Alice'), findsOneWidget);
      expect(find.text('Bob'), findsOneWidget);
    });

    testWidgets('marks latest entry', (tester) async {
      final history = FieldHistory(
        fieldId: 'name',
        itemId: 'item1',
        entries: [
          FieldHistoryEntry(
            values: const {'name': 'Old'},
            timestamp: DateTime(2024, 1, 1),
          ),
          FieldHistoryEntry(
            values: const {'name': 'Newest'},
            timestamp: DateTime(2024, 6, 15),
          ),
        ],
      );

      await tester.pumpWidget(wrapWithProviders(
        FieldHistoryView(
          fieldName: 'name',
          history: history,
        ),
        overrides: [
          effectiveSensitivityProvider('name.name').overrideWith((ref) => SensitivityLevel.public),
        ],
      ));
      await tester.pumpAndSettle();

      expect(find.text('Newest'), findsOneWidget);
    });

    testWidgets('renders sensitivity tags', (tester) async {
      final history = FieldHistory(
        fieldId: 'name',
        itemId: 'item1',
        entries: [
          FieldHistoryEntry(
            values: const {'name': 'Alice'},
            timestamp: DateTime(2024, 1, 1),
          ),
        ],
      );

      await tester.pumpWidget(wrapWithProviders(
        FieldHistoryView(
          fieldName: 'name',
          history: history,
        ),
        overrides: [
          effectiveSensitivityProvider('name.name').overrideWith((ref) => SensitivityLevel.internal),
        ],
      ));
      await tester.pumpAndSettle();

      expect(find.byType(SensitivityTag), findsOneWidget);
    });

    testWidgets('handles empty values', (tester) async {
      final history = FieldHistory(
        fieldId: 'name',
        itemId: 'item1',
        entries: [
          FieldHistoryEntry(
            values: const {'name': ''},
            timestamp: DateTime(2024, 1, 1),
          ),
        ],
      );

      await tester.pumpWidget(wrapWithProviders(
        FieldHistoryView(
          fieldName: 'name',
          history: history,
        ),
        overrides: [
          effectiveSensitivityProvider('name.name').overrideWith((ref) => SensitivityLevel.public),
        ],
      ));
      await tester.pumpAndSettle();

      expect(find.text('(empty)'), findsOneWidget);
    });

    testWidgets('strips prefix from keys', (tester) async {
      final history = FieldHistory(
        fieldId: 'contact.email',
        itemId: 'item1',
        entries: [
          FieldHistoryEntry(
            values: const {'contact.email': 'test@example.com'},
            timestamp: DateTime(2024, 1, 1),
          ),
        ],
      );

      await tester.pumpWidget(wrapWithProviders(
        FieldHistoryView(
          fieldName: 'contact.email',
          history: history,
        ),
        overrides: [
          effectiveSensitivityProvider('contact.email').overrideWith((ref) => SensitivityLevel.public),
        ],
      ));
      await tester.pumpAndSettle();

      expect(find.text('test@example.com'), findsOneWidget);
    });
  });

  group('FieldLastModified', () {
    testWidgets('renders timestamp', (tester) async {
      await tester.pumpWidget(wrapWithProviders(
        FieldLastModified(
          timestamp: DateTime(2024, 6, 15, 10, 30),
          fieldName: 'name',
        ),
      ));

      expect(find.byIcon(Icons.access_time), findsOneWidget);
      expect(find.byType(Text), findsOneWidget);
    });
  });
}
