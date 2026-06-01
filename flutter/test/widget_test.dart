import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/main.dart';

void main() {
  testWidgets('App launches and shows splash screen', (WidgetTester tester) async {
    // Build our app and trigger a frame.
    await tester.pumpWidget(
      const ProviderScope(
        child: SoloSoulApp(),
      ),
    );

    // Allow animations to run briefly (splash has infinite animations)
    await tester.pump(const Duration(seconds: 2));

    // Verify that the splash page shows the app name
    expect(find.text('SoloSoul'), findsOneWidget);
  });
}
