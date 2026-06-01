import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/widgets/generic_filter_section.dart';

Widget wrap(Widget child) {
  return ProviderScope(
    child: MaterialApp(
      localizationsDelegates: AppLocalizations.localizationsDelegates,
      supportedLocales: AppLocalizations.supportedLocales,
      home: Scaffold(body: child),
    ),
  );
}

void main() {
  group('GenericFilterSection', () {
    testWidgets('renders header with result count', (tester) async {
      await tester.pumpWidget(wrap(const GenericFilterSection<int>(
        filterGroups: [],
        resultCount: 42,
      )));

      expect(find.text('Filters'), findsOneWidget);
      expect(find.text('42'), findsOneWidget);
    });

    testWidgets('renders filter chips and toggles selection', (tester) async {
      final group1 = FilterGroup<String>(
        label: 'Type',
        options: const [
          FilterOption(id: 'a', label: 'A', icon: Icons.label, color: Colors.red),
          FilterOption(id: 'b', label: 'B', icon: Icons.label, color: Colors.blue),
        ],
        selectedIds: const {'a'},
        onSelectionChanged: (_) {},
      );

      await tester.pumpWidget(wrap(GenericFilterSection<String>(
        filterGroups: [group1],
        resultCount: 5,
      )));

      expect(find.text('Type'), findsOneWidget);
      expect(find.text('A'), findsOneWidget);
      expect(find.text('B'), findsOneWidget);
    });

    testWidgets('multi-select toggles chip on tap', (tester) async {
      Set<String> currentSelection = const {'a'};
      await tester.pumpWidget(wrap(StatefulBuilder(
        builder: (context, setState) {
          return GenericFilterSection<String>(
            filterGroups: [
              FilterGroup<String>(
                label: 'Tag',
                options: const [
                  FilterOption(id: 'a', label: 'A', icon: Icons.label, color: Colors.red),
                  FilterOption(id: 'b', label: 'B', icon: Icons.label, color: Colors.blue),
                ],
                selectedIds: currentSelection,
                onSelectionChanged: (sel) => setState(() => currentSelection = sel),
              ),
            ],
            resultCount: 3,
          );
        },
      )));

      await tester.tap(find.byType(FilterChip).at(1));
      await tester.pump();
      expect(currentSelection, contains('b'));
    });

    testWidgets('single-select clears previous selection', (tester) async {
      Set<String> currentSelection = const {'a'};
      await tester.pumpWidget(wrap(StatefulBuilder(
        builder: (context, setState) {
          return GenericFilterSection<String>(
            filterGroups: [
              FilterGroup<String>(
                label: 'Mode',
                options: const [
                  FilterOption(id: 'a', label: 'A', icon: Icons.label, color: Colors.red),
                  FilterOption(id: 'b', label: 'B', icon: Icons.label, color: Colors.blue),
                ],
                selectedIds: currentSelection,
                onSelectionChanged: (sel) => setState(() => currentSelection = sel),
                singleSelect: true,
              ),
            ],
            resultCount: 3,
          );
        },
      )));

      await tester.tap(find.byType(FilterChip).at(1));
      await tester.pump();
      expect(currentSelection, equals({'b'}));
    });

    testWidgets('collapses when tapped and expanded is false', (tester) async {
      bool toggled = false;
      await tester.pumpWidget(wrap(GenericFilterSection<int>(
        filterGroups: const [],
        resultCount: 0,
        expanded: false,
        onToggle: () => toggled = true,
      )));

      expect(find.byType(AnimatedRotation), findsOneWidget);
      await tester.tap(find.byType(InkWell).first);
      await tester.pump();
      expect(toggled, isTrue);
    });

    testWidgets('does not collapse when collapsible is false', (tester) async {
      bool toggled = false;
      await tester.pumpWidget(wrap(GenericFilterSection<int>(
        filterGroups: const [],
        resultCount: 0,
        collapsible: false,
        onToggle: () => toggled = true,
      )));

      await tester.tap(find.byType(InkWell).first);
      await tester.pump();
      expect(toggled, isFalse);
    });

    testWidgets('shows clear all button when active filters exist', (tester) async {
      bool cleared = false;
      await tester.pumpWidget(wrap(GenericFilterSection<String>(
        filterGroups: [
          FilterGroup<String>(
            label: 'Tag',
            options: const [
              FilterOption(id: 'a', label: 'A', icon: Icons.label, color: Colors.red),
            ],
            selectedIds: const {'a'},
            onSelectionChanged: (_) {},
          ),
        ],
        resultCount: 1,
        showClearAll: true,
        onClearAll: () => cleared = true,
      )));

      expect(find.byIcon(Icons.clear_all), findsOneWidget);
      await tester.tap(find.byIcon(Icons.clear_all));
      await tester.pump();
      expect(cleared, isTrue);
    });

    testWidgets('hides clear all when only all is selected in single select', (tester) async {
      await tester.pumpWidget(wrap(GenericFilterSection<String>(
        filterGroups: [
          FilterGroup<String>(
            label: 'Mode',
            options: const [
              FilterOption(id: 'all', label: 'All', icon: Icons.label, color: Colors.grey),
            ],
            selectedIds: const {'all'},
            onSelectionChanged: (_) {},
            singleSelect: true,
          ),
        ],
        resultCount: 1,
        showClearAll: true,
      )));

      expect(find.byIcon(Icons.clear_all), findsNothing);
    });

    testWidgets('uses custom header label and icon', (tester) async {
      await tester.pumpWidget(wrap(const GenericFilterSection<int>(
        filterGroups: [],
        resultCount: 0,
        headerLabel: 'Custom Header',
        headerIcon: Icons.sort,
      )));

      expect(find.text('Custom Header'), findsOneWidget);
    });
  });
}
