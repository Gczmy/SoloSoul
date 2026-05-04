import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/presentation/widgets/sidebar/add_page_input.dart';

void main() {
  group('AddPageInput', () {
    testWidgets('renders icon, text field and confirm button', (tester) async {
      final controller = TextEditingController();
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: AddPageInput(
              controller: controller,
              iconName: 'folder',
              onIconTap: () {},
              onConfirm: () {},
            ),
          ),
        ),
      );

      expect(find.byType(TextField), findsOneWidget);
      expect(find.byIcon(Icons.check), findsOneWidget);
      expect(find.byIcon(Icons.folder_outlined), findsOneWidget); // Icon
    });

    testWidgets('calls onIconTap when icon tapped', (tester) async {
      var called = false;
      final controller = TextEditingController();
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: AddPageInput(
              controller: controller,
              iconName: 'folder',
              onIconTap: () => called = true,
              onConfirm: () {},
            ),
          ),
        ),
      );

      await tester.tap(find.byIcon(Icons.folder_outlined));
      expect(called, isTrue);
    });

    testWidgets('calls onConfirm when check button tapped', (tester) async {
      var called = false;
      final controller = TextEditingController();
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: AddPageInput(
              controller: controller,
              iconName: 'folder',
              onIconTap: () {},
              onConfirm: () => called = true,
            ),
          ),
        ),
      );

      await tester.tap(find.byIcon(Icons.check));
      expect(called, isTrue);
    });

    testWidgets('calls onConfirm on text field submit', (tester) async {
      var called = false;
      final controller = TextEditingController();
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: AddPageInput(
              controller: controller,
              iconName: 'folder',
              onIconTap: () {},
              onConfirm: () => called = true,
            ),
          ),
        ),
      );

      await tester.showKeyboard(find.byType(TextField));
      await tester.testTextInput.receiveAction(TextInputAction.done);
      await tester.pump();
      expect(called, isTrue);
    });

    testWidgets('has autofocus enabled', (tester) async {
      final controller = TextEditingController();
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: AddPageInput(
              controller: controller,
              iconName: 'folder',
              onIconTap: () {},
              onConfirm: () {},
            ),
          ),
        ),
      );

      final textField = tester.widget<TextField>(find.byType(TextField));
      expect(textField.autofocus, isTrue);
    });
  });
}
