import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/pages/profile_page.dart';
import 'package:solosoul_flutter/presentation/providers/unified_object_provider.dart';
import 'package:solosoul_flutter/presentation/theme/glass_adapters.dart';

/// Minimal placeholder properties so [ObjectCard] treats sections as having a schema.
Map<String, PropertyValue> _placeholderProperties() => {
      'title': const TextProperty(text: '', sensitivity: SensitivityLevel.public),
    };

/// Build mock [UnifiedObjectData] with the four default Profile sections.
UnifiedObjectData _mockProfileData() {
  return UnifiedObjectData(
    objects: [
      UnifiedObject(
        id: '__page_profile',
        typeId: 'page',
        name: 'Profile',
        iconName: 'person',
        parentId: null,
        childrenIds: const [
          '__section_identity',
          '__section_contact',
          '__section_id_card',
          '__section_address',
        ],
        properties: const {},
        createdAt: 0,
        updatedAt: 0,
      ),
      UnifiedObject(
        id: '__section_identity',
        typeId: 'profile_identity',
        name: 'Identity Profile',
        iconName: 'person',
        parentId: '__page_profile',
        childrenIds: const [],
        properties: _placeholderProperties(),
        createdAt: 0,
        updatedAt: 0,
      ),
      UnifiedObject(
        id: '__section_contact',
        typeId: 'profile_contact',
        name: 'Contact Information',
        iconName: 'contact_mail',
        parentId: '__page_profile',
        childrenIds: const [],
        properties: _placeholderProperties(),
        createdAt: 0,
        updatedAt: 0,
      ),
      UnifiedObject(
        id: '__section_id_card',
        typeId: 'profile_id_card',
        name: 'Identity Documents',
        iconName: 'badge',
        parentId: '__page_profile',
        childrenIds: const [],
        properties: _placeholderProperties(),
        createdAt: 0,
        updatedAt: 0,
      ),
      UnifiedObject(
        id: '__section_address',
        typeId: 'profile_address',
        name: 'Addresses',
        iconName: 'home',
        parentId: '__page_profile',
        childrenIds: const [],
        properties: _placeholderProperties(),
        createdAt: 0,
        updatedAt: 0,
      ),
    ],
    customTypes: const [],
  );
}

class _TestUnifiedObjectNotifier extends UnifiedObjectNotifier {
  final UnifiedObjectData _data;

  _TestUnifiedObjectNotifier(this._data);

  @override
  UnifiedObjectData build() => _data;
}

Widget _buildProfilePageWithData(UnifiedObjectData data) {
  return ProviderScope(
    overrides: [
      unifiedObjectProvider.overrideWith(() => _TestUnifiedObjectNotifier(data)),
    ],
    child: MaterialApp(
      localizationsDelegates: AppLocalizations.localizationsDelegates,
      supportedLocales: AppLocalizations.supportedLocales,
      home: const ProfilePage(),
    ),
  );
}

void main() {
  group('ProfilePage Widget Tests', () {
    testWidgets('renders profile page with scaffold', (tester) async {
      await tester.pumpWidget(_buildProfilePageWithData(_mockProfileData()));
      await tester.pumpAndSettle();

      expect(find.byType(Scaffold), findsOneWidget);
    });

    testWidgets('has app bar with Profile title', (tester) async {
      await tester.pumpWidget(_buildProfilePageWithData(_mockProfileData()));
      await tester.pumpAndSettle();

      expect(find.text('Profile'), findsOneWidget);
      expect(find.byType(SoloGlassAppBar), findsOneWidget);
    });

    testWidgets('shows Identity Profile section', (tester) async {
      await tester.pumpWidget(_buildProfilePageWithData(_mockProfileData()));
      await tester.pumpAndSettle();

      expect(find.text('Identity Profile'), findsOneWidget);
    });

    testWidgets('shows Contact Information section', (tester) async {
      await tester.pumpWidget(_buildProfilePageWithData(_mockProfileData()));
      await tester.pumpAndSettle();

      expect(find.text('Contact Information'), findsOneWidget);
    });

    testWidgets('shows Identity Documents section', (tester) async {
      await tester.pumpWidget(_buildProfilePageWithData(_mockProfileData()));
      await tester.pumpAndSettle();

      expect(find.text('Identity Documents'), findsOneWidget);
    });

    testWidgets('shows Addresses section', (tester) async {
      await tester.pumpWidget(_buildProfilePageWithData(_mockProfileData()));
      await tester.pumpAndSettle();

      expect(find.text('Addresses'), findsOneWidget);
    });
  });
}
