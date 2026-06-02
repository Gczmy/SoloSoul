import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/models/search_models.dart';
import 'package:solosoul_flutter/presentation/providers/account_style_provider.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';
import 'package:solosoul_flutter/presentation/widgets/search_result_tile.dart';

Widget wrap(Widget child) {
  return ProviderScope(
    overrides: [
      accountStyleProvider.overrideWith(() => AccountStyleNotifier()),
      isSensitiveAccessGrantedProvider.overrideWithValue(false),
    ],
    child: MaterialApp(
      localizationsDelegates: AppLocalizations.localizationsDelegates,
      supportedLocales: AppLocalizations.supportedLocales,
      home: Scaffold(body: child),
    ),
  );
}

void main() {
  group('SearchResultTile', () {
    testWidgets('renders public field without mask', (tester) async {
      const result = SearchResultItem(
        fieldPath: 'profile.name',
        fieldName: 'Name',
        section: 'profile',
        sectionDisplayName: 'Profile',
        value: 'Alice',
        sensitivityLevel: SensitivityLevel.public,
      );

      await tester.pumpWidget(wrap(SearchResultTile(
        result: result,
        onReveal: () {},
      )));

      expect(find.text('Alice'), findsOneWidget);
      expect(find.text('Name'), findsOneWidget);
    });

    testWidgets('masks sensitive field', (tester) async {
      const result = SearchResultItem(
        fieldPath: 'profile.password',
        fieldName: 'Password',
        section: 'profile',
        sectionDisplayName: 'Profile',
        value: 'secret123',
        sensitivityLevel: SensitivityLevel.sensitive,
      );

      await tester.pumpWidget(wrap(SearchResultTile(
        result: result,
        onReveal: () {},
      )));

      expect(find.text('••••••••'), findsOneWidget);
      expect(find.text('secret123'), findsNothing);
    });

    testWidgets('shows reveal button for masked field', (tester) async {
      const result = SearchResultItem(
        fieldPath: 'profile.secret',
        fieldName: 'Secret',
        section: 'profile',
        sectionDisplayName: 'Profile',
        value: 'hidden',
        sensitivityLevel: SensitivityLevel.sensitive,
      );

      await tester.pumpWidget(wrap(SearchResultTile(
        result: result,
        onReveal: () {},
      )));

      expect(find.byIcon(Icons.visibility_off), findsOneWidget);
    });

    testWidgets('shows deleted badge when isDeleted', (tester) async {
      const result = SearchResultItem(
        fieldPath: 'profile.old',
        fieldName: 'Old Field',
        section: 'profile',
        sectionDisplayName: 'Profile',
        value: 'old',
        sensitivityLevel: SensitivityLevel.public,
        isDeleted: true,
      );

      await tester.pumpWidget(wrap(SearchResultTile(
        result: result,
        onReveal: () {},
      )));

      expect(find.byType(Card), findsOneWidget);
    });
  });
}
