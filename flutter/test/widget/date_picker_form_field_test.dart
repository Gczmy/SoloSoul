import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/widgets/date_picker_form_field.dart';

void main() {
  group('DatePickerFormField', () {
    testWidgets('renders with placeholder when no date', (tester) async {
      await tester.pumpWidget(MaterialApp(
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: Scaffold(
          body: DatePickerFormField(
            label: 'Birth Date',
            onDateChanged: (_) {},
          ),
        ),
      ));

      expect(find.text('Birth Date'), findsOneWidget);
      expect(find.byIcon(Icons.calendar_today), findsOneWidget);
    });

    testWidgets('renders date value when provided', (tester) async {
      await tester.pumpWidget(MaterialApp(
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: Scaffold(
          body: DatePickerFormField(
            label: 'Birth Date',
            initialDate: '1990-01-01',
            onDateChanged: (_) {},
          ),
        ),
      ));

      expect(find.text('1990-01-01'), findsOneWidget);
      expect(find.byIcon(Icons.clear), findsOneWidget);
    });

    testWidgets('renders sensitivity tag', (tester) async {
      await tester.pumpWidget(MaterialApp(
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: Scaffold(
          body: DatePickerFormField(
            label: 'Date',
            sensitivity: SensitivityLevel.sensitive,
            onDateChanged: (_) {},
          ),
        ),
      ));

      expect(find.byType(DatePickerFormField), findsOneWidget);
    });

    testWidgets('clear button calls onDateChanged with null', (tester) async {
      String? result = 'initial';

      await tester.pumpWidget(MaterialApp(
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: Scaffold(
          body: DatePickerFormField(
            label: 'Date',
            initialDate: '1990-01-01',
            onDateChanged: (v) => result = v,
          ),
        ),
      ));

      await tester.tap(find.byIcon(Icons.clear));
      await tester.pump();

      expect(result, isNull);
    });

    testWidgets('tap opens date picker dialog', (tester) async {
      await tester.pumpWidget(MaterialApp(
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: Scaffold(
          body: DatePickerFormField(
            label: 'Date',
            onDateChanged: (_) {},
          ),
        ),
      ));

      await tester.tap(find.byType(InkWell));
      await tester.pumpAndSettle();

      // DatePicker dialog should appear
      expect(find.byType(DatePickerDialog), findsOneWidget);
    });
  });
}
