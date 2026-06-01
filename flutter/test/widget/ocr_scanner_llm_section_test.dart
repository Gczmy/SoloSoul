import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/widgets/ocr_scanner_llm_section.dart';
import 'package:solosoul_flutter/presentation/widgets/ocr_scanner_llm_option.dart';

Widget wrap(Widget child) {
  return MaterialApp(
    localizationsDelegates: AppLocalizations.localizationsDelegates,
    supportedLocales: AppLocalizations.supportedLocales,
    home: Scaffold(body: child),
  );
}

void main() {
  group('OcrScannerLlmSection', () {
    testWidgets('renders unchecked checkbox when useLlmAssist is false', (tester) async {
      await tester.pumpWidget(wrap(OcrScannerLlmSection(
        useLlmAssist: false,
        modelOptions: const [],
        isCheckingModels: false,
        selectedModelId: null,
        onLlmAssistChanged: (_) {},
        onModelChanged: (_) {},
      )));

      expect(find.byType(CheckboxListTile), findsOneWidget);
      expect(find.text('Use LLM to assist extraction'), findsOneWidget);
    });

    testWidgets('shows loading indicator when checking models', (tester) async {
      await tester.pumpWidget(wrap(OcrScannerLlmSection(
        useLlmAssist: true,
        modelOptions: const [],
        isCheckingModels: true,
        selectedModelId: null,
        onLlmAssistChanged: (_) {},
        onModelChanged: (_) {},
      )));

      expect(find.byType(CircularProgressIndicator), findsOneWidget);
    });

    testWidgets('shows no-model state when no models available', (tester) async {
      await tester.pumpWidget(wrap(OcrScannerLlmSection(
        useLlmAssist: true,
        modelOptions: const [],
        isCheckingModels: false,
        selectedModelId: null,
        onLlmAssistChanged: (_) {},
        onModelChanged: (_) {},
      )));

      expect(find.byIcon(Icons.error_outline), findsOneWidget);
      expect(find.byType(FilledButton), findsOneWidget);
    });

    testWidgets('shows model selector when models available', (tester) async {
      await tester.pumpWidget(wrap(OcrScannerLlmSection(
        useLlmAssist: true,
        modelOptions: const [
          OcrScannerLlmOption(id: 'm1', displayName: 'Model One', isLocal: true, isAvailable: true),
          OcrScannerLlmOption(id: 'm2', displayName: 'Model Two', isLocal: false, isAvailable: false),
        ],
        isCheckingModels: false,
        selectedModelId: 'm1',
        onLlmAssistChanged: (_) {},
        onModelChanged: (_) {},
      )));

      expect(find.byType(DropdownButtonFormField<String>), findsOneWidget);
    });

    testWidgets('toggles LLM assist checkbox', (tester) async {
      bool? toggledValue;
      await tester.pumpWidget(wrap(OcrScannerLlmSection(
        useLlmAssist: false,
        modelOptions: const [],
        isCheckingModels: false,
        selectedModelId: null,
        onLlmAssistChanged: (v) => toggledValue = v,
        onModelChanged: (_) {},
      )));

      await tester.tap(find.byType(CheckboxListTile));
      await tester.pump();
      expect(toggledValue, isTrue);
    });
  });
}
