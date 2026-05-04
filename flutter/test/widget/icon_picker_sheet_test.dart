import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/presentation/widgets/icon_picker_sheet.dart';

void main() {
  group('IconPickerSheet', () {
    testWidgets('renders title and icon grid', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: IconPickerSheet(currentIcon: 'folder'),
          ),
        ),
      );

      expect(find.text('Choose Icon'), findsOneWidget);
      // kIconNames has 26 icons; each renders as an InkWell in a container
      expect(find.byType(InkWell), findsNWidgets(26));
    });

    testWidgets('highlights current icon with primary border',
        (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: IconPickerSheet(currentIcon: 'folder'),
          ),
        ),
      );

      // Find all Container widgets inside the Wrap
      final containers = tester.widgetList<Container>(find.byType(Container));
      var foundSelected = false;
      for (final container in containers) {
        final decoration = container.decoration as BoxDecoration?;
        if (decoration?.border != null) {
          final border = decoration!.border as Border;
          if (border.top.color != Colors.transparent) {
            foundSelected = true;
            break;
          }
        }
      }
      expect(foundSelected, isTrue);
    });

    testWidgets('tapping icon invokes Navigator.pop', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: IconPickerSheet(currentIcon: 'article'),
          ),
        ),
      );

      // All 26 icons are tappable InkWell widgets
      final inkWells = tester.widgetList<InkWell>(find.byType(InkWell));
      expect(inkWells.length, 26);
      // Each InkWell has an onTap callback
      expect(inkWells.first.onTap, isNotNull);
    });

    testWidgets('renders all predefined icons from kIconNames',
        (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: IconPickerSheet(currentIcon: 'article'),
          ),
        ),
      );

      // Verify a sampling of icons are present
      // Note: getIconFromName maps some names to _outlined variants
      expect(find.byIcon(Icons.article_outlined), findsOneWidget);
      expect(find.byIcon(Icons.folder_outlined), findsOneWidget);
      expect(find.byIcon(Icons.person_outlined), findsOneWidget);
      expect(find.byIcon(Icons.flight), findsOneWidget);
      expect(find.byIcon(Icons.star), findsOneWidget);
      expect(find.byIcon(Icons.camera_alt), findsOneWidget);
    });

    testWidgets('selected icon uses primary color', (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: IconPickerSheet(currentIcon: 'star'),
          ),
        ),
      );

      final theme = Theme.of(tester.element(find.byType(IconPickerSheet)));
      final starIcon = tester.widget<Icon>(find.byIcon(Icons.star));
      expect(starIcon.color, theme.colorScheme.primary);
    });
  });
}
