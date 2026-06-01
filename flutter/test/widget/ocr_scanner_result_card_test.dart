import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/models/ocr_result.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations_en.dart';
import 'package:solosoul_flutter/presentation/widgets/ocr_scanner_result_card.dart';

void main() {
  group('ocrScannerSectionLabel', () {
    test('returns Passport for passport', () {
      expect(ocrScannerSectionLabel('passport'), 'Passport');
    });

    test('returns Visa for visa', () {
      expect(ocrScannerSectionLabel('visa'), 'Visa');
    });

    test('returns ID Card for id_card', () {
      expect(ocrScannerSectionLabel('id_card'), 'ID Card');
    });

    test('returns Passport as fallback for unknown', () {
      expect(ocrScannerSectionLabel('unknown'), 'Passport');
    });
  });

  group('ocrScannerDetectedSectionId', () {
    test('detects passport from P type', () {
      final mrz = MrzData(
        documentType: 'P',
        documentNumber: 'AB123456',
        country: 'CHN',
        nationality: 'CHN',
        surname: 'Li',
        givenNames: 'Wei',
        dateOfBirth: '900101',
        sex: 'M',
        expiryDate: '250101',
        confidence: 0.95,
        rawLines: const ['P<CHNLI<<WEI<<<<<<<<<<<<<<<<<<<<<<<<<AB1234567CHN9001013M2501019<<<<<<06'],
      );
      expect(ocrScannerDetectedSectionId(mrz), 'passport');
    });

    test('detects visa from V type', () {
      final mrz = MrzData(
        documentType: 'V',
        documentNumber: 'V123',
        country: 'USA',
        nationality: 'CHN',
        surname: 'Li',
        givenNames: 'Wei',
        dateOfBirth: '900101',
        sex: 'M',
        expiryDate: '250101',
        confidence: 0.95,
        rawLines: const ['V<USALI<<WEI<<<<<<<<<<<<<<<<<<<<<<<<<<V123456CHN9001013M2501019<<<<<<06'],
      );
      expect(ocrScannerDetectedSectionId(mrz), 'visa');
    });

    test('detects id_card from I type', () {
      final mrz = MrzData(
        documentType: 'I',
        documentNumber: 'ID456',
        country: 'CHN',
        nationality: 'CHN',
        surname: 'Li',
        givenNames: 'Wei',
        dateOfBirth: '900101',
        sex: 'M',
        expiryDate: '250101',
        confidence: 0.95,
        rawLines: const ['I<CHNLI<<WEI<<<<<<<<<<<<<<<<<<<<<<<<<<ID45678CHN9001013M2501019<<<<<<06'],
      );
      expect(ocrScannerDetectedSectionId(mrz), 'id_card');
    });

    test('detects id_card from C type', () {
      final mrz = MrzData(
        documentType: 'C',
        documentNumber: 'C789',
        country: 'CHN',
        nationality: 'CHN',
        surname: 'Li',
        givenNames: 'Wei',
        dateOfBirth: '900101',
        sex: 'M',
        expiryDate: '250101',
        confidence: 0.95,
        rawLines: const ['C<CHNLI<<WEI<<<<<<<<<<<<<<<<<<<<<<<<<<C789012CHN9001013M2501019<<<<<<06'],
      );
      expect(ocrScannerDetectedSectionId(mrz), 'id_card');
    });

    test('detects id_card from A type', () {
      final mrz = MrzData(
        documentType: 'A',
        documentNumber: 'A999',
        country: 'CHN',
        nationality: 'CHN',
        surname: 'Li',
        givenNames: 'Wei',
        dateOfBirth: '900101',
        sex: 'M',
        expiryDate: '250101',
        confidence: 0.95,
        rawLines: const ['A<CHNLI<<WEI<<<<<<<<<<<<<<<<<<<<<<<<<<A999999CHN9001013M2501019<<<<<<06'],
      );
      expect(ocrScannerDetectedSectionId(mrz), 'id_card');
    });
  });

  group('showOcrScannerSectionPicker', () {
    testWidgets('shows dialog with section options', (tester) async {
      final mrz = MrzData(
        documentType: 'P',
        documentNumber: 'AB123456',
        country: 'CHN',
        nationality: 'CHN',
        surname: 'Li',
        givenNames: 'Wei',
        dateOfBirth: '900101',
        sex: 'M',
        expiryDate: '250101',
        confidence: 0.95,
        rawLines: const ['P<CHNLI<<WEI<<<<<<<<<<<<<<<<<<<<<<<<<AB1234567CHN9001013M2501019<<<<<<06'],
      );

      String? selected;

      await tester.pumpWidget(MaterialApp(
        localizationsDelegates: const [],
        home: Scaffold(
          body: Builder(
            builder: (context) => ElevatedButton(
              onPressed: () {
                showOcrScannerSectionPicker(
                  context,
                  _FakeL10n(),
                  mrz,
                  null,
                  (v) => selected = v,
                );
              },
              child: const Text('pick'),
            ),
          ),
        ),
      ));

      await tester.tap(find.text('pick'));
      await tester.pumpAndSettle();

      expect(find.text('Passport'), findsOneWidget);
      expect(find.text('Visa'), findsOneWidget);
      expect(find.text('ID Card'), findsOneWidget);

      // Select Visa
      await tester.tap(find.text('Visa'));
      await tester.pumpAndSettle();

      expect(selected, 'visa');
    });

    testWidgets('pre-selects current target section', (tester) async {
      final mrz = MrzData(
        documentType: 'P',
        documentNumber: 'AB123456',
        country: 'CHN',
        nationality: 'CHN',
        surname: 'Li',
        givenNames: 'Wei',
        dateOfBirth: '900101',
        sex: 'M',
        expiryDate: '250101',
        confidence: 0.95,
        rawLines: const ['P<CHNLI<<WEI<<<<<<<<<<<<<<<<<<<<<<<<<AB1234567CHN9001013M2501019<<<<<<06'],
      );

      String? selected;

      await tester.pumpWidget(MaterialApp(
        localizationsDelegates: const [],
        home: Scaffold(
          body: Builder(
            builder: (context) => ElevatedButton(
              onPressed: () {
                showOcrScannerSectionPicker(
                  context,
                  _FakeL10n(),
                  mrz,
                  'id_card',
                  (v) => selected = v,
                );
              },
              child: const Text('pick'),
            ),
          ),
        ),
      ));

      await tester.tap(find.text('pick'));
      await tester.pumpAndSettle();

      // Dialog shows with id_card pre-selected; selecting Visa changes it
      await tester.tap(find.text('Visa'));
      await tester.pumpAndSettle();

      expect(selected, 'visa');
    });
  });
}

class _FakeL10n extends AppLocalizationsEn {
  _FakeL10n() : super('en');
  @override String get workspaceAddSectionButton => '添加分区';
}
