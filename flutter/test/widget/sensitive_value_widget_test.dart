import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/presentation/providers/account_style_provider.dart';
import 'package:solosoul_flutter/presentation/providers/auth/auth_notifier.dart';
import 'package:solosoul_flutter/presentation/providers/auth/auth_state.dart';
import 'package:solosoul_flutter/presentation/providers/auth/auth_types.dart';
import 'package:solosoul_flutter/presentation/providers/sensitivity_provider.dart';
import 'package:solosoul_flutter/presentation/widgets/sensitive_value_widget.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  Widget buildWidget({
    required String fieldId,
    required String value,
    required SensitivityLevel sensitivity,
    SensitivityDisplayMode displayMode = SensitivityDisplayMode.showAll,
    bool accessGranted = false,
    Widget? child,
  }) {
    return ProviderScope(
      overrides: [
        effectiveSensitivityProvider(fieldId).overrideWithValue(sensitivity),
        accountStyleProvider.overrideWith(() => _TestAccountStyleNotifier(displayMode)),
        isSensitiveAccessGrantedProvider.overrideWithValue(accessGranted),
        authNotifierProvider.overrideWith(() => _TestAuthNotifier()),
      ],
      child: MaterialApp(
        home: Scaffold(
          body: SensitiveValueWidget(
            fieldId: fieldId,
            value: value,
            child: child,
          ),
        ),
      ),
    );
  }

  group('SensitiveValueWidget public field', () {
    testWidgets('shows plaintext without eye icon', (tester) async {
      await tester.pumpWidget(buildWidget(
        fieldId: 'public_field',
        value: 'Hello World',
        sensitivity: SensitivityLevel.public,
      ));
      await tester.pumpAndSettle();

      expect(find.text('Hello World'), findsOneWidget);
      expect(find.byIcon(Icons.visibility), findsNothing);
      expect(find.byIcon(Icons.visibility_off), findsNothing);
    });
  });

  group('SensitiveValueWidget internal field', () {
    testWidgets('shows plaintext when privacy shield is OFF', (tester) async {
      await tester.pumpWidget(buildWidget(
        fieldId: 'internal_field',
        value: 'Internal Data',
        sensitivity: SensitivityLevel.internal,
        displayMode: SensitivityDisplayMode.showAll,
      ));
      await tester.pumpAndSettle();

      expect(find.text('Internal Data'), findsOneWidget);
      expect(find.byIcon(Icons.visibility_off), findsNothing);
    });

    testWidgets('shows masked when privacy shield is ON', (tester) async {
      await tester.pumpWidget(buildWidget(
        fieldId: 'internal_field',
        value: 'Internal Data',
        sensitivity: SensitivityLevel.internal,
        displayMode: SensitivityDisplayMode.hidePrivate,
      ));
      await tester.pumpAndSettle();

      expect(find.text('Internal Data'), findsNothing);
      expect(find.byIcon(Icons.visibility_off), findsOneWidget);
    });
  });

  group('SensitiveValueWidget sensitive field', () {
    testWidgets('always masked with eye icon', (tester) async {
      await tester.pumpWidget(buildWidget(
        fieldId: 'sensitive_field',
        value: 'Secret123',
        sensitivity: SensitivityLevel.sensitive,
        displayMode: SensitivityDisplayMode.showAll,
      ));
      await tester.pumpAndSettle();

      expect(find.text('Secret123'), findsNothing);
      expect(find.byIcon(Icons.visibility_off), findsOneWidget);
    });
  });

  group('SensitiveValueWidget critical field', () {
    testWidgets('masked when no recent verification', (tester) async {
      await tester.pumpWidget(buildWidget(
        fieldId: 'critical_field',
        value: 'CriticalData',
        sensitivity: SensitivityLevel.critical,
        accessGranted: false,
      ));
      await tester.pumpAndSettle();

      expect(find.text('CriticalData'), findsNothing);
      expect(find.byIcon(Icons.visibility_off), findsOneWidget);
    });

    testWidgets('shows plaintext when recently verified and privacy OFF', (tester) async {
      await tester.pumpWidget(buildWidget(
        fieldId: 'critical_field',
        value: 'CriticalData',
        sensitivity: SensitivityLevel.critical,
        accessGranted: true,
        displayMode: SensitivityDisplayMode.showAll,
      ));
      await tester.pumpAndSettle();

      // With verification and no privacy mode, critical is treated as public
      expect(find.text('CriticalData'), findsOneWidget);
      expect(find.byIcon(Icons.visibility), findsNothing);
      expect(find.byIcon(Icons.visibility_off), findsNothing);
    });

    testWidgets('still masked when privacy mode is ON even with verification', (tester) async {
      await tester.pumpWidget(buildWidget(
        fieldId: 'critical_field',
        value: 'CriticalData',
        sensitivity: SensitivityLevel.critical,
        accessGranted: true,
        displayMode: SensitivityDisplayMode.hidePrivate,
      ));
      await tester.pumpAndSettle();

      // In privacy mode, critical fields are always masked
      expect(find.text('CriticalData'), findsNothing);
      expect(find.byIcon(Icons.visibility_off), findsOneWidget);
    });
  });

  group('SensitiveValueWidget masking logic', () {
    testWidgets('short values are fully masked', (tester) async {
      await tester.pumpWidget(buildWidget(
        fieldId: 's_field',
        value: 'short',
        sensitivity: SensitivityLevel.sensitive,
      ));
      await tester.pumpAndSettle();

      expect(find.text('short'), findsNothing);
      // The masked text should be ••••••••
      expect(find.textContaining('•'), findsOneWidget);
    });

    testWidgets('long values are partially masked', (tester) async {
      await tester.pumpWidget(buildWidget(
        fieldId: 's_field',
        value: '12345678901234',
        sensitivity: SensitivityLevel.sensitive,
      ));
      await tester.pumpAndSettle();

      expect(find.text('12345678901234'), findsNothing);
      // Partial mask: first 4 + •••••••• + last 4
      expect(find.textContaining('1234••••••••1234'), findsOneWidget);
    });

    testWidgets('exactly 12 char values are fully masked', (tester) async {
      await tester.pumpWidget(buildWidget(
        fieldId: 's_field',
        value: '123456789012',
        sensitivity: SensitivityLevel.sensitive,
      ));
      await tester.pumpAndSettle();

      expect(find.text('123456789012'), findsNothing);
      expect(find.textContaining('•'), findsOneWidget);
    });
  });

  group('SensitiveValueWidget custom child', () {
    testWidgets('renders custom child when provided and not masked', (tester) async {
      await tester.pumpWidget(buildWidget(
        fieldId: 'public_field',
        value: 'Hello',
        sensitivity: SensitivityLevel.public,
        child: const Text('Custom Display'),
      ));
      await tester.pumpAndSettle();

      expect(find.text('Custom Display'), findsOneWidget);
      expect(find.text('Hello'), findsNothing);
    });
  });

  group('SensitiveFieldTile', () {
    testWidgets('renders label and value', (tester) async {
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            effectiveSensitivityProvider('tile_field').overrideWithValue(
              SensitivityLevel.public,
            ),
            accountStyleProvider.overrideWith(() => _TestAccountStyleNotifier(SensitivityDisplayMode.showAll)),
            isSensitiveAccessGrantedProvider.overrideWithValue(false),
            authNotifierProvider.overrideWith(() => _TestAuthNotifier()),
          ],
          child: const MaterialApp(
            home: Scaffold(
              body: SensitiveFieldTile(
                label: 'Test Label',
                fieldId: 'tile_field',
                value: 'Tile Value',
              ),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.text('Test Label'), findsOneWidget);
      expect(find.text('Tile Value'), findsOneWidget);
    });

    testWidgets('shows empty state', (tester) async {
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            effectiveSensitivityProvider('tile_field').overrideWithValue(
              SensitivityLevel.public,
            ),
            accountStyleProvider.overrideWith(() => _TestAccountStyleNotifier(SensitivityDisplayMode.showAll)),
            isSensitiveAccessGrantedProvider.overrideWithValue(false),
            authNotifierProvider.overrideWith(() => _TestAuthNotifier()),
          ],
          child: const MaterialApp(
            home: Scaffold(
              body: SensitiveFieldTile(
                label: 'Test Label',
                fieldId: 'tile_field',
                value: '',
                isEmpty: true,
              ),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.text('Tap to add'), findsOneWidget);
    });
  });
}

// Test helpers
class _TestAccountStyleNotifier extends AccountStyleNotifier {
  final SensitivityDisplayMode _displayMode;

  _TestAccountStyleNotifier(this._displayMode);

  @override
  Future<AccountStyle> build() async {
    return AccountStyle(displayMode: _displayMode);
  }
}

class _TestAuthNotifier extends AuthNotifier {
  @override
  Future<AuthState> build() async {
    return AuthState.initial;
  }
}
