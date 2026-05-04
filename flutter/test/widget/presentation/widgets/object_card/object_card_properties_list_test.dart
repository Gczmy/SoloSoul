import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/presentation/widgets/object_card/object_card_properties_list.dart';

void main() {
  group('ObjectCardPropertiesList', () {
    final object = UnifiedObject(
      id: 'o1',
      typeId: 'item',
      name: 'Test',
      iconName: 'folder',
      parentId: null,
      childrenIds: const [],
      properties: const {},
      isDeleted: false,
      deletedAt: null,
      createdAt: 0,
      updatedAt: 0,
    );

    PropertyValue _prop(dynamic value, SensitivityLevel sensitivity) {
      return TextProperty(text: value.toString(), sensitivity: sensitivity);
    }

    testWidgets('renders properties excluding title key', (tester) async {
      await tester.pumpWidget(
        ProviderScope(
          child: MaterialApp(
            home: Scaffold(
              body: ObjectCardPropertiesList(
                item: object.copyWith(properties: {
                  'Title': _prop('My Title', SensitivityLevel.public),
                  'Description': _prop('Details', SensitivityLevel.public),
                  'Amount': _prop('100', SensitivityLevel.public),
                }),
              ),
            ),
          ),
        ),
      );

      expect(find.text('My Title'), findsNothing); // excluded
      expect(find.text('Details'), findsOneWidget);
      expect(find.text('100'), findsOneWidget);
    });

    testWidgets('renders empty value placeholder', (tester) async {
      await tester.pumpWidget(
        ProviderScope(
          child: MaterialApp(
            home: Scaffold(
              body: ObjectCardPropertiesList(
                item: object.copyWith(properties: {
                  'EmptyField': _prop('', SensitivityLevel.public),
                }),
              ),
            ),
          ),
        ),
      );

      expect(find.text('(empty)'), findsOneWidget);
    });

    testWidgets('renders sensitivity tags', (tester) async {
      await tester.pumpWidget(
        ProviderScope(
          child: MaterialApp(
            home: Scaffold(
              body: SingleChildScrollView(
                child: ObjectCardPropertiesList(
                  item: object.copyWith(properties: {
                    'Public': _prop('x', SensitivityLevel.public),
                    'Internal': _prop('x', SensitivityLevel.internal),
                    'Sensitive': _prop('x', SensitivityLevel.sensitive),
                    'Critical': _prop('x', SensitivityLevel.critical),
                  }),
                ),
              ),
            ),
          ),
        ),
      );

      // SensitivityTag renders text for each level
      expect(find.text('Public'), findsOneWidget);
      expect(find.text('Internal'), findsOneWidget);
      expect(find.text('Sensitive'), findsOneWidget);
      expect(find.text('Critical'), findsOneWidget);
    });

    testWidgets('uses custom titlePropertyKey', (tester) async {
      await tester.pumpWidget(
        ProviderScope(
          child: MaterialApp(
            home: Scaffold(
              body: ObjectCardPropertiesList(
                item: object.copyWith(properties: {
                  'Name': _prop('My Name', SensitivityLevel.public),
                  'Other': _prop('Other Value', SensitivityLevel.public),
                }),
                titlePropertyKey: 'Name',
              ),
            ),
          ),
        ),
      );

      expect(find.text('My Name'), findsNothing); // excluded
      expect(find.text('Other Value'), findsOneWidget);
    });

    testWidgets('formats label with wrapEveryNChars', (tester) async {
      await tester.pumpWidget(
        ProviderScope(
          child: MaterialApp(
            home: Scaffold(
              body: ObjectCardPropertiesList(
                item: object.copyWith(properties: {
                  'ShortName': _prop('val', SensitivityLevel.public),
                }),
              ),
            ),
          ),
        ),
      );

      // Short labels are formatted with colon appended by wrapEveryNChars
      expect(find.textContaining('Short Name'), findsOneWidget);
    });

    testWidgets('renders SelectableText for non-sensitive values', (tester) async {
      await tester.pumpWidget(
        ProviderScope(
          child: MaterialApp(
            home: Scaffold(
              body: ObjectCardPropertiesList(
                item: object.copyWith(properties: {
                  'Normal': _prop('Plaintext', SensitivityLevel.public),
                }),
              ),
            ),
          ),
        ),
      );

      expect(find.text('Plaintext'), findsOneWidget);
      expect(find.byType(SelectableText), findsWidgets);
    });
  });
}
