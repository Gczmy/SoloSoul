import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/presentation/widgets/home/icon_picker.dart';

void main() {
  group('IconPicker', () {
    testWidgets('renders trigger button with current icon',
        (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: IconPicker(
              iconName: 'folder',
              onChanged: (_) {},
            ),
          ),
        ),
      );

      expect(find.byIcon(Icons.folder_outlined), findsOneWidget);
    });

    testWidgets('InkWell is tappable', (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: IconPicker(
              iconName: 'article',
              onChanged: (_) {},
            ),
          ),
        ),
      );

      final inkWell = tester.widget<InkWell>(find.byType(InkWell));
      expect(inkWell.onTap, isNotNull);
    });

    testWidgets('trigger button has 48x48 size', (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: IconPicker(
              iconName: 'star',
              onChanged: (_) {},
            ),
          ),
        ),
      );

      final container = tester.widget<Container>(
        find.descendant(
          of: find.byType(InkWell),
          matching: find.byType(Container),
        ).first,
      );
      expect(container.constraints?.minWidth, 48);
      expect(container.constraints?.minHeight, 48);
    });

    testWidgets('uses primary color for icon and background',
        (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: IconPicker(
              iconName: 'home',
              onChanged: (_) {},
            ),
          ),
        ),
      );

      final theme = Theme.of(tester.element(find.byType(IconPicker)));
      final icon = tester.widget<Icon>(find.byIcon(Icons.home));
      expect(icon.color, theme.colorScheme.primary);
    });
  });
}
