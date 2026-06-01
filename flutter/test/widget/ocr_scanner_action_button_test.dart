import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/presentation/widgets/ocr_scanner_action_button.dart';

void main() {
  group('OcrScannerActionButton', () {
    testWidgets('renders icon, label, description and chevron', (tester) async {
      await tester.pumpWidget(MaterialApp(
        home: Scaffold(
          body: OcrScannerActionButton(
            icon: Icons.camera_alt,
            label: 'Camera',
            description: 'Scan with camera',
            onTap: () {},
          ),
        ),
      ));

      expect(find.byIcon(Icons.camera_alt), findsOneWidget);
      expect(find.text('Camera'), findsOneWidget);
      expect(find.text('Scan with camera'), findsOneWidget);
      expect(find.byIcon(Icons.chevron_right), findsOneWidget);
    });

    testWidgets('calls onTap when tapped', (tester) async {
      bool tapped = false;

      await tester.pumpWidget(MaterialApp(
        home: Scaffold(
          body: OcrScannerActionButton(
            icon: Icons.camera_alt,
            label: 'Camera',
            description: 'Scan with camera',
            onTap: () => tapped = true,
          ),
        ),
      ));

      await tester.tap(find.byType(InkWell));
      expect(tapped, isTrue);
    });

    testWidgets('uses Card with InkWell', (tester) async {
      await tester.pumpWidget(MaterialApp(
        home: Scaffold(
          body: OcrScannerActionButton(
            icon: Icons.camera_alt,
            label: 'Camera',
            description: 'Scan with camera',
            onTap: () {},
          ),
        ),
      ));

      expect(find.byType(Card), findsOneWidget);
      expect(find.byType(InkWell), findsOneWidget);
    });
  });
}
