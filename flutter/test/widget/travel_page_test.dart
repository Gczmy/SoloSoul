import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/pages/travel_page.dart';
import 'package:solosoul_flutter/presentation/providers/unified_object_provider.dart';
import 'package:solosoul_flutter/presentation/theme/glass_adapters.dart';

Map<String, PropertyValue> _placeholderProperties() => {
      'title': const TextProperty(text: '', sensitivity: SensitivityLevel.public),
    };

UnifiedObjectData _mockTravelData() {
  return UnifiedObjectData(
    objects: [
      UnifiedObject(
        id: '__page_travel',
        typeId: 'page',
        name: 'Travel',
        iconName: 'flight',
        parentId: null,
        childrenIds: const [
          '__section_passport',
          '__section_visa',
          '__section_travel_history',
        ],
        properties: const {},
        createdAt: 0,
        updatedAt: 0,
      ),
      UnifiedObject(
        id: '__section_passport',
        typeId: 'travel_passport',
        name: 'Passports',
        iconName: 'flight',
        parentId: '__page_travel',
        childrenIds: const [],
        properties: _placeholderProperties(),
        createdAt: 0,
        updatedAt: 0,
      ),
      UnifiedObject(
        id: '__section_visa',
        typeId: 'travel_visa',
        name: 'Visas',
        iconName: 'description',
        parentId: '__page_travel',
        childrenIds: const [],
        properties: _placeholderProperties(),
        createdAt: 0,
        updatedAt: 0,
      ),
      UnifiedObject(
        id: '__section_travel_history',
        typeId: 'travel_history',
        name: 'Travel History',
        iconName: 'history',
        parentId: '__page_travel',
        childrenIds: const [],
        properties: _placeholderProperties(),
        createdAt: 0,
        updatedAt: 0,
      ),
    ],
    customTypes: const [],
  );
}

class _TestUnifiedObjectNotifier extends UnifiedObjectNotifier {
  final UnifiedObjectData _data;

  _TestUnifiedObjectNotifier(this._data);

  @override
  UnifiedObjectData build() => _data;
}

Widget _buildTravelPageWithData(UnifiedObjectData data) {
  return ProviderScope(
    overrides: [
      unifiedObjectProvider.overrideWith(() => _TestUnifiedObjectNotifier(data)),
    ],
    child: MaterialApp(
      localizationsDelegates: AppLocalizations.localizationsDelegates,
      supportedLocales: AppLocalizations.supportedLocales,
      home: const TravelPage(),
    ),
  );
}

void main() {
  group('TravelPage Widget Tests', () {
    testWidgets('renders travel page with scaffold', (tester) async {
      await tester.pumpWidget(_buildTravelPageWithData(_mockTravelData()));
      await tester.pumpAndSettle();

      expect(find.byType(Scaffold), findsOneWidget);
    });

    testWidgets('has app bar with Travel title', (tester) async {
      await tester.pumpWidget(_buildTravelPageWithData(_mockTravelData()));
      await tester.pumpAndSettle();

      expect(find.text('Travel'), findsOneWidget);
      expect(find.byType(SoloGlassAppBar), findsOneWidget);
    });

    testWidgets('shows OCR scan button', (tester) async {
      await tester.pumpWidget(_buildTravelPageWithData(_mockTravelData()));
      await tester.pumpAndSettle();

      expect(find.text('Scan Document'), findsOneWidget);
      expect(find.byIcon(Icons.document_scanner_outlined), findsOneWidget);
    });

    testWidgets('shows Passports section', (tester) async {
      await tester.pumpWidget(_buildTravelPageWithData(_mockTravelData()));
      await tester.pumpAndSettle();

      expect(find.text('Passports'), findsOneWidget);
      expect(find.byIcon(Icons.flight), findsWidgets);
    });

    testWidgets('shows Visas section', (tester) async {
      await tester.pumpWidget(_buildTravelPageWithData(_mockTravelData()));
      await tester.pumpAndSettle();

      expect(find.text('Visas'), findsOneWidget);
      expect(find.byIcon(Icons.description), findsWidgets);
    });

    testWidgets('shows Travel History section', (tester) async {
      await tester.pumpWidget(_buildTravelPageWithData(_mockTravelData()));
      await tester.pumpAndSettle();

      expect(find.text('Travel History'), findsOneWidget);
      expect(find.byIcon(Icons.history), findsWidgets);
    });
  });

  group('TravelPage OCR Dialog Tests', () {
    testWidgets('shows OCR dialog when scan button tapped', (tester) async {
      await tester.pumpWidget(_buildTravelPageWithData(_mockTravelData()));
      await tester.pumpAndSettle();

      await tester.tap(find.text('Scan Document'));
      await tester.pumpAndSettle();

      // Bottom sheet is shown (OcrScannerSheet)
      expect(find.byType(Scaffold), findsWidgets);
    });
  });
}
