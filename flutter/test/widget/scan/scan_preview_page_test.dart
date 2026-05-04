import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/core/models/scan/scan_result_model.dart';
import 'package:solosoul_flutter/presentation/pages/scan/scan_preview_page.dart';
import 'package:solosoul_flutter/presentation/providers/scan/local_search_provider.dart';
import 'package:solosoul_flutter/presentation/providers/scan/local_search_state.dart';

void main() {
  group('ScanPreviewPage', () {
    testWidgets('renders empty state when no candidates', (tester) async {
      await tester.pumpWidget(
        ProviderScope(
          child: MaterialApp(
            home: const ScanPreviewPage(),
          ),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.text('No importable items found'), findsOneWidget);
      expect(find.text('Back'), findsOneWidget);
    });

    testWidgets('renders candidate cards with fields', (tester) async {
      final container = ProviderContainer();
      addTearDown(container.dispose);

      // Set up state via notifier
      final notifier = container.read(localSearchProvider.notifier);
      final scanResult = ScanResult(
        meta: ScanMeta(
          scanId: 'test-scan',
          createdAt: 0,
          sourceFile: '/test/resume.pdf',
          confidence: 0.9,
        ),
        sections: [
          ScanSection(
            section: 'identity',
            display: 'Personal Information',
            fields: [
              ScanField(
                key: 'fullName',
                value: 'Zhang San',
                sensitivity: SensitivityLevel.public,
                confidence: 0.95,
              ),
              ScanField(
                key: 'idCard',
                value: '110101199001011234',
                sensitivity: SensitivityLevel.critical,
                confidence: 0.99,
              ),
            ],
          ),
        ],
      );

      notifier.state = LocalSearchState(
        scanResults: [scanResult],
        importCandidates: [
          ImportCandidate(
            source: scanResult.sections.first,
            fields: scanResult.sections.first.fields.map((f) {
              return ImportFieldCandidate(
                source: f,
                targetPropertyId: f.key,
                suggestedAction: ImportAction.createNew,
              );
            }).toList(),
          ),
        ],
      );

      await tester.pumpWidget(
        UncontrolledProviderScope(
          container: container,
          child: MaterialApp(
            home: const ScanPreviewPage(),
          ),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.text('Personal Information'), findsOneWidget);
      expect(find.text('fullName'), findsOneWidget);
      expect(find.text('idCard'), findsOneWidget);
      expect(find.text('New'), findsOneWidget);
    });

    testWidgets('masks critical field values', (tester) async {
      final container = ProviderContainer();
      addTearDown(container.dispose);

      final notifier = container.read(localSearchProvider.notifier);
      final scanResult = ScanResult(
        meta: ScanMeta(
          scanId: 'test-scan',
          createdAt: 0,
          sourceFile: '/test/resume.pdf',
          confidence: 0.9,
        ),
        sections: [
          ScanSection(
            section: 'identity',
            display: 'Personal Information',
            fields: [
              ScanField(
                key: 'idCard',
                value: '110101199001011234',
                sensitivity: SensitivityLevel.critical,
                confidence: 0.99,
              ),
            ],
          ),
        ],
      );

      notifier.state = LocalSearchState(
        scanResults: [scanResult],
        importCandidates: [
          ImportCandidate(
            source: scanResult.sections.first,
            fields: [
              ImportFieldCandidate(
                source: scanResult.sections.first.fields.first,
                targetPropertyId: 'idCard',
                suggestedAction: ImportAction.createNew,
              ),
            ],
          ),
        ],
      );

      await tester.pumpWidget(
        UncontrolledProviderScope(
          container: container,
          child: MaterialApp(
            home: const ScanPreviewPage(),
          ),
        ),
      );
      await tester.pumpAndSettle();

      // Critical value should be masked, not shown in plaintext
      expect(find.text('110101199001011234'), findsNothing);
      // Should show masked version (SensitiveValueWidget uses ••••••••)
      expect(find.textContaining('••••'), findsOneWidget);
    });

    testWidgets('shows import button with selected count', (tester) async {
      final container = ProviderContainer();
      addTearDown(container.dispose);

      final notifier = container.read(localSearchProvider.notifier);
      final scanResult = ScanResult(
        meta: ScanMeta(
          scanId: 'test-scan',
          createdAt: 0,
          sourceFile: '/test/resume.pdf',
          confidence: 0.9,
        ),
        sections: [
          ScanSection(
            section: 'identity',
            display: 'Personal Information',
            fields: [
              ScanField(
                key: 'fullName',
                value: 'Zhang San',
                sensitivity: SensitivityLevel.public,
                confidence: 0.95,
              ),
            ],
          ),
        ],
      );

      notifier.state = LocalSearchState(
        scanResults: [scanResult],
        importCandidates: [
          ImportCandidate(
            source: scanResult.sections.first,
            fields: [
              ImportFieldCandidate(
                source: scanResult.sections.first.fields.first,
                targetPropertyId: 'fullName',
                suggestedAction: ImportAction.createNew,
              ),
            ],
          ),
        ],
      );

      await tester.pumpWidget(
        UncontrolledProviderScope(
          container: container,
          child: MaterialApp(
            home: const ScanPreviewPage(),
          ),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.text('Import (1)'), findsOneWidget);
    });
  });
}
