import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/services/debug_logger.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/widgets/settings/debug_log_sheet.dart';

Widget wrap(Widget child) {
  return MaterialApp(
    localizationsDelegates: AppLocalizations.localizationsDelegates,
    supportedLocales: AppLocalizations.supportedLocales,
    home: Scaffold(body: child),
  );
}

void main() {
  group('DebugLogSheet', () {
    setUp(() {
      DebugLogger.instance.clearBuffer();
      DebugLogger.instance.log('TEST', 'hello world', LogLevel.info);
      DebugLogger.instance.log('TEST', 'error msg', LogLevel.error);
    });

    tearDown(() {
      DebugLogger.instance.clearBuffer();
    });

    testWidgets('renders log entries as SelectableText', (tester) async {
      await tester.pumpWidget(wrap(DebugLogSheet(
        scrollController: ScrollController(),
        onDisableDebugMode: () async {},
      )));

      // Non-empty logs render SelectableText.rich (not plain Text)
      expect(find.byType(SelectableText), findsOneWidget);
    });

    testWidgets('shows empty state when no logs', (tester) async {
      DebugLogger.instance.clearBuffer();
      await tester.pumpWidget(wrap(DebugLogSheet(
        scrollController: ScrollController(),
        onDisableDebugMode: () async {},
      )));

      expect(find.byType(SelectableText), findsNothing);
    });

    testWidgets('refresh button reloads logs', (tester) async {
      await tester.pumpWidget(wrap(DebugLogSheet(
        scrollController: ScrollController(),
        onDisableDebugMode: () async {},
      )));

      expect(find.byIcon(Icons.refresh), findsOneWidget);
      await tester.tap(find.byIcon(Icons.refresh));
      await tester.pump();
      expect(find.byType(SelectableText), findsOneWidget);
    });

    testWidgets('copy button shows confirmation dialog', (tester) async {
      await tester.pumpWidget(wrap(DebugLogSheet(
        scrollController: ScrollController(),
        onDisableDebugMode: () async {},
      )));

      await tester.tap(find.byIcon(Icons.copy));
      await tester.pumpAndSettle();

      expect(find.byType(AlertDialog), findsOneWidget);
      expect(find.text('Copy'), findsOneWidget);
    });

    testWidgets('disable button calls onDisableDebugMode', (tester) async {
      bool called = false;
      await tester.pumpWidget(wrap(DebugLogSheet(
        scrollController: ScrollController(),
        onDisableDebugMode: () async => called = true,
      )));

      await tester.tap(find.byIcon(Icons.power_settings_new));
      await tester.pump();
      expect(called, isTrue);
    });
  });
}
