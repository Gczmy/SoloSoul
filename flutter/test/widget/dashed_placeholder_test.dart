import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/presentation/widgets/home/dashed_placeholder.dart';

void main() {
  group('DashedPlaceholder', () {
    testWidgets('renders as 90x90 SizedBox', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(body: DashedPlaceholder()),
        ),
      );

      final size = tester.getSize(find.byType(DashedPlaceholder));
      expect(size.width, 90);
      expect(size.height, 90);
    });

    testWidgets('contains CustomPaint with DashedBorderPainter',
        (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(body: DashedPlaceholder()),
        ),
      );

      final customPaint = tester.widget<CustomPaint>(find.descendant(
        of: find.byType(DashedPlaceholder),
        matching: find.byType(CustomPaint),
      ));
      expect(customPaint.painter, isA<DashedBorderPainter>());
    });

    testWidgets('renders child when provided', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: DashedPlaceholder(
              child: Text('Test Child'),
            ),
          ),
        ),
      );

      expect(find.text('Test Child'), findsOneWidget);
    });

    testWidgets('uses default color when none provided', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(body: DashedPlaceholder()),
        ),
      );

      final customPaint = tester.widget<CustomPaint>(find.descendant(
        of: find.byType(DashedPlaceholder),
        matching: find.byType(CustomPaint),
      ));
      final painter = customPaint.painter as DashedBorderPainter;
      final theme = Theme.of(tester.element(find.byType(DashedPlaceholder)));
      expect(painter.color, theme.colorScheme.primary.withValues(alpha: 0.4));
    });

    testWidgets('uses custom color when provided', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: DashedPlaceholder(color: Colors.red),
          ),
        ),
      );

      final customPaint = tester.widget<CustomPaint>(find.descendant(
        of: find.byType(DashedPlaceholder),
        matching: find.byType(CustomPaint),
      ));
      final painter = customPaint.painter as DashedBorderPainter;
      expect(painter.color, Colors.red);
    });

    testWidgets('DashedBorderPainter shouldRepaint returns false',
        (tester) async {
      const painter1 = DashedBorderPainter(color: Colors.blue);
      const painter2 = DashedBorderPainter(color: Colors.blue);

      expect(painter1.shouldRepaint(painter2), isFalse);
    });
  });
}
