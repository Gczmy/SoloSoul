import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/providers/sensitivity_provider.dart';
import 'package:solosoul_flutter/presentation/widgets/responsive_label_field.dart';
import 'package:solosoul_flutter/presentation/widgets/sensitive_value_widget.dart';
import 'package:solosoul_flutter/presentation/widgets/sensitivity_tag.dart';

Widget wrapWithProviders(Widget child, {List overrides = const []}) {
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
  group('ResponsiveLabelField', () {
    testWidgets('returns SizedBox.shrink when fields is empty', (tester) async {
      await tester.pumpWidget(wrapWithProviders(
        const ResponsiveLabelField(fields: []),
      ));

      expect(find.byType(SizedBox), findsOneWidget);
      expect(find.text('Label'), findsNothing);
    });

    testWidgets('renders horizontal wrap by default', (tester) async {
      await tester.pumpWidget(wrapWithProviders(
        const ResponsiveLabelField(
          fields: [
            LabelValueField(label: 'Name', value: 'Alice'),
            LabelValueField(label: 'Age', value: '30'),
          ],
        ),
      ));

      expect(find.text('Name: '), findsOneWidget);
      expect(find.text('Age: '), findsOneWidget);
      expect(find.text('Alice'), findsOneWidget);
      expect(find.text('30'), findsOneWidget);
      expect(find.byType(Wrap), findsOneWidget);
    });

    testWidgets('renders vertical column when layoutAxis is vertical', (tester) async {
      await tester.pumpWidget(wrapWithProviders(
        const ResponsiveLabelField(
          fields: [
            LabelValueField(label: 'A', value: '1'),
            LabelValueField(label: 'B', value: '2'),
          ],
          layoutAxis: Axis.vertical,
        ),
      ));

      expect(find.byType(Column), findsOneWidget);
      expect(find.text('A: '), findsOneWidget);
      expect(find.text('B: '), findsOneWidget);
    });

    testWidgets('renders sensitive value widget for sensitive fields', (tester) async {
      await tester.pumpWidget(wrapWithProviders(
        const ResponsiveLabelField(
          fields: [
            LabelValueField(
              label: 'Password',
              value: 'secret',
              isSensitive: true,
            ),
          ],
        ),
      ));

      expect(find.byType(SensitiveValueWidget), findsOneWidget);
    });

    testWidgets('renders plain text for non-sensitive fields', (tester) async {
      await tester.pumpWidget(wrapWithProviders(
        const ResponsiveLabelField(
          fields: [
            LabelValueField(label: 'Name', value: 'Bob'),
          ],
        ),
      ));

      expect(find.byType(SensitiveValueWidget), findsNothing);
      expect(find.text('Bob'), findsOneWidget);
    });

    testWidgets('renders sensitivity tag for each field', (tester) async {
      await tester.pumpWidget(wrapWithProviders(
        const ResponsiveLabelField(
          fields: [
            LabelValueField(label: 'A', value: '1'),
            LabelValueField(label: 'B', value: '2'),
          ],
        ),
      ));

      expect(find.byType(SensitivityTag), findsNWidgets(2));
    });

    testWidgets('uses provided sensitivityLevel', (tester) async {
      await tester.pumpWidget(wrapWithProviders(
        const ResponsiveLabelField(
          fields: [
            LabelValueField(
              label: 'Secret',
              value: 'hidden',
              sensitivityLevel: SensitivityLevel.critical,
            ),
          ],
        ),
      ));

      expect(find.byType(SensitiveValueWidget), findsOneWidget);
    });

    testWidgets('looks up registry when sensitivityLevel not provided', (tester) async {
      await tester.pumpWidget(wrapWithProviders(
        const ResponsiveLabelField(
          fields: [
            LabelValueField(
              label: 'Name',
              value: 'Test',
              fieldId: 'name',
            ),
          ],
        ),
        overrides: [
          effectiveSensitivityProvider('name').overrideWith((ref) => SensitivityLevel.sensitive),
        ],
      ));

      // Because registry returns sensitive, it should render SensitiveValueWidget
      expect(find.byType(SensitiveValueWidget), findsOneWidget);
    });

    testWidgets('uses label-based fieldId fallback', (tester) async {
      await tester.pumpWidget(wrapWithProviders(
        const ResponsiveLabelField(
          fields: [
            LabelValueField(
              label: 'Email Address',
              value: 'test@example.com',
            ),
          ],
        ),
        overrides: [
          effectiveSensitivityProvider('email.address').overrideWith((ref) => SensitivityLevel.public),
        ],
      ));

      // Should render plain text since registry returns public
      expect(find.byType(SensitiveValueWidget), findsNothing);
      expect(find.text('test@example.com'), findsOneWidget);
    });
  });
}
