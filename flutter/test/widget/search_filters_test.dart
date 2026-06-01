import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/providers/search_provider.dart';
import 'package:solosoul_flutter/presentation/widgets/generic_filter_section.dart';
import 'package:solosoul_flutter/presentation/widgets/search_filters.dart';

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
  group('SearchFilters', () {
    testWidgets('renders filter section', (tester) async {
      await tester.pumpWidget(wrap(
        child: const SearchFilters(resultCount: 5),
        overrides: [
          searchProvider.overrideWith(() => SearchNotifier()),
        ],
      ));

      expect(find.byType(GenericFilterSection<SensitivityLevel>), findsOneWidget);
    });
  });
}
