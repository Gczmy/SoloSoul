import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/presentation/widgets/home/dashed_placeholder.dart';

void main() {
  group('DashedPlaceholder', () {
    testWidgets('renders with default size', (tester) async {
      await tester.pumpWidget(const MaterialApp(
        home: Scaffold(body: DashedPlaceholder()),
      ));

      expect(find.byType(DashedPlaceholder), findsOneWidget);
    });

    testWidgets('renders with child', (tester) async {
      await tester.pumpWidget(const MaterialApp(
        home: Scaffold(
          body: DashedPlaceholder(child: Text('test')),
        ),
      ));

      expect(find.text('test'), findsOneWidget);
    });

    testWidgets('renders with custom color', (tester) async {
      await tester.pumpWidget(const MaterialApp(
        home: Scaffold(
          body: DashedPlaceholder(color: Colors.red),
        ),
      ));

      expect(find.byType(DashedPlaceholder), findsOneWidget);
    });
  });

  group('DashedBorderPainter', () {
    test('shouldRepaint returns false for same color', () {
      const painter1 = DashedBorderPainter(color: Colors.blue);
      const painter2 = DashedBorderPainter(color: Colors.blue);
      expect(painter1.shouldRepaint(painter2), isFalse);
    });

    test('shouldRepaint always returns false', () {
      const painter1 = DashedBorderPainter(color: Colors.blue);
      const painter2 = DashedBorderPainter(color: Colors.red);
      expect(painter1.shouldRepaint(painter2), isFalse);
    });
  });
}
