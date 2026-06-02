import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/models/ocr_result.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/widgets/mrz_preview_card.dart';

Widget wrap(Widget child) {
  return MaterialApp(
    localizationsDelegates: AppLocalizations.localizationsDelegates,
    supportedLocales: AppLocalizations.supportedLocales,
    home: Scaffold(body: child),
  );
}

void main() {
  group('MrzPreviewCard', () {
    testWidgets('renders all MRZ fields', (tester) async {
      const mrzData = MrzData(
        documentType: 'P',
        country: 'CHN',
        surname: 'ZHANG',
        givenNames: 'SAN',
        documentNumber: 'E12345678',
        nationality: 'CHN',
        dateOfBirth: '900101',
        sex: 'M',
        expiryDate: '300101',
        confidence: 0.97,
        rawLines: ['P<CHNZHANG<<SAN<<<<<<<<<<<<<<<<<<<<<<<<<<<<<', 'E12345678<8CHN9001014M3001017<<<<<<<<<<<<<<04'],
      );

      await tester.pumpWidget(wrap(const MrzPreviewCard(mrzData: mrzData)));

      expect(find.text('Recognized Information'), findsOneWidget);
      expect(find.text('Passport'), findsOneWidget);
      expect(find.text('E12345678'), findsOneWidget);
      expect(find.text('ZHANG'), findsOneWidget);
      expect(find.text('SAN'), findsOneWidget);
      expect(find.text('CHN'), findsOneWidget); // nationality only
      expect(find.text('1990-01-01'), findsOneWidget);
      expect(find.text('Male'), findsOneWidget);
      expect(find.text('2030-01-01'), findsOneWidget);
      expect(find.text('97%'), findsOneWidget);
    });

    testWidgets('renders unknown document type as-is', (tester) async {
      const mrzData = MrzData(
        documentType: 'X',
        country: 'USA',
        surname: 'DOE',
        givenNames: 'JOHN',
        documentNumber: 'X999',
        nationality: 'USA',
        dateOfBirth: '850512',
        sex: 'X',
        expiryDate: '250630',
        confidence: 0.82,
        rawLines: [],
      );

      await tester.pumpWidget(wrap(const MrzPreviewCard(mrzData: mrzData)));

      expect(find.text('X'), findsOneWidget); // document type
      expect(find.text('Unspecified'), findsOneWidget); // sex
      expect(find.text('82%'), findsOneWidget);
    });

    testWidgets('formats short date-of-birth as-is', (tester) async {
      const mrzData = MrzData(
        documentType: 'P',
        country: 'CHN',
        surname: 'A',
        givenNames: 'B',
        documentNumber: 'C',
        nationality: 'CHN',
        dateOfBirth: '12345',
        sex: 'F',
        expiryDate: 'ABCDE',
        confidence: 0.5,
        rawLines: [],
      );

      await tester.pumpWidget(wrap(const MrzPreviewCard(mrzData: mrzData)));

      expect(find.text('12345'), findsOneWidget);
      expect(find.text('ABCDE'), findsOneWidget);
    });

    testWidgets('expands raw lines section on tap', (tester) async {
      const mrzData = MrzData(
        documentType: 'P',
        country: 'CHN',
        surname: 'A',
        givenNames: 'B',
        documentNumber: 'C',
        nationality: 'CHN',
        dateOfBirth: '900101',
        sex: 'M',
        expiryDate: '300101',
        confidence: 1.0,
        rawLines: ['LINE1', 'LINE2'],
      );

      await tester.pumpWidget(wrap(const MrzPreviewCard(mrzData: mrzData)));

      expect(find.text('LINE1'), findsNothing);
      await tester.tap(find.text('Raw MRZ Lines'));
      await tester.pumpAndSettle();
      expect(find.text('LINE1'), findsOneWidget);
      expect(find.text('LINE2'), findsOneWidget);
    });

    testWidgets('displays orange badge for medium confidence', (tester) async {
      const mrzData = MrzData(
        documentType: 'P',
        country: 'CHN',
        surname: 'A',
        givenNames: 'B',
        documentNumber: 'C',
        nationality: 'CHN',
        dateOfBirth: '900101',
        sex: 'M',
        expiryDate: '300101',
        confidence: 0.90,
        rawLines: [],
      );

      await tester.pumpWidget(wrap(const MrzPreviewCard(mrzData: mrzData)));
      expect(find.text('90%'), findsOneWidget);
    });

    testWidgets('displays red badge for low confidence', (tester) async {
      const mrzData = MrzData(
        documentType: 'P',
        country: 'CHN',
        surname: 'A',
        givenNames: 'B',
        documentNumber: 'C',
        nationality: 'CHN',
        dateOfBirth: '900101',
        sex: 'M',
        expiryDate: '300101',
        confidence: 0.80,
        rawLines: [],
      );

      await tester.pumpWidget(wrap(const MrzPreviewCard(mrzData: mrzData)));
      expect(find.text('80%'), findsOneWidget);
    });

    testWidgets('shows dash for empty values', (tester) async {
      const mrzData = MrzData(
        documentType: 'P',
        country: '',
        surname: '',
        givenNames: '',
        documentNumber: '',
        nationality: '',
        dateOfBirth: '',
        sex: '',
        expiryDate: '',
        confidence: 1.0,
        rawLines: [],
      );

      await tester.pumpWidget(wrap(const MrzPreviewCard(mrzData: mrzData)));
      expect(find.text('-'), findsWidgets);
    });
  });
}
