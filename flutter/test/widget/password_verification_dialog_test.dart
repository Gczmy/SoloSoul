import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/presentation/widgets/password_verification_dialog.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  group('PasswordVerificationDialogContent', () {
    testWidgets('renders dialog with correct title and message', (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: PasswordVerificationDialogContent(
              message: 'Test message here',
              onVerify: (_) async => true,
            ),
          ),
        ),
      );

      expect(find.text('Verify Identity'), findsOneWidget);
      expect(find.text('Test message here'), findsOneWidget);
    });

    testWidgets('has password text field', (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: PasswordVerificationDialogContent(
              message: 'Enter password',
              onVerify: (_) async => true,
            ),
          ),
        ),
      );

      expect(find.byType(TextField), findsOneWidget);
      expect(find.text('Master Password'), findsOneWidget);
    });

    testWidgets('verify button is disabled when password is empty', (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: PasswordVerificationDialogContent(
              message: 'Enter password',
              onVerify: (_) async => true,
            ),
          ),
        ),
      );

      final button = tester.widget<ElevatedButton>(find.byType(ElevatedButton));
      expect(button.onPressed, isNull);
    });

    testWidgets('verify button is enabled when password is entered', (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: PasswordVerificationDialogContent(
              message: 'Enter password',
              onVerify: (_) async => true,
            ),
          ),
        ),
      );

      await tester.enterText(find.byType(TextField), 'mypassword');
      await tester.pump();

      final button = tester.widget<ElevatedButton>(find.byType(ElevatedButton));
      expect(button.onPressed, isNotNull);
    });

    testWidgets('successful verification pops with password', (tester) async {
      String? poppedValue;

      await tester.pumpWidget(
        MaterialApp(
          home: Builder(
            builder: (context) {
              return Scaffold(
                body: ElevatedButton(
                  onPressed: () async {
                    poppedValue = await showDialog<String>(
                      context: context,
                      builder: (_) => const AlertDialog(
                        content: PasswordVerificationDialogContent(
                          message: 'Test',
                          onVerify: _alwaysTrue,
                        ),
                      ),
                    );
                  },
                  child: const Text('Show'),
                ),
              );
            },
          ),
        ),
      );

      await tester.tap(find.text('Show'));
      await tester.pumpAndSettle();

      await tester.enterText(find.byType(TextField), 'secret123');
      await tester.pump();

      await tester.tap(find.text('Verify'));
      await tester.pumpAndSettle();

      expect(poppedValue, 'secret123');
    });

    testWidgets('failed verification shows error', (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: PasswordVerificationDialogContent(
              message: 'Test',
              onVerify: (_) async => false,
            ),
          ),
        ),
      );

      await tester.enterText(find.byType(TextField), 'wrong');
      await tester.pump();

      await tester.tap(find.text('Verify'));
      await tester.pumpAndSettle();

      expect(find.text('Invalid password'), findsOneWidget);
    });

    testWidgets('cancel button pops with null', (tester) async {
      String? poppedValue = 'initial';

      await tester.pumpWidget(
        MaterialApp(
          home: Builder(
            builder: (context) {
              return Scaffold(
                body: ElevatedButton(
                  onPressed: () async {
                    poppedValue = await showDialog<String>(
                      context: context,
                      builder: (_) => const AlertDialog(
                        content: PasswordVerificationDialogContent(
                          message: 'Test',
                          onVerify: _alwaysTrue,
                        ),
                      ),
                    );
                  },
                  child: const Text('Show'),
                ),
              );
            },
          ),
        ),
      );

      await tester.tap(find.text('Show'));
      await tester.pumpAndSettle();

      await tester.tap(find.text('Cancel'));
      await tester.pumpAndSettle();

      expect(poppedValue, isNull);
    });

    testWidgets('password visibility toggle works', (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: PasswordVerificationDialogContent(
              message: 'Test',
              onVerify: (_) async => true,
            ),
          ),
        ),
      );

      final textField = tester.widget<TextField>(find.byType(TextField));
      expect(textField.obscureText, isTrue);

      // Tap visibility toggle
      await tester.tap(find.byIcon(Icons.visibility_outlined));
      await tester.pump();

      final textField2 = tester.widget<TextField>(find.byType(TextField));
      expect(textField2.obscureText, isFalse);

      // Tap again to hide
      await tester.tap(find.byIcon(Icons.visibility_off_outlined));
      await tester.pump();

      final textField3 = tester.widget<TextField>(find.byType(TextField));
      expect(textField3.obscureText, isTrue);
    });

    testWidgets('shows password hint button', (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: PasswordVerificationDialogContent(
              message: 'Test',
              passwordHint: 'My hint',
              onVerify: (_) async => true,
            ),
          ),
        ),
      );

      expect(find.byIcon(Icons.help_outline), findsOneWidget);
    });

    testWidgets('verifying shows loading indicator', (tester) async {
      final completer = Completer<bool>();

      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: PasswordVerificationDialogContent(
              message: 'Test',
              onVerify: (_) => completer.future,
            ),
          ),
        ),
      );

      await tester.enterText(find.byType(TextField), 'password');
      await tester.pump();

      await tester.tap(find.text('Verify'));
      await tester.pump();

      expect(find.byType(CircularProgressIndicator), findsOneWidget);
      expect(find.text('Verify'), findsNothing);

      completer.complete(true);
      await tester.pumpAndSettle();
    });
  });
}

Future<bool> _alwaysTrue(String _) async => true;
