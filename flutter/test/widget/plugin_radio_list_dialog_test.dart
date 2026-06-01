import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/presentation/widgets/plugin_radio_list_dialog.dart';

void main() {
  group('PluginRadioListDialog', () {
    testWidgets('renders title and description', (tester) async {
      await tester.pumpWidget(MaterialApp(
        home: Scaffold(
          body: Builder(
            builder: (context) => ElevatedButton(
              onPressed: () => showDialog(
                context: context,
                builder: (_) => PluginRadioListDialog(
                  title: '选择一项',
                  description: '请从以下列表中选择',
                  items: [
                    PluginRadioItem(id: 'a', label: '选项A'),
                    PluginRadioItem(id: 'b', label: '选项B'),
                  ],
                ),
              ),
              child: const Text('show'),
            ),
          ),
        ),
      ));

      await tester.tap(find.text('show'));
      await tester.pumpAndSettle();

      expect(find.text('选择一项'), findsOneWidget);
      expect(find.text('请从以下列表中选择'), findsOneWidget);
      expect(find.text('选项A'), findsOneWidget);
      expect(find.text('选项B'), findsOneWidget);
    });

    testWidgets('renders without description when null', (tester) async {
      await tester.pumpWidget(MaterialApp(
        home: Scaffold(
          body: Builder(
            builder: (context) => ElevatedButton(
              onPressed: () => showDialog(
                context: context,
                builder: (_) => PluginRadioListDialog(
                  title: '选择',
                  items: [
                    PluginRadioItem(id: 'x', label: 'X'),
                  ],
                ),
              ),
              child: const Text('show'),
            ),
          ),
        ),
      ));

      await tester.tap(find.text('show'));
      await tester.pumpAndSettle();

      expect(find.text('选择'), findsOneWidget);
      expect(find.text('X'), findsOneWidget);
      // description absent
      expect(find.textContaining('请从以下'), findsNothing);
    });

    testWidgets('selects item and confirms returns id', (tester) async {
      String? result;

      await tester.pumpWidget(MaterialApp(
        home: Scaffold(
          body: Builder(
            builder: (context) => ElevatedButton(
              onPressed: () async {
                result = await showDialog<String>(
                  context: context,
                  builder: (_) => PluginRadioListDialog(
                    title: '选择',
                    items: [
                      PluginRadioItem(id: 'id1', label: 'One'),
                      PluginRadioItem(id: 'id2', label: 'Two'),
                    ],
                  ),
                );
              },
              child: const Text('show'),
            ),
          ),
        ),
      ));

      await tester.tap(find.text('show'));
      await tester.pumpAndSettle();

      // confirm button disabled before selection
      final confirmBtn = find.widgetWithText(FilledButton, '确认');
      expect(tester.widget<FilledButton>(confirmBtn).onPressed, isNull);

      // select id2
      await tester.tap(find.text('Two'));
      await tester.pumpAndSettle();

      // confirm button now enabled
      expect(tester.widget<FilledButton>(confirmBtn).onPressed, isNotNull);

      await tester.tap(confirmBtn);
      await tester.pumpAndSettle();

      expect(result, 'id2');
    });

    testWidgets('cancel returns null', (tester) async {
      String? result;

      await tester.pumpWidget(MaterialApp(
        home: Scaffold(
          body: Builder(
            builder: (context) => ElevatedButton(
              onPressed: () async {
                result = await showDialog<String>(
                  context: context,
                  builder: (_) => PluginRadioListDialog(
                    title: '选择',
                    items: [PluginRadioItem(id: 'x', label: 'X')],
                  ),
                );
              },
              child: const Text('show'),
            ),
          ),
        ),
      ));

      await tester.tap(find.text('show'));
      await tester.pumpAndSettle();

      await tester.tap(find.widgetWithText(TextButton, '取消'));
      await tester.pumpAndSettle();

      expect(result, isNull);
    });

    testWidgets('confirm disabled when no selection', (tester) async {
      await tester.pumpWidget(MaterialApp(
        home: Scaffold(
          body: Builder(
            builder: (context) => ElevatedButton(
              onPressed: () => showDialog(
                context: context,
                builder: (_) => PluginRadioListDialog(
                  title: '选择',
                  items: [PluginRadioItem(id: 'x', label: 'X')],
                ),
              ),
              child: const Text('show'),
            ),
          ),
        ),
      ));

      await tester.tap(find.text('show'));
      await tester.pumpAndSettle();

      expect(
        tester.widget<FilledButton>(find.widgetWithText(FilledButton, '确认')).onPressed,
        isNull,
      );
    });
  });
}
