import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/models/ocr_result.dart';
import 'package:solosoul_flutter/core/services/document_field_extractor.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/widgets/extracted_fields_preview.dart';

Widget wrap(Widget child) {
  return MaterialApp(
    localizationsDelegates: AppLocalizations.localizationsDelegates,
    supportedLocales: AppLocalizations.supportedLocales,
    home: Scaffold(body: child),
  );
}

void main() {
  group('ExtractedFieldsPreview', () {
    testWidgets('renders document type badge and fields', (tester) async {
      final result = ExtractionResult(
        documentType: 'business_card',
        fields: {
          'name': ExtractedField(
            value: 'Alice',
            bbox: const BoundingBox(x: 0, y: 0, width: 1, height: 1),
          ),
          'phone': ExtractedField(
            value: '123-456',
            bbox: const BoundingBox(x: 0, y: 0, width: 1, height: 1),
          ),
        },
        rawText: 'Alice 123-456',
      );

      await tester.pumpWidget(wrap(ExtractedFieldsPreview(
        result: result,
        selectedKeys: const {'name'},
        onToggle: (_) {},
      )));

      expect(find.text('Business Card Detected'), findsOneWidget);
      expect(find.text('Alice'), findsOneWidget);
      expect(find.text('123-456'), findsOneWidget);
    });

    testWidgets('shows empty state when no fields', (tester) async {
      final result = ExtractionResult(
        documentType: 'generic',
        fields: const {},
        rawText: 'raw',
      );

      await tester.pumpWidget(wrap(ExtractedFieldsPreview(
        result: result,
        selectedKeys: const {},
        onToggle: (_) {},
      )));

      expect(find.textContaining('No structured fields detected'), findsOneWidget);
    });

    testWidgets('toggles field selection on tap', (tester) async {
      final result = ExtractionResult(
        documentType: 'invoice',
        fields: {
          'total': ExtractedField(
            value: '100.00',
            bbox: const BoundingBox(x: 0, y: 0, width: 1, height: 1),
          ),
        },
        rawText: 'Total: 100.00',
      );

      String? toggledKey;
      await tester.pumpWidget(wrap(ExtractedFieldsPreview(
        result: result,
        selectedKeys: const {},
        onToggle: (key) => toggledKey = key,
      )));

      await tester.tap(find.text('100.00'));
      await tester.pump();
      expect(toggledKey, equals('total'));
    });

    testWidgets('renders invoice type badge', (tester) async {
      final result = ExtractionResult(
        documentType: 'invoice',
        fields: const {},
        rawText: '',
      );

      await tester.pumpWidget(wrap(ExtractedFieldsPreview(
        result: result,
        selectedKeys: const {},
        onToggle: (_) {},
      )));

      expect(find.text('Invoice Detected'), findsOneWidget);
    });

    testWidgets('renders resume type badge', (tester) async {
      final result = ExtractionResult(
        documentType: 'resume',
        fields: const {},
        rawText: '',
      );

      await tester.pumpWidget(wrap(ExtractedFieldsPreview(
        result: result,
        selectedKeys: const {},
        onToggle: (_) {},
      )));

      expect(find.text('Resume Detected'), findsOneWidget);
    });

    testWidgets('renders generic type badge fallback', (tester) async {
      final result = ExtractionResult(
        documentType: 'unknown',
        fields: const {},
        rawText: '',
      );

      await tester.pumpWidget(wrap(ExtractedFieldsPreview(
        result: result,
        selectedKeys: const {},
        onToggle: (_) {},
      )));

      expect(find.text('Document Detected'), findsOneWidget);
    });

    testWidgets('checkbox reflects selected state', (tester) async {
      final result = ExtractionResult(
        documentType: 'business_card',
        fields: {
          'email': ExtractedField(
            value: 'a@b.com',
            bbox: const BoundingBox(x: 0, y: 0, width: 1, height: 1),
          ),
        },
        rawText: '',
      );

      await tester.pumpWidget(wrap(ExtractedFieldsPreview(
        result: result,
        selectedKeys: const {'email'},
        onToggle: (_) {},
      )));

      final checkbox = tester.widget<Checkbox>(find.byType(Checkbox));
      expect(checkbox.value, isTrue);
    });
  });
}
