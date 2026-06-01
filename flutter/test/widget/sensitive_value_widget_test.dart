import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/providers/account_style_provider.dart';
import 'package:solosoul_flutter/presentation/providers/auth/auth_state.dart';
import 'package:solosoul_flutter/presentation/providers/sensitivity_provider.dart';
import 'package:solosoul_flutter/presentation/widgets/sensitive_value_widget.dart';

Widget wrapWithProviders(
  Widget child, {
  List overrides = const [],
}) {
  return ProviderScope(
    overrides: overrides.cast(),
    child: MaterialApp(
      localizationsDelegates: AppLocalizations.localizationsDelegates,
      supportedLocales: AppLocalizations.supportedLocales,
      home: Scaffold(body: child),
    ),
  );
}

void main() {
  group('SensitiveFieldTile', () {
    testWidgets('renders label and value', (tester) async {
      await tester.pumpWidget(wrapWithProviders(
        const SensitiveFieldTile(
          label: 'Name',
          fieldId: 'name',
          value: 'Alice',
        ),
      ));

      expect(find.text('Name'), findsOneWidget);
      expect(find.text('Alice'), findsOneWidget);
    });

    testWidgets('renders empty state', (tester) async {
      await tester.pumpWidget(wrapWithProviders(
        const SensitiveFieldTile(
          label: 'Email',
          fieldId: 'email',
          value: '',
          isEmpty: true,
        ),
      ));

      expect(find.text('Email'), findsOneWidget);
      expect(find.text('Tap to add'), findsOneWidget);
    });
  });

  group('SensitiveValueWidget public field', () {
    testWidgets('shows plaintext without mask', (tester) async {
      await tester.pumpWidget(wrapWithProviders(
        const SensitiveValueWidget(
          fieldId: 'name',
          value: 'Alice',
          sensitivityLevel: SensitivityLevel.public,
        ),
      ));

      expect(find.text('Alice'), findsOneWidget);
      expect(find.byIcon(Icons.visibility_off), findsNothing);
    });

    testWidgets('renders custom child when provided', (tester) async {
      await tester.pumpWidget(wrapWithProviders(
        SensitiveValueWidget(
          fieldId: 'name',
          value: 'Alice',
          sensitivityLevel: SensitivityLevel.public,
          child: const Chip(label: Text('Custom')),
        ),
      ));

      expect(find.text('Custom'), findsOneWidget);
    });
  });

  group('SensitiveValueWidget sensitive field', () {
    testWidgets('shows masked value initially', (tester) async {
      await tester.pumpWidget(wrapWithProviders(
        const SensitiveValueWidget(
          fieldId: 'password',
          value: 'secret123',
          sensitivityLevel: SensitivityLevel.sensitive,
        ),
      ));

      // Should show masked text, not the actual value
      expect(find.text('secret123'), findsNothing);
      expect(find.byIcon(Icons.visibility_off), findsOneWidget);
    });

    testWidgets('tap reveals value', (tester) async {
      await tester.pumpWidget(wrapWithProviders(
        const SensitiveValueWidget(
          fieldId: 'password',
          value: 'secret123',
          sensitivityLevel: SensitivityLevel.sensitive,
        ),
      ));

      // Tap the eye icon
      await tester.tap(find.byIcon(Icons.visibility_off));
      await tester.pumpAndSettle();

      expect(find.text('secret123'), findsOneWidget);
      expect(find.byIcon(Icons.visibility), findsOneWidget);
    });

    testWidgets('tap again hides value', (tester) async {
      await tester.pumpWidget(wrapWithProviders(
        const SensitiveValueWidget(
          fieldId: 'password',
          value: 'secret123',
          sensitivityLevel: SensitivityLevel.sensitive,
        ),
      ));

      await tester.tap(find.byIcon(Icons.visibility_off));
      await tester.pumpAndSettle();
      expect(find.text('secret123'), findsOneWidget);

      await tester.tap(find.byIcon(Icons.visibility));
      await tester.pumpAndSettle();
      expect(find.text('secret123'), findsNothing);
    });
  });

  group('SensitiveValueWidget masking logic', () {
    testWidgets('fully masks short values', (tester) async {
      await tester.pumpWidget(wrapWithProviders(
        const SensitiveValueWidget(
          fieldId: 'pin',
          value: '1234',
          sensitivityLevel: SensitivityLevel.sensitive,
        ),
      ));

      // Short values (≤12 chars) are fully masked with ••••••••
      expect(find.textContaining('•'), findsOneWidget);
      expect(find.text('1234'), findsNothing);
    });

    testWidgets('partially masks long values', (tester) async {
      await tester.pumpWidget(wrapWithProviders(
        const SensitiveValueWidget(
          fieldId: 'card',
          value: '6222123456781234',
          sensitivityLevel: SensitivityLevel.sensitive,
        ),
      ));

      // Long values show first 4 and last 4 with •••••••• in middle
      expect(find.text('6222123456781234'), findsNothing);
    });
  });

  group('SensitiveValueWidget critical field', () {
    testWidgets('shows masked when requireVerification is false', (tester) async {
      await tester.pumpWidget(wrapWithProviders(
        const SensitiveValueWidget(
          fieldId: 'ssn',
          value: '123-45-6789',
          sensitivityLevel: SensitivityLevel.critical,
          requireVerification: false,
        ),
      ));

      expect(find.text('123-45-6789'), findsNothing);
      expect(find.byIcon(Icons.visibility_off), findsOneWidget);
    });

    testWidgets('tap reveals critical field without verification', (tester) async {
      await tester.pumpWidget(wrapWithProviders(
        const SensitiveValueWidget(
          fieldId: 'ssn',
          value: '123-45-6789',
          sensitivityLevel: SensitivityLevel.critical,
          requireVerification: false,
        ),
      ));

      await tester.tap(find.byIcon(Icons.visibility_off));
      await tester.pumpAndSettle();

      expect(find.text('123-45-6789'), findsOneWidget);
    });
  });

  group('SensitiveValueWidget with provider lookup', () {
    testWidgets('uses effectiveSensitivityProvider when no explicit level', (tester) async {
      await tester.pumpWidget(wrapWithProviders(
        const SensitiveValueWidget(
          fieldId: 'email',
          value: 'test@example.com',
        ),
        overrides: [
          effectiveSensitivityProvider('email').overrideWith((ref) => SensitivityLevel.public),
        ],
      ));

      expect(find.text('test@example.com'), findsOneWidget);
    });

    testWidgets('masks when provider returns sensitive', (tester) async {
      await tester.pumpWidget(wrapWithProviders(
        const SensitiveValueWidget(
          fieldId: 'password',
          value: 'secret',
        ),
        overrides: [
          effectiveSensitivityProvider('password').overrideWith((ref) => SensitivityLevel.sensitive),
        ],
      ));

      expect(find.text('secret'), findsNothing);
    });
  });

  group('SensitiveValueWidget copy button', () {
    testWidgets('shows copy button when revealed', (tester) async {
      await tester.pumpWidget(wrapWithProviders(
        const SensitiveValueWidget(
          fieldId: 'password',
          value: 'secret123',
          sensitivityLevel: SensitivityLevel.sensitive,
        ),
      ));

      await tester.tap(find.byIcon(Icons.visibility_off));
      await tester.pumpAndSettle();

      expect(find.byIcon(Icons.copy), findsOneWidget);
    });

    testWidgets('hides copy button when masked', (tester) async {
      await tester.pumpWidget(wrapWithProviders(
        const SensitiveValueWidget(
          fieldId: 'password',
          value: 'secret123',
          sensitivityLevel: SensitivityLevel.sensitive,
        ),
      ));

      expect(find.byIcon(Icons.copy), findsNothing);
    });
  });
}
