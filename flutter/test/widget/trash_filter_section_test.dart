import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/providers/trash_filter_provider.dart';
import 'package:solosoul_flutter/presentation/widgets/generic_filter_section.dart';
import 'package:solosoul_flutter/presentation/widgets/trash/trash_filter_section.dart';

Widget wrap({required Widget child, required List overrides}) {
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
  group('TrashFilterSection', () {
    testWidgets('renders filter groups', (tester) async {
      await tester.pumpWidget(wrap(
        child: const TrashFilterSection(resultCount: 5),
        overrides: [
          trashTimeFilterProvider.overrideWithValue(null),
          trashTypeFilterProvider.overrideWithValue(const {}),
        ],
      ));

      expect(find.byType(GenericFilterSection<String>), findsOneWidget);
    });

    testWidgets('toggles collapse', (tester) async {
      await tester.pumpWidget(wrap(
        child: const TrashFilterSection(resultCount: 3),
        overrides: [
          trashTimeFilterProvider.overrideWithValue(null),
          trashTypeFilterProvider.overrideWithValue(const {}),
        ],
      ));

      // Tap header to collapse
      await tester.tap(find.byType(InkWell).first);
      await tester.pump();
      // Should still render without error
      expect(find.byType(GenericFilterSection<String>), findsOneWidget);
    });

    testWidgets('renders with active filters', (tester) async {
      await tester.pumpWidget(wrap(
        child: const TrashFilterSection(resultCount: 2),
        overrides: [
          trashTimeFilterProvider.overrideWithValue('10days'),
          trashTypeFilterProvider.overrideWithValue(const {'page'}),
        ],
      ));

      expect(find.byType(GenericFilterSection<String>), findsOneWidget);
    });
  });
}
