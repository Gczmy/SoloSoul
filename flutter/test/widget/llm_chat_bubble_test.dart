import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/widgets/llm/llm_chat_bubble.dart';

Widget wrap(Widget child) {
  return MaterialApp(
    localizationsDelegates: AppLocalizations.localizationsDelegates,
    supportedLocales: AppLocalizations.supportedLocales,
    home: Scaffold(body: child),
  );
}

void main() {
  group('LlmChatBubble', () {
    testWidgets('renders user message aligned right', (tester) async {
      await tester.pumpWidget(wrap(const LlmChatBubble(
        message: 'Hello AI',
        isUser: true,
      )));

      expect(find.byType(SelectableText), findsOneWidget);
      final align = tester.widget<Align>(find.byType(Align));
      expect(align.alignment, Alignment.centerRight);
    });

    testWidgets('renders AI message aligned left with copy button', (tester) async {
      await tester.pumpWidget(wrap(const LlmChatBubble(
        message: 'Hello User',
        isUser: false,
      )));

      expect(find.byType(SelectableText), findsOneWidget);
      final align = tester.widget<Align>(find.byType(Align));
      expect(align.alignment, Alignment.centerLeft);
      expect(find.byIcon(Icons.copy), findsOneWidget);
    });

    testWidgets('shows typing dots when streaming', (tester) async {
      await tester.pumpWidget(wrap(const LlmChatBubble(
        message: '',
        isUser: false,
        isStreaming: true,
      )));

      expect(find.byType(AnimatedBuilder), findsWidgets);
    });

    testWidgets('hides copy button for empty AI message', (tester) async {
      await tester.pumpWidget(wrap(const LlmChatBubble(
        message: '',
        isUser: false,
      )));

      expect(find.byIcon(Icons.copy), findsNothing);
    });

    testWidgets('copy button is present for AI message', (tester) async {
      await tester.pumpWidget(wrap(const LlmChatBubble(
        message: 'copy me',
        isUser: false,
      )));

      expect(find.byIcon(Icons.copy), findsOneWidget);
    });
  });
}
