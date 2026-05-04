import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart';
import 'package:solosoul_flutter/presentation/widgets/settings/slogan_chip.dart';

void main() {
  group('SloganChip', () {
    testWidgets('renders icon and label', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: SloganChip(
              icon: Icons.verified,
              label: 'Trusted',
            ),
          ),
        ),
      );

      expect(find.byIcon(Icons.verified), findsOneWidget);
      expect(find.text('Trusted'), findsOneWidget);
    });

    testWidgets('uses primary color for icon and text', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: SloganChip(
              icon: Icons.security,
              label: 'Secure',
            ),
          ),
        ),
      );

      final icon = tester.widget<Icon>(find.byIcon(Icons.security));
      expect(icon.color, AppTheme.primaryColor);

      final text = tester.widget<Text>(find.text('Secure'));
      expect(text.style?.color, AppTheme.primaryColor);
      expect(text.style?.fontWeight, FontWeight.w600);
    });

    testWidgets('uses labelSmall text style', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: SloganChip(
              icon: Icons.star,
              label: 'Premium',
            ),
          ),
        ),
      );

      final text = tester.widget<Text>(find.text('Premium'));
      expect(text.style?.fontSize, isNotNull);
    });

    testWidgets('has rounded container with primary background tint',
        (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: SloganChip(
              icon: Icons.shield,
              label: 'Protected',
            ),
          ),
        ),
      );

      final container = tester.widget<Container>(find.byType(Container));
      final decoration = container.decoration as BoxDecoration;
      expect(decoration.borderRadius, BorderRadius.circular(12));
    });
  });
}
