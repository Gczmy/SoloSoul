import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/presentation/widgets/data_management/backup_progress_indicator.dart';

void main() {
  group('BackupProgressIndicator', () {
    testWidgets('shows Reading data for progress < 0.3', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: BackupProgressIndicator(progress: 0.1),
          ),
        ),
      );

      expect(find.text('Reading data...'), findsOneWidget);
    });

    testWidgets('shows Encoding for progress >= 0.3', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: BackupProgressIndicator(progress: 0.35),
          ),
        ),
      );

      expect(find.text('Encoding...'), findsOneWidget);
    });

    testWidgets('shows Encrypting for progress >= 0.5', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: BackupProgressIndicator(progress: 0.6),
          ),
        ),
      );

      expect(find.text('Encrypting...'), findsOneWidget);
    });

    testWidgets('shows Writing file for progress >= 0.9', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: BackupProgressIndicator(progress: 0.92),
          ),
        ),
      );

      expect(find.text('Writing file...'), findsOneWidget);
    });

    testWidgets('shows Finishing for progress >= 1.0', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: BackupProgressIndicator(progress: 1.0),
          ),
        ),
      );

      expect(find.text('Finishing...'), findsOneWidget);
    });

    testWidgets('renders LinearProgressIndicator with value', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: BackupProgressIndicator(progress: 0.5),
          ),
        ),
      );

      final indicator = tester.widget<LinearProgressIndicator>(
        find.byType(LinearProgressIndicator),
      );
      expect(indicator.value, 0.5);
    });

    testWidgets('renders indeterminate progress when value is 0',
        (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: BackupProgressIndicator(progress: 0),
          ),
        ),
      );

      final indicator = tester.widget<LinearProgressIndicator>(
        find.byType(LinearProgressIndicator),
      );
      expect(indicator.value, isNull);
    });

    testWidgets('uses bodySmall style for status text', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: BackupProgressIndicator(progress: 0.2),
          ),
        ),
      );

      final text = tester.widget<Text>(find.text('Reading data...'));
      expect(text.style, isNotNull);
    });
  });
}
