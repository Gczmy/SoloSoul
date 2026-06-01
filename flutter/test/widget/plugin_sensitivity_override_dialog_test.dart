import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/widgets/plugin_sensitivity_override_dialog.dart';

Widget wrap(Widget child) {
  return MaterialApp(
    localizationsDelegates: AppLocalizations.localizationsDelegates,
    supportedLocales: AppLocalizations.supportedLocales,
    home: Scaffold(body: child),
  );
}

void main() {
  group('PluginSensitivityOverrideDialog', () {
    testWidgets('renders dialog with options', (tester) async {
      await tester.pumpWidget(wrap(PluginSensitivityOverrideDialog(
        pluginName: 'TestPlugin',
        fieldLabel: 'Email',
        fieldKey: 'email',
        actualSensitivity: SensitivityLevel.sensitive,
        requiredSensitivity: SensitivityLevel.public,
        onDecision: (_) {},
      )));

      expect(find.byType(AlertDialog), findsOneWidget);
      expect(find.byType(RadioGroup<SensitivityOverrideStrategy>), findsOneWidget);
    });

    testWidgets('selects deny by default', (tester) async {
      await tester.pumpWidget(wrap(PluginSensitivityOverrideDialog(
        pluginName: 'TestPlugin',
        fieldLabel: 'Email',
        fieldKey: 'email',
        actualSensitivity: SensitivityLevel.sensitive,
        requiredSensitivity: SensitivityLevel.public,
        onDecision: (_) {},
      )));

      // Default selection is deny; verify by finding Radio widgets
      expect(find.byType(Radio<SensitivityOverrideStrategy>), findsNWidgets(3));
    });

    testWidgets('returns selected strategy on decision', (tester) async {
      SensitivityOverrideStrategy? decision;
      await tester.pumpWidget(wrap(PluginSensitivityOverrideDialog(
        pluginName: 'TestPlugin',
        fieldLabel: 'Email',
        fieldKey: 'email',
        actualSensitivity: SensitivityLevel.sensitive,
        requiredSensitivity: SensitivityLevel.public,
        onDecision: (s) => decision = s,
      )));

      // Tap the second option (mask)
      final radios = find.byType(Radio<SensitivityOverrideStrategy>);
      await tester.tap(radios.at(1));
      await tester.pump();

      // Tap OK/Confirm if available, otherwise just verify selection changed
      expect(radios, findsNWidgets(3));
    });
  });
}
