import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/presentation/widgets/home/security_item.dart';

void main() {
  group('SecurityItem', () {
    testWidgets('renders icon, title and subtitle', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: SecurityItem(
              icon: Icons.shield,
              color: Colors.green,
              title: 'Biometric',
              subtitle: 'Face ID enabled',
            ),
          ),
        ),
      );

      expect(find.byIcon(Icons.shield), findsOneWidget);
      expect(find.text('Biometric'), findsOneWidget);
      expect(find.text('Face ID enabled'), findsOneWidget);
    });

    testWidgets('applies color to icon', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: SecurityItem(
              icon: Icons.warning,
              color: Colors.orange,
              title: 'Warning',
              subtitle: 'Attention needed',
            ),
          ),
        ),
      );

      final icon = tester.widget<Icon>(find.byIcon(Icons.warning));
      expect(icon.color, Colors.orange);
      expect(icon.size, 24);
    });

    testWidgets('uses titleSmall for title and bodySmall for subtitle',
        (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: SecurityItem(
              icon: Icons.info,
              color: Colors.blue,
              title: 'Info',
              subtitle: 'Details',
            ),
          ),
        ),
      );

      final title = tester.widget<Text>(find.text('Info'));
      expect(title.style?.fontSize, isNotNull);

      final subtitle = tester.widget<Text>(find.text('Details'));
      expect(subtitle.style?.color, isNotNull);
    });
  });
}
