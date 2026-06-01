import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/widgets/sensitivity_tag.dart';

void main() {
  group('SensitivityTag', () {
    Future<void> pumpTag(WidgetTester tester, SensitivityLevel level) async {
      await tester.pumpWidget(
        MaterialApp(
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          home: Scaffold(
            body: SensitivityTag(level: level),
          ),
        ),
      );
      await tester.pumpAndSettle();
    }

    testWidgets('renders for public level', (tester) async {
      await pumpTag(tester, SensitivityLevel.public);
      expect(find.byType(Container), findsOneWidget);
      expect(find.byType(Text), findsOneWidget);
    });

    testWidgets('renders for internal level', (tester) async {
      await pumpTag(tester, SensitivityLevel.internal);
      expect(find.byType(Container), findsOneWidget);
    });

    testWidgets('renders for sensitive level', (tester) async {
      await pumpTag(tester, SensitivityLevel.sensitive);
      expect(find.byType(Container), findsOneWidget);
    });

    testWidgets('renders for critical level', (tester) async {
      await pumpTag(tester, SensitivityLevel.critical);
      expect(find.byType(Container), findsOneWidget);
    });

    testWidgets('container has decoration with border', (tester) async {
      await pumpTag(tester, SensitivityLevel.public);
      final container = tester.widget<Container>(find.byType(Container));
      final decoration = container.decoration as BoxDecoration?;
      expect(decoration, isNotNull);
      expect(decoration!.border, isNotNull);
      expect(decoration.borderRadius, isNotNull);
    });
  });

  group('getSensitivityLabel', () {
    test('returns correct labels', () {
      expect(getSensitivityLabel(SensitivityLevel.public), 'Public');
      expect(getSensitivityLabel(SensitivityLevel.internal), 'Internal');
      expect(getSensitivityLabel(SensitivityLevel.sensitive), 'Sensitive');
      expect(getSensitivityLabel(SensitivityLevel.critical), 'Critical');
    });
  });

  group('getSensitivityColor', () {
    test('returns distinct colors for all levels', () {
      final public = getSensitivityColor(SensitivityLevel.public);
      final internal = getSensitivityColor(SensitivityLevel.internal);
      final sensitive = getSensitivityColor(SensitivityLevel.sensitive);
      final critical = getSensitivityColor(SensitivityLevel.critical);

      expect(public, isNot(equals(internal)));
      expect(internal, isNot(equals(sensitive)));
      expect(sensitive, isNot(equals(critical)));
    });

    test('public is green', () {
      expect(getSensitivityColor(SensitivityLevel.public), Colors.green);
    });

    test('internal is blue', () {
      expect(getSensitivityColor(SensitivityLevel.internal), Colors.blue);
    });

    test('sensitive is orange', () {
      expect(getSensitivityColor(SensitivityLevel.sensitive), Colors.orange);
    });

    test('critical is red shade', () {
      expect(getSensitivityColor(SensitivityLevel.critical), Colors.red.shade900);
    });
  });
}
