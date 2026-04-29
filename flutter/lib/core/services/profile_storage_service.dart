import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:isolate';
import 'dart:typed_data';

import 'package:path_provider/path_provider.dart';
import 'package:uuid/uuid.dart';

import 'package:solosoul_flutter/core/models/profile_data.dart';
export 'package:solosoul_flutter/core/models/profile_data.dart';

import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/presentation/models/sensitivity_models.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';

import 'package:solosoul_flutter/core/services/debug_logger.dart';
import 'package:solosoul_flutter/core/services/rust_vault_service.dart';
import 'package:solosoul_flutter/core/services/unified_object_service.dart';

const _uuid = Uuid();

/// Generates a new unique ID using UUID v4
String generateEntryId() => _uuid.v4();


/// Profile storage service - stores encrypted profile data locally
/// Delegates to RustVaultService via FFI for SQLCipher-encrypted storage
// TODO: [P2] ProfileStorageService is 700+ lines - consider extracting:
// - DeletedItemInfo caching logic to a separate service
// - restoreItem/permanentDeleteItem to a TrashService class
class ProfileStorageService {
  static ProfileStorageService? _instance;

  // Reference to Rust vault service
  final RustVaultService _rustVault = RustVaultService.instance;

  ProfileStorageService._();

  static ProfileStorageService get instance {
    _instance ??= ProfileStorageService._();
    return _instance!;
  }

  /// Set the encryption key (derived from master password)
  /// Also sets it on the RustVaultService
  void setEncryptionKey(Uint8List key) {
    _rustVault.setEncryptionKey(key);
  }

  /// Get the encryption key (for use by OperationLogService)
  Uint8List? get encryptionKey => _rustVault.encryptionKey;

  /// Get the storage directory for logs and other files
  /// Uses the app's documents directory
  Future<Directory> get storageDir async {
    final appDir = await getApplicationDocumentsDirectory();
    return Directory('${appDir.path}/solosoul_storage');
  }

  /// Clear the encryption key (on lock)
  void clearEncryptionKey() {
    _rustVault.clearEncryptionKey();
    _invalidateDeletedItemsCache();
  }

  // Caching for getDeletedItems - invalidated on any profile mutation
  List<DeletedItemInfo>? _cachedDeletedItems;

  void _invalidateDeletedItemsCache() {
    _cachedDeletedItems = null;
  }

  /// Current schema version for unified object model.
  static const int kSchemaVersion = 4;

  /// Migrate profile to latest schema if needed.
  /// v3: Unified object model - everything is a UnifiedObject.
  /// v4: Default pages (profile/travel/financial/professional) migrated to UnifiedObject tree.
  static ProfileData _migrateIfNeeded(ProfileData profile, Map<String, dynamic> rawJson) {
    var currentVersion = profile.schemaVersion ?? 0;
    var migrated = profile;

    // Recovery guard: if unifiedObjects is empty/missing default pages but
    // legacy fields still have data, re-run migration regardless of schemaVersion.
    // This handles cases where unifiedObjects was accidentally wiped.
    final unifiedObjects = migrated.unifiedObjects;
    final hasDefaultPages = unifiedObjects?.objects.any(
          (o) => o.id == DefaultPageIds.profile || o.id == DefaultPageIds.travel,
        ) ?? false;
    final hasLegacyData = migrated.identity != null ||
        migrated.travel != null ||
        migrated.financial != null ||
        migrated.professional != null;

    if (!hasDefaultPages && hasLegacyData) {
      if (currentVersion < 3) {
        if (unifiedObjects == null || unifiedObjects.objects.isEmpty) {
          final unifiedData = _migrateLegacyToUnified(rawJson);
          migrated = migrated.copyWith(unifiedObjects: unifiedData);
        }
        currentVersion = 3;
      }
      final existingData = migrated.unifiedObjects ?? const UnifiedObjectData();
      final migratedObjects = _migrateProfileDataToUnified(migrated, existingData);
      return migrated.copyWith(
        unifiedObjects: migratedObjects,
        schemaVersion: kSchemaVersion,
      );
    }

    if (currentVersion >= kSchemaVersion) return profile;

    // v0/v1/v2 → v3: migrate any legacy flexibleObjects/flexibleSections to UnifiedObjectData
    if (currentVersion < 3) {
      if (migrated.unifiedObjects == null || migrated.unifiedObjects!.objects.isEmpty) {
        final unifiedData = _migrateLegacyToUnified(rawJson);
        migrated = migrated.copyWith(
          unifiedObjects: unifiedData,
          schemaVersion: 3,
        );
      } else {
        migrated = migrated.copyWith(schemaVersion: 3);
      }
      currentVersion = 3;
    }

    // v3 → v4: migrate default page data (identity/travel/financial/professional)
    // into the UnifiedObject tree with predefined schemas.
    if (currentVersion < 4) {
      final existingData = migrated.unifiedObjects ?? const UnifiedObjectData();
      final hasDefaultPages = existingData.objects.any(
        (o) => o.id == DefaultPageIds.profile || o.id == DefaultPageIds.travel,
      );
      if (!hasDefaultPages) {
        final migratedObjects = _migrateProfileDataToUnified(migrated, existingData);
        migrated = migrated.copyWith(
          unifiedObjects: migratedObjects,
          schemaVersion: kSchemaVersion,
        );
      } else {
        migrated = migrated.copyWith(schemaVersion: kSchemaVersion);
      }
    }

    return migrated;
  }

  /// Validate and repair data integrity after migration.
  ///
  /// Checks performed:
  /// - Duplicate UnifiedObject IDs (keep first occurrence)
  /// - Invalid [childrenIds] references (remove IDs pointing to non-existent objects)
  /// - Invalid [parentId] references (set to null if parent no longer exists)
  ///
  /// Returns a repaired copy if fixes were applied, or the original if valid.
  static (ProfileData, bool) _validateAndRepairProfile(ProfileData profile) {
    var repaired = profile;
    var wasRepaired = false;

    final unifiedObjects = repaired.unifiedObjects;
    if (unifiedObjects != null && unifiedObjects.objects.isNotEmpty) {
      final objectMap = <String, UnifiedObject>{};
      final seenIds = <String>{};

      // Pass 1: deduplicate by ID (keep first occurrence)
      for (final obj in unifiedObjects.objects) {
        if (seenIds.contains(obj.id)) {
          wasRepaired = true;
          continue;
        }
        seenIds.add(obj.id);
        objectMap[obj.id] = obj;
      }

      final validIds = objectMap.keys.toSet();
      final repairedObjects = <UnifiedObject>[];

      // Pass 2: repair references
      for (final obj in objectMap.values) {
        final validChildrenIds =
            obj.childrenIds.where(validIds.contains).toList();
        final fixedParentId =
            (obj.parentId != null && validIds.contains(obj.parentId))
                ? obj.parentId
                : null;

        if (validChildrenIds.length != obj.childrenIds.length ||
            fixedParentId != obj.parentId) {
          wasRepaired = true;
          repairedObjects.add(
            obj.copyWith(
              childrenIds: validChildrenIds,
              parentId: fixedParentId,
            ),
          );
        } else {
          repairedObjects.add(obj);
        }
      }

      if (wasRepaired) {
        repaired = repaired.copyWith(
          unifiedObjects: unifiedObjects.copyWith(objects: repairedObjects),
        );
      }
    }

    return (repaired, wasRepaired);
  }

  /// Migrate legacy flexibleSections / flexibleObjects to UnifiedObjectData.
  /// Operates on raw JSON maps because old type definitions have been removed.
  static UnifiedObjectData _migrateLegacyToUnified(Map<String, dynamic> rawJson) {
    final objects = <UnifiedObject>[];
    final timestamp = currentTimestamp();

    String? parseString(dynamic v) => v?.toString();
    bool parseBool(dynamic v) => v == true || v == 'true';
    int? parseMillis(dynamic v) => v is int ? v : (v is num ? v.toInt() : null);
    DateTime? parseDateTime(dynamic v) {
      if (v == null) return null;
      if (v is String) return DateTime.tryParse(v);
      return null;
    }

    // -------------------------------------------------------------------------
    // Path A: old flexibleObjects (v2/v3-early FlexibleObject model)
    // -------------------------------------------------------------------------
    final legacyObjectsRaw = rawJson['flexible_objects'] as Map<String, dynamic>?;
    final legacyObjects = legacyObjectsRaw != null
        ? (legacyObjectsRaw['objects'] as List<dynamic>? ?? [])
            .map((e) => e as Map<String, dynamic>)
            .toList()
        : const <Map<String, dynamic>>[];
    if (legacyObjects.isNotEmpty) {
      final childrenByParent = <String, List<String>>{};
      for (final o in legacyObjects) {
        final parentId = parseString(o['parentId']);
        if (parentId != null) {
          childrenByParent.putIfAbsent(parentId, () => []).add(parseString(o['id']) ?? generateEntryId());
        }
      }

      for (final o in legacyObjects) {
        final objectTypeName = parseString(o['objectType']) ?? 'item';
        final typeId = switch (objectTypeName) {
          'page' => 'page',
          'section' => 'collection',
          'item' || _ => 'note',
        };
        final id = parseString(o['id']) ?? generateEntryId();
        objects.add(UnifiedObject(
          id: id,
          typeId: typeId,
          name: parseString(o['name']) ?? 'Untitled',
          iconName: parseString(o['iconName']) ?? 'folder',
          parentId: parseString(o['parentId']),
          childrenIds: childrenByParent[id] ?? const [],
          properties: const {}, // Legacy properties used old PropertyValue; safest to drop
          isDeleted: parseBool(o['isDeleted']),
          deletedAt: parseDateTime(o['deletedAt']),
          createdAt: timestamp,
          updatedAt: parseMillis(o['updatedAt']) ?? timestamp,
        ));
      }
      return UnifiedObjectData(objects: objects);
    }

    // -------------------------------------------------------------------------
    // Path B: old flexibleSections (v1 FlexibleSection model)
    // -------------------------------------------------------------------------
    final legacySectionsRaw = rawJson['flexible_sections'] as Map<String, dynamic>?;
    final legacySections = legacySectionsRaw != null
        ? (legacySectionsRaw['sections'] as List<dynamic>? ?? [])
            .map((e) => e as Map<String, dynamic>)
            .toList()
        : const <Map<String, dynamic>>[];
    if (legacySections.isNotEmpty) {
      for (final section in legacySections) {
        if (parseBool(section['isDeleted'])) continue;

        final sectionId = parseString(section['id']) ?? generateEntryId();
        final sectionTitle = parseString(section['title']) ?? 'Untitled';
        final sectionIcon = parseString(section['iconName']) ?? 'folder';
        final pageId = 'page_$sectionId';

        // Create a synthetic page for this section
        objects.add(UnifiedObject(
          id: pageId,
          typeId: 'page',
          name: sectionTitle,
          iconName: sectionIcon,
          parentId: null,
          childrenIds: [sectionId],
          createdAt: timestamp,
          updatedAt: timestamp,
        ));

        // Convert section items
        final itemIds = <String>[];
        final items = (section['items'] as List<dynamic>? ?? [])
            .map((e) => e as Map<String, dynamic>)
            .toList();
        for (final item in items) {
          if (parseBool(item['isDeleted'])) continue;
          final itemId = parseString(item['id']) ?? generateEntryId();
          itemIds.add(itemId);
          objects.add(UnifiedObject(
            id: itemId,
            typeId: 'note',
            name: parseString(item['title']) ?? 'Untitled',
            iconName: 'description',
            parentId: sectionId,
            properties: {
              'data': TextProperty(text: jsonEncode(item['data'])),
            },
            createdAt: timestamp,
            updatedAt: parseMillis(item['updatedAt']) ?? timestamp,
          ));
        }

        // Convert section to collection
        objects.add(UnifiedObject(
          id: sectionId,
          typeId: 'collection',
          name: sectionTitle,
          iconName: sectionIcon,
          parentId: pageId,
          childrenIds: itemIds,
          isDeleted: false,
          deletedAt: parseDateTime(section['deletedAt']),
          createdAt: timestamp,
          updatedAt: parseMillis(section['updatedAt']) ?? timestamp,
        ));
      }
      return UnifiedObjectData(objects: objects);
    }

    // Empty default
    return const UnifiedObjectData();
  }

  /// Migrate legacy profile data (identity/travel/financial/professional) to
  /// the UnifiedObject tree. Creates default pages, sections, and items.
  static UnifiedObjectData _migrateProfileDataToUnified(
    ProfileData profile,
    UnifiedObjectData existingData,
  ) {
    final objects = List<UnifiedObject>.from(existingData.objects);
    final timestamp = currentTimestamp();

    TextProperty prop(String? value, SensitivityLevel sensitivity) {
      return TextProperty(text: value ?? '', sensitivity: sensitivity);
    }

    // Helper to look up sensitivity from FieldRegistry / FormFieldRegistry
    SensitivityLevel sens(String fieldId) {
      // 1. Try runtime-registered FormFieldRegistry first (contact.* etc.)
      final formField = FormFieldRegistry.getField(fieldId);
      if (formField != null) return formField.level;
      // 2. Fallback to static FieldRegistry defaults
      try {
        return FieldRegistry.defaultFields
            .firstWhere((f) => f.fieldId == fieldId)
            .level;
      } on Exception catch (_) {
        return SensitivityLevel.public;
      }
    }

    // -------------------------------------------------------------------------
    // Profile page
    // -------------------------------------------------------------------------
    final profileSectionChildren = <String>[];

    // Identity section (single item)
    if (profile.identity != null) {
      final identity = profile.identity!;
      final identityId = generateEntryId();
      profileSectionChildren.add(identityId);
      objects.add(UnifiedObject(
        id: identityId,
        typeId: 'profile_identity',
        name: identity.fullName ?? 'Identity',
        parentId: DefaultSectionIds.identity,
        properties: {
          'fullName': prop(identity.fullName, sens('identity.fullName')),
          'givenName': prop(identity.givenName, sens('identity.givenName')),
          'familyName': prop(identity.familyName, sens('identity.familyName')),
          'dateOfBirth': prop(identity.dateOfBirth, sens('identity.dateOfBirth')),
          'gender': prop(identity.gender, sens('identity.gender')),
          'nationality': prop(identity.nationality, sens('identity.nationality')),
        },
        createdAt: timestamp,
        updatedAt: timestamp,
      ));
    }

    // Contact items
    final contactChildren = <String>[];
    final contactEntries = profile.identity?.contact?.entries ?? [];
    for (final entry in contactEntries) {
      contactChildren.add(entry.id);
      objects.add(UnifiedObject(
        id: entry.id,
        typeId: 'profile_contact',
        name: entry.title.isNotEmpty ? entry.title : entry.value,
        parentId: DefaultSectionIds.contact,
        properties: {
          'title': prop(entry.title, sens('contact.title')),
          'type': prop(entry.type, sens('contact.type')),
          'value': prop(entry.value, sens('contact.value')),
        },
        isDeleted: entry.isDeleted,
        deletedAt: entry.deletedAt,
        createdAt: timestamp,
        updatedAt: entry.updatedAt,
      ));
    }

    // ID Card items
    final idCardChildren = <String>[];
    final idCards = profile.identity?.idCards ?? [];
    for (final card in idCards) {
      idCardChildren.add(card.id);
      objects.add(UnifiedObject(
        id: card.id,
        typeId: 'profile_id_card',
        name: card.title ?? 'ID Card',
        parentId: DefaultSectionIds.idCard,
        properties: {
          'title': prop(card.title, sens('idCard.title')),
          'number': prop(card.number, sens('idCard.number')),
          'issueDate': prop(card.issueDate, sens('idCard.issueDate')),
          'expiryDate': prop(card.expiryDate, sens('idCard.expiryDate')),
          'holderName': prop(card.holderName, sens('idCard.holderName')),
          'country': prop(card.country, sens('idCard.country')),
        },
        isDeleted: card.isDeleted,
        deletedAt: card.deletedAt,
        createdAt: timestamp,
        updatedAt: card.updatedAt,
      ));
    }

    // Address items
    final addressChildren = <String>[];
    final addresses = profile.identity?.addresses ?? [];
    for (final addr in addresses) {
      addressChildren.add(addr.id);
      objects.add(UnifiedObject(
        id: addr.id,
        typeId: 'profile_address',
        name: addr.title ?? 'Address',
        parentId: DefaultSectionIds.address,
        properties: {
          'title': prop(addr.title, sens('address.title')),
          'street': prop(addr.street, sens('address.street')),
          'city': prop(addr.city, sens('address.city')),
          'state': prop(addr.state, SensitivityLevel.public),
          'postalCode': prop(addr.postalCode, sens('address.postalCode')),
          'country': prop(addr.country, sens('address.country')),
        },
        isDeleted: addr.isDeleted,
        deletedAt: addr.deletedAt,
        createdAt: timestamp,
        updatedAt: addr.updatedAt,
      ));
    }

    // Build profile sections
    objects.add(UnifiedObject(
      id: DefaultSectionIds.identity,
      typeId: 'collection',
      name: 'Identity',
      iconName: 'person',
      parentId: DefaultPageIds.profile,
      childrenIds: profile.identity != null ? [profileSectionChildren.first] : const [],
      createdAt: timestamp,
      updatedAt: timestamp,
    ));
    objects.add(UnifiedObject(
      id: DefaultSectionIds.contact,
      typeId: 'collection',
      name: 'Contact Information',
      iconName: 'contact_mail',
      parentId: DefaultPageIds.profile,
      childrenIds: contactChildren,
      createdAt: timestamp,
      updatedAt: timestamp,
    ));
    objects.add(UnifiedObject(
      id: DefaultSectionIds.idCard,
      typeId: 'collection',
      name: 'ID Cards',
      iconName: 'badge',
      parentId: DefaultPageIds.profile,
      childrenIds: idCardChildren,
      createdAt: timestamp,
      updatedAt: timestamp,
    ));
    objects.add(UnifiedObject(
      id: DefaultSectionIds.address,
      typeId: 'collection',
      name: 'Addresses',
      iconName: 'home',
      parentId: DefaultPageIds.profile,
      childrenIds: addressChildren,
      createdAt: timestamp,
      updatedAt: timestamp,
    ));
    objects.add(UnifiedObject(
      id: DefaultPageIds.profile,
      typeId: 'page',
      name: 'Profile',
      iconName: 'person',
      parentId: null,
      childrenIds: const [
        DefaultSectionIds.identity,
        DefaultSectionIds.contact,
        DefaultSectionIds.idCard,
        DefaultSectionIds.address,
      ],
      createdAt: timestamp,
      updatedAt: timestamp,
    ));

    // -------------------------------------------------------------------------
    // Travel page
    // -------------------------------------------------------------------------
    final passports = profile.travel?.passports ?? [];
    final passportChildren = <String>[];
    for (final p in passports) {
      passportChildren.add(p.id);
      objects.add(UnifiedObject(
        id: p.id,
        typeId: 'travel_passport',
        name: p.title ?? p.country ?? 'Passport',
        parentId: DefaultSectionIds.passport,
        properties: {
          'title': prop(p.title, sens('passport.title')),
          'country': prop(p.country, sens('passport.country')),
          'countryCode': prop(p.countryCode, sens('passport.countryCode')),
          'number': prop(p.number, sens('passport.number')),
          'issueDate': prop(p.issueDate, sens('passport.issueDate')),
          'placeOfIssue': prop(p.placeOfIssue, sens('passport.placeOfIssue')),
          'expiryDate': prop(p.expiryDate, sens('passport.expiryDate')),
          'holderName': prop(p.holderName, sens('passport.holderName')),
          'dateOfBirth': prop(p.dateOfBirth, sens('passport.dateOfBirth')),
          'placeOfBirth': prop(p.placeOfBirth, sens('passport.placeOfBirth')),
          'sex': prop(p.sex, sens('passport.sex')),
          'nationality': prop(p.nationality, sens('passport.nationality')),
          'authority': prop(p.authority, sens('passport.authority')),
        },
        isDeleted: p.isDeleted,
        deletedAt: p.deletedAt,
        createdAt: timestamp,
        updatedAt: p.updatedAt,
      ));
    }

    final visas = profile.travel?.visas ?? [];
    final visaChildren = <String>[];
    for (final v in visas) {
      visaChildren.add(v.id);
      objects.add(UnifiedObject(
        id: v.id,
        typeId: 'travel_visa',
        name: v.title ?? v.country ?? 'Visa',
        parentId: DefaultSectionIds.visa,
        properties: {
          'title': prop(v.title, sens('visa.title')),
          'country': prop(v.country, sens('visa.country')),
          'visaType': prop(v.visaType, sens('visa.visaType')),
          'number': prop(v.number, sens('visa.number')),
          'issueDate': prop(v.issueDate, sens('visa.issueDate')),
          'expiryDate': prop(v.expiryDate, sens('visa.expiryDate')),
        },
        isDeleted: v.isDeleted,
        deletedAt: v.deletedAt,
        createdAt: timestamp,
        updatedAt: v.updatedAt,
      ));
    }

    final histories = profile.travel?.travelHistory ?? [];
    final historyChildren = <String>[];
    for (final h in histories) {
      historyChildren.add(h.id);
      objects.add(UnifiedObject(
        id: h.id,
        typeId: 'travel_history',
        name: h.destination,
        parentId: DefaultSectionIds.travelHistory,
        properties: {
          'destination': prop(h.destination, sens('travel.destination')),
          'travelType': prop(h.travelType, sens('travel.travelType')),
          'date': prop(h.date, sens('travel.date')),
          'departureCity': prop(h.departureCity, sens('travel.departureCity')),
          'departureTime': prop(h.departureTime, sens('travel.departureTime')),
          'arrivalTime': prop(h.arrivalTime, sens('travel.arrivalTime')),
          'flightNumber': prop(h.flightNumber, sens('travel.flightNumber')),
          'ticketPrice': prop(h.ticketPrice, sens('travel.ticketPrice')),
          'airline': prop(h.airline, sens('travel.airline')),
        },
        isDeleted: h.isDeleted,
        deletedAt: h.deletedAt,
        createdAt: timestamp,
        updatedAt: h.updatedAt,
      ));
    }

    objects.add(UnifiedObject(
      id: DefaultSectionIds.passport,
      typeId: 'collection',
      name: 'Passports',
      iconName: 'book',
      parentId: DefaultPageIds.travel,
      childrenIds: passportChildren,
      createdAt: timestamp,
      updatedAt: timestamp,
    ));
    objects.add(UnifiedObject(
      id: DefaultSectionIds.visa,
      typeId: 'collection',
      name: 'Visas',
      iconName: 'assignment_ind',
      parentId: DefaultPageIds.travel,
      childrenIds: visaChildren,
      createdAt: timestamp,
      updatedAt: timestamp,
    ));
    objects.add(UnifiedObject(
      id: DefaultSectionIds.travelHistory,
      typeId: 'collection',
      name: 'Travel History',
      iconName: 'history',
      parentId: DefaultPageIds.travel,
      childrenIds: historyChildren,
      createdAt: timestamp,
      updatedAt: timestamp,
    ));
    objects.add(UnifiedObject(
      id: DefaultPageIds.travel,
      typeId: 'page',
      name: 'Travel',
      iconName: 'flight',
      parentId: null,
      childrenIds: const [
        DefaultSectionIds.passport,
        DefaultSectionIds.visa,
        DefaultSectionIds.travelHistory,
      ],
      createdAt: timestamp,
      updatedAt: timestamp,
    ));

    // -------------------------------------------------------------------------
    // Financial page
    // -------------------------------------------------------------------------
    final bankAccounts = profile.financial?.bankAccounts ?? [];
    final bankAccountChildren = <String>[];
    for (final b in bankAccounts) {
      bankAccountChildren.add(b.id);
      objects.add(UnifiedObject(
        id: b.id,
        typeId: 'financial_bank_account',
        name: b.title ?? b.bankName ?? 'Bank Account',
        parentId: DefaultSectionIds.bankAccount,
        properties: {
          'title': prop(b.title, sens('bankAccount.title')),
          'bankName': prop(b.bankName, sens('bankAccount.bankName')),
          'accountNumber': prop(b.accountNumber, sens('bankAccount.accountNumber')),
          'currency': prop(b.currency, sens('bankAccount.currency')),
          'swiftBic': prop(b.swiftBic, sens('bankAccount.swiftBic')),
          'sortCode': prop(b.sortCode, sens('bankAccount.sortCode')),
        },
        isDeleted: b.isDeleted,
        deletedAt: b.deletedAt,
        createdAt: timestamp,
        updatedAt: b.updatedAt,
      ));
    }

    final cards = profile.financial?.cards ?? [];
    final cardChildren = <String>[];
    for (final c in cards) {
      cardChildren.add(c.id);
      objects.add(UnifiedObject(
        id: c.id,
        typeId: 'financial_card',
        name: c.title ?? c.cardType ?? 'Card',
        parentId: DefaultSectionIds.card,
        properties: {
          'title': prop(c.title, sens('card.title')),
          'cardNumber': prop(c.cardNumber, sens('card.cardNumber')),
          'cardType': prop(c.cardType, sens('card.cardType')),
          'expiryDate': prop(c.expiryDate, sens('card.expiryDate')),
          'holderName': prop(c.holderName, sens('card.holderName')),
          'cvv': prop(c.cvv, sens('card.cvv')),
        },
        isDeleted: c.isDeleted,
        deletedAt: c.deletedAt,
        createdAt: timestamp,
        updatedAt: c.updatedAt,
      ));
    }

    final taxIds = profile.financial?.taxIds ?? [];
    final taxIdChildren = <String>[];
    for (final t in taxIds) {
      taxIdChildren.add(t.id);
      objects.add(UnifiedObject(
        id: t.id,
        typeId: 'financial_tax_id',
        name: t.title ?? 'Tax ID',
        parentId: DefaultSectionIds.taxId,
        properties: {
          'title': prop(t.title, sens('taxId.title')),
          'taxIdNumber': prop(t.taxIdNumber, sens('taxId.taxIdNumber')),
          'taxIdType': prop(t.taxIdType, sens('taxId.taxIdType')),
          'issuingAuthority': prop(t.issuingAuthority, sens('taxId.issuingAuthority')),
          'country': prop(t.country, sens('taxId.country')),
        },
        isDeleted: t.isDeleted,
        deletedAt: t.deletedAt,
        createdAt: timestamp,
        updatedAt: t.updatedAt,
      ));
    }

    objects.add(UnifiedObject(
      id: DefaultSectionIds.bankAccount,
      typeId: 'collection',
      name: 'Bank Accounts',
      iconName: 'account_balance',
      parentId: DefaultPageIds.financial,
      childrenIds: bankAccountChildren,
      createdAt: timestamp,
      updatedAt: timestamp,
    ));
    objects.add(UnifiedObject(
      id: DefaultSectionIds.card,
      typeId: 'collection',
      name: 'Cards',
      iconName: 'credit_card',
      parentId: DefaultPageIds.financial,
      childrenIds: cardChildren,
      createdAt: timestamp,
      updatedAt: timestamp,
    ));
    objects.add(UnifiedObject(
      id: DefaultSectionIds.taxId,
      typeId: 'collection',
      name: 'Tax IDs',
      iconName: 'description',
      parentId: DefaultPageIds.financial,
      childrenIds: taxIdChildren,
      createdAt: timestamp,
      updatedAt: timestamp,
    ));
    objects.add(UnifiedObject(
      id: DefaultPageIds.financial,
      typeId: 'page',
      name: 'Financial',
      iconName: 'account_balance',
      parentId: null,
      childrenIds: const [
        DefaultSectionIds.bankAccount,
        DefaultSectionIds.card,
        DefaultSectionIds.taxId,
      ],
      createdAt: timestamp,
      updatedAt: timestamp,
    ));

    // -------------------------------------------------------------------------
    // Professional page
    // -------------------------------------------------------------------------
    final educationList = profile.professional?.education ?? [];
    final educationChildren = <String>[];
    for (final e in educationList) {
      educationChildren.add(e.id);
      objects.add(UnifiedObject(
        id: e.id,
        typeId: 'professional_education',
        name: e.institution ?? 'Education',
        parentId: DefaultSectionIds.education,
        properties: {
          'institution': prop(e.institution, sens('education.institution')),
          'degree': prop(e.degree, sens('education.degree')),
          'degreeCustom': prop(e.degreeCustom, sens('education.degreeCustom')),
          'field': prop(e.field, sens('education.field')),
          'startDate': prop(e.startDate, sens('education.startDate')),
          'endDate': prop(e.endDate, sens('education.endDate')),
        },
        isDeleted: e.isDeleted,
        deletedAt: e.deletedAt,
        createdAt: timestamp,
        updatedAt: e.updatedAt,
      ));
    }

    final employmentList = profile.professional?.employment ?? [];
    final employmentChildren = <String>[];
    for (final e in employmentList) {
      employmentChildren.add(e.id);
      objects.add(UnifiedObject(
        id: e.id,
        typeId: 'professional_employment',
        name: e.company ?? 'Employment',
        parentId: DefaultSectionIds.employment,
        properties: {
          'company': prop(e.company, sens('employment.company')),
          'position': prop(e.position, sens('employment.position')),
          'responsibilities': prop(e.responsibilities, sens('employment.responsibilities')),
          'startDate': prop(e.startDate, sens('employment.startDate')),
          'endDate': prop(e.endDate, sens('employment.endDate')),
        },
        isDeleted: e.isDeleted,
        deletedAt: e.deletedAt,
        createdAt: timestamp,
        updatedAt: e.updatedAt,
      ));
    }

    final skills = profile.professional?.skills ?? [];
    final skillChildren = <String>[];
    for (final s in skills) {
      skillChildren.add(s.id);
      objects.add(UnifiedObject(
        id: s.id,
        typeId: 'professional_skill',
        name: s.name,
        parentId: DefaultSectionIds.skill,
        properties: {
          'name': prop(s.name, sens('skill.name')),
          'level': prop(s.level, sens('skill.level')),
        },
        isDeleted: s.isDeleted,
        deletedAt: s.deletedAt,
        createdAt: timestamp,
        updatedAt: s.updatedAt,
      ));
    }

    final languages = profile.professional?.languages ?? [];
    final languageChildren = <String>[];
    for (final l in languages) {
      languageChildren.add(l.id);
      objects.add(UnifiedObject(
        id: l.id,
        typeId: 'professional_language',
        name: l.name,
        parentId: DefaultSectionIds.language,
        properties: {
          'name': prop(l.name, sens('language.name')),
          'proficiency': prop(l.proficiency, sens('language.proficiency')),
        },
        isDeleted: l.isDeleted,
        deletedAt: l.deletedAt,
        createdAt: timestamp,
        updatedAt: l.updatedAt,
      ));
    }

    final awards = profile.professional?.awards ?? [];
    final awardChildren = <String>[];
    for (final a in awards) {
      awardChildren.add(a.id);
      objects.add(UnifiedObject(
        id: a.id,
        typeId: 'professional_award',
        name: a.title ?? 'Award',
        parentId: DefaultSectionIds.award,
        properties: {
          'title': prop(a.title, sens('award.title')),
          'issuer': prop(a.issuer, sens('award.issuer')),
          'date': prop(a.date, sens('award.date')),
          'description': prop(a.description, sens('award.description')),
        },
        isDeleted: a.isDeleted,
        deletedAt: a.deletedAt,
        createdAt: timestamp,
        updatedAt: a.updatedAt,
      ));
    }

    objects.add(UnifiedObject(
      id: DefaultSectionIds.education,
      typeId: 'collection',
      name: 'Education',
      iconName: 'school',
      parentId: DefaultPageIds.professional,
      childrenIds: educationChildren,
      createdAt: timestamp,
      updatedAt: timestamp,
    ));
    objects.add(UnifiedObject(
      id: DefaultSectionIds.employment,
      typeId: 'collection',
      name: 'Employment',
      iconName: 'work',
      parentId: DefaultPageIds.professional,
      childrenIds: employmentChildren,
      createdAt: timestamp,
      updatedAt: timestamp,
    ));
    objects.add(UnifiedObject(
      id: DefaultSectionIds.skill,
      typeId: 'collection',
      name: 'Skills',
      iconName: 'star',
      parentId: DefaultPageIds.professional,
      childrenIds: skillChildren,
      createdAt: timestamp,
      updatedAt: timestamp,
    ));
    objects.add(UnifiedObject(
      id: DefaultSectionIds.language,
      typeId: 'collection',
      name: 'Languages',
      iconName: 'language',
      parentId: DefaultPageIds.professional,
      childrenIds: languageChildren,
      createdAt: timestamp,
      updatedAt: timestamp,
    ));
    objects.add(UnifiedObject(
      id: DefaultSectionIds.award,
      typeId: 'collection',
      name: 'Awards',
      iconName: 'emoji_events',
      parentId: DefaultPageIds.professional,
      childrenIds: awardChildren,
      createdAt: timestamp,
      updatedAt: timestamp,
    ));
    objects.add(UnifiedObject(
      id: DefaultPageIds.professional,
      typeId: 'page',
      name: 'Professional',
      iconName: 'work',
      parentId: null,
      childrenIds: const [
        DefaultSectionIds.education,
        DefaultSectionIds.employment,
        DefaultSectionIds.skill,
        DefaultSectionIds.language,
        DefaultSectionIds.award,
      ],
      createdAt: timestamp,
      updatedAt: timestamp,
    ));

    return UnifiedObjectData(
      objects: objects,
      customTypes: existingData.customTypes,
    );
  }

  /// Load profile data for an account
  /// Returns ProfileData with all fields decrypted, or null if not found
  Future<ProfileData?> loadProfile(String accountId) async {
    try {
      // Try to load from Rust vault
      final decrypted = await _rustVault.loadProfileDecrypted(accountId);
      if (decrypted == null) {
        return null;
      }

      final (profile, needsSave, logs) = await Isolate.run(() {
        final json = jsonDecode(decrypted) as Map<String, dynamic>;
        final profile = ProfileData.fromJson(json);
        final migratedProfile = ProfileStorageService._migrateIfNeeded(profile, json);
        final (repairedProfile, wasRepaired) = ProfileStorageService._validateAndRepairProfile(migratedProfile);
        final logs = <String>[];
        if (wasRepaired) {
          logs.add('Data integrity repairs applied during load');
        }
        return (repairedProfile, wasRepaired, logs);
      });

      // Replay isolate logs on main thread
      for (final msg in logs) {
        DebugLogger.instance.logInfo('PROFILE', msg);
      }

      // Persist repairs so they don't need to be re-applied next load
      if (needsSave) {
        unawaited(
          saveProfile(accountId, profile).catchError((e) {
            DebugLogger.instance.logError(
              'PROFILE',
              'Failed to persist repaired profile: $e',
            );
            return false;
          }),
        );
      }
      return profile;
    } on RemoteError catch (e) {
      DebugLogger.instance.logError(
        'PROFILE',
        'Profile load failed in isolate: ${e.toString()}',
      );
      return null;
    } on Exception catch (e, st) {
      DebugLogger.instance.logError('PROFILE', 'loadProfile failed: $e\n$st');
      return null;
    }
  }

  /// Save profile data for an account
  /// Encrypts and stores via RustVaultService
  Future<bool> saveProfile(String accountId, ProfileData profile) async {
    try {
      // Data protection: prevent accidental loss of unifiedObjects
      final existing = await loadProfile(accountId);
      if (existing?.unifiedObjects != null && profile.unifiedObjects == null) {
        profile = profile.copyWith(unifiedObjects: existing!.unifiedObjects);
      }

      final json = await Isolate.run(() => jsonEncode(profile.toJson()));

      final result = await _rustVault.saveProfileEncrypted(accountId, json);

      if (result == null) {
        return false;
      }

      // Invalidate deleted items cache since profile data changed
      _invalidateDeletedItemsCache();

      return true;
    } on Exception catch (_) {
      // IOException or other Error subclasses could occur here
      return false;
    }
  }

  /// Get all soft-deleted items across all sections
  /// Results are cached to avoid rebuilding the list on every call
  /// Cache is invalidated on any profile mutation (restore, permanent delete, etc.)
  List<DeletedItemInfo> getDeletedItems(ProfileData profile) {
    if (_cachedDeletedItems != null) {
      return _cachedDeletedItems!;
    }

    final items = <DeletedItemInfo>[];

    if (profile.travel != null) {
      for (var i = 0; i < profile.travel!.passports.length; i++) {
        final p = profile.travel!.passports[i];
        if (p.isDeleted) {
          items.add(
            DeletedItemInfo(
              section: 'travel',
              itemType: 'passport',
              id: p.id,
              itemLabel: p.country ?? 'Passport',
              deletedAt: p.deletedAt ?? DateTime.now(),
            ),
          );
        }
      }
    }

    // Visa loop (separate from passport loop above)
    if (profile.travel != null) {
      for (var i = 0; i < profile.travel!.visas.length; i++) {
        final v = profile.travel!.visas[i];
        if (v.isDeleted) {
          items.add(
            DeletedItemInfo(
              section: 'travel',
              itemType: 'visa',
              id: v.id,
              itemLabel: v.country ?? 'Visa',
              deletedAt: v.deletedAt ?? DateTime.now(),
            ),
          );
        }
      }
      for (var i = 0; i < profile.travel!.travelHistory.length; i++) {
        final t = profile.travel!.travelHistory[i];
        if (t.isDeleted) {
          items.add(
            DeletedItemInfo(
              section: 'travel',
              itemType: 'travel_history',
              id: t.id,
              itemLabel: t.destination,
              deletedAt: t.deletedAt ?? DateTime.now(),
            ),
          );
        }
      }
    }

    // Financial section
    if (profile.financial != null) {
      for (var i = 0; i < profile.financial!.bankAccounts.length; i++) {
        final b = profile.financial!.bankAccounts[i];
        if (b.isDeleted) {
          items.add(
            DeletedItemInfo(
              section: 'financial',
              itemType: 'bank_account',
              id: b.id,
              itemLabel: b.bankName ?? 'Bank Account',
              deletedAt: b.deletedAt ?? DateTime.now(),
            ),
          );
        }
      }
      for (var i = 0; i < profile.financial!.cards.length; i++) {
        final c = profile.financial!.cards[i];
        if (c.isDeleted) {
          items.add(
            DeletedItemInfo(
              section: 'financial',
              itemType: 'card',
              id: c.id,
              itemLabel: c.cardType ?? 'Card',
              deletedAt: c.deletedAt ?? DateTime.now(),
            ),
          );
        }
      }
      for (var i = 0; i < profile.financial!.taxIds.length; i++) {
        final t = profile.financial!.taxIds[i];
        if (t.isDeleted) {
          items.add(
            DeletedItemInfo(
              section: 'financial',
              itemType: 'tax_id',
              id: t.id,
              itemLabel: t.taxIdType ?? 'Tax ID',
              deletedAt: t.deletedAt ?? DateTime.now(),
            ),
          );
        }
      }
    }

    // Professional section
    if (profile.professional != null) {
      for (var i = 0; i < profile.professional!.education.length; i++) {
        final e = profile.professional!.education[i];
        if (e.isDeleted) {
          items.add(
            DeletedItemInfo(
              section: 'professional',
              itemType: 'education',
              id: e.id,
              itemLabel: e.institution ?? 'Education',
              deletedAt: e.deletedAt ?? DateTime.now(),
            ),
          );
        }
      }
      for (var i = 0; i < profile.professional!.employment.length; i++) {
        final emp = profile.professional!.employment[i];
        if (emp.isDeleted) {
          items.add(
            DeletedItemInfo(
              section: 'professional',
              itemType: 'employment',
              id: emp.id,
              itemLabel: emp.company ?? 'Employment',
              deletedAt: emp.deletedAt ?? DateTime.now(),
            ),
          );
        }
      }
      for (var i = 0; i < profile.professional!.skills.length; i++) {
        final s = profile.professional!.skills[i];
        if (s.isDeleted) {
          items.add(
            DeletedItemInfo(
              section: 'professional',
              itemType: 'skill',
              id: s.id,
              itemLabel: s.toString(),
              deletedAt: s.deletedAt ?? DateTime.now(),
            ),
          );
        }
      }
      for (var i = 0; i < profile.professional!.languages.length; i++) {
        final l = profile.professional!.languages[i];
        if (l.isDeleted) {
          items.add(
            DeletedItemInfo(
              section: 'professional',
              itemType: 'language',
              id: l.id,
              itemLabel: l.toString(),
              deletedAt: l.deletedAt ?? DateTime.now(),
            ),
          );
        }
      }
      for (var i = 0; i < profile.professional!.awards.length; i++) {
        final a = profile.professional!.awards[i];
        if (a.isDeleted) {
          items.add(
            DeletedItemInfo(
              section: 'professional',
              itemType: 'award',
              id: a.id,
              itemLabel: a.title ?? 'Award',
              deletedAt: a.deletedAt ?? DateTime.now(),
            ),
          );
        }
      }
    }

    // Profile/Identity section - contact entries
    if (profile.identity?.contact != null) {
      for (var i = 0; i < profile.identity!.contact!.entries.length; i++) {
        final e = profile.identity!.contact!.entries[i];
        if (e.isDeleted) {
          items.add(
            DeletedItemInfo(
              section: 'profile',
              itemType: 'contact',
              id: e.id,
              itemLabel: e.title.isNotEmpty
                  ? '${e.title} - ${e.value}'
                  : e.value,
              deletedAt: e.deletedAt ?? DateTime.now(),
            ),
          );
        }
      }
    }

    // Profile/Identity section - ID cards
    if (profile.identity?.idCards != null) {
      for (var i = 0; i < profile.identity!.idCards!.length; i++) {
        final c = profile.identity!.idCards![i];
        if (c.isDeleted) {
          items.add(
            DeletedItemInfo(
              section: 'profile',
              itemType: 'idCard',
              id: c.id,
              itemLabel: c.title ?? c.number ?? 'ID Card',
              deletedAt: c.deletedAt ?? DateTime.now(),
            ),
          );
        }
      }
    }

    // Profile/Identity section - addresses
    if (profile.identity?.addresses != null) {
      for (var i = 0; i < profile.identity!.addresses!.length; i++) {
        final a = profile.identity!.addresses![i];
        if (a.isDeleted) {
          items.add(
            DeletedItemInfo(
              section: 'profile',
              itemType: 'address',
              id: a.id,
              itemLabel: a.title ?? 'Address',
              deletedAt: a.deletedAt ?? DateTime.now(),
            ),
          );
        }
      }
    }

    // Sort by deletedAt descending (most recent first)
    items.sort((a, b) => b.deletedAt.compareTo(a.deletedAt));

    // Cache the result
    _cachedDeletedItems = items;
    return items;
  }

  /// Restore a soft-deleted item
  Future<void> restoreItem(
    ProfileData profile,
    String accountId,
    String section,
    String itemType,
    int index,
  ) async {
    _invalidateDeletedItemsCache();
    final updatedProfile = _calculateRestoreItem(
      profile,
      section,
      itemType,
      index,
    );
    await saveProfile(accountId, updatedProfile);
  }

  /// Pure function: calculates a new ProfileData with the restored item.
  /// Does not mutate the input profile.
  static ProfileData _calculateRestoreItem(
    ProfileData profile,
    String section,
    String itemType,
    int index,
  ) {
    switch (section) {
      case 'travel':
        if (profile.travel == null) return profile;
        if (itemType == 'passport' &&
            index < profile.travel!.passports.length) {
          final passports = List<PassportData>.from(profile.travel!.passports);
          passports[index] = passports[index].copyWith(
            isDeleted: false,
            deletedAt: null,
          );
          return profile.copyWith(
            travel: profile.travel!.copyWith(passports: passports),
          );
        } else if (itemType == 'visa' && index < profile.travel!.visas.length) {
          final visas = List<VisaData>.from(profile.travel!.visas);
          visas[index] = visas[index].copyWith(
            isDeleted: false,
            deletedAt: null,
          );
          return profile.copyWith(
            travel: profile.travel!.copyWith(visas: visas),
          );
        }
        return profile;
      case 'financial':
        if (profile.financial == null) return profile;
        if (itemType == 'bank_account' &&
            index < profile.financial!.bankAccounts.length) {
          final bankAccounts = List<BankAccountData>.from(
            profile.financial!.bankAccounts,
          );
          bankAccounts[index] = bankAccounts[index].copyWith(
            isDeleted: false,
            deletedAt: null,
          );
          return profile.copyWith(
            financial: profile.financial!.copyWith(bankAccounts: bankAccounts),
          );
        } else if (itemType == 'card' &&
            index < profile.financial!.cards.length) {
          final cards = List<CardData>.from(profile.financial!.cards);
          cards[index] = cards[index].copyWith(
            isDeleted: false,
            deletedAt: null,
          );
          return profile.copyWith(
            financial: profile.financial!.copyWith(cards: cards),
          );
        } else if (itemType == 'tax_id' &&
            index < profile.financial!.taxIds.length) {
          final taxIds = List<TaxIdData>.from(profile.financial!.taxIds);
          taxIds[index] = taxIds[index].copyWith(
            isDeleted: false,
            deletedAt: null,
          );
          return profile.copyWith(
            financial: profile.financial!.copyWith(taxIds: taxIds),
          );
        }
        return profile;
      case 'professional':
        if (profile.professional == null) return profile;
        if (itemType == 'education' &&
            index < profile.professional!.education.length) {
          final education = List<EducationData>.from(
            profile.professional!.education,
          );
          education[index] = education[index].copyWith(
            isDeleted: false,
            deletedAt: null,
          );
          return profile.copyWith(
            professional: profile.professional!.copyWith(education: education),
          );
        } else if (itemType == 'employment' &&
            index < profile.professional!.employment.length) {
          final employment = List<EmploymentData>.from(
            profile.professional!.employment,
          );
          employment[index] = employment[index].copyWith(
            isDeleted: false,
            deletedAt: null,
          );
          return profile.copyWith(
            professional: profile.professional!.copyWith(
              employment: employment,
            ),
          );
        } else if (itemType == 'skill' &&
            index < profile.professional!.skills.length) {
          final skills = List<SkillData>.from(profile.professional!.skills);
          skills[index] = skills[index].copyWith(
            isDeleted: false,
            deletedAt: null,
          );
          return profile.copyWith(
            professional: profile.professional!.copyWith(skills: skills),
          );
        } else if (itemType == 'language' &&
            index < profile.professional!.languages.length) {
          final languages = List<LanguageData>.from(
            profile.professional!.languages,
          );
          languages[index] = languages[index].copyWith(
            isDeleted: false,
            deletedAt: null,
          );
          return profile.copyWith(
            professional: profile.professional!.copyWith(languages: languages),
          );
        }
        return profile;
      case 'profile':
        if (profile.identity == null) return profile;
        if (itemType == 'contact' &&
            index < (profile.identity!.contact?.entries.length ?? 0)) {
          final entries = List<ContactEntry>.from(
            profile.identity!.contact!.entries,
          );
          entries[index] = entries[index].copyWith(
            isDeleted: false,
            deletedAt: null,
          );
          return profile.copyWith(
            identity: profile.identity!.copyWith(
              contact: ContactData(entries: entries),
            ),
          );
        } else if (itemType == 'idCard' &&
            index < (profile.identity!.idCards?.length ?? 0)) {
          final idCards = List<IdCardData>.from(profile.identity!.idCards!);
          idCards[index] = idCards[index].copyWith(
            isDeleted: false,
            deletedAt: null,
          );
          return profile.copyWith(
            identity: profile.identity!.copyWith(idCards: idCards),
          );
        } else if (itemType == 'address' &&
            index < (profile.identity!.addresses?.length ?? 0)) {
          final addresses = List<AddressData>.from(
            profile.identity!.addresses!,
          );
          addresses[index] = addresses[index].copyWith(
            isDeleted: false,
            deletedAt: null,
          );
          return profile.copyWith(
            identity: profile.identity!.copyWith(addresses: addresses),
          );
        }
        return profile;
    }
    return profile;
  }

  /// Calculate the result of permanently deleting an item (pure function).
  /// Returns a new ProfileData with the item removed, or null if the item
  /// could not be deleted (e.g., invalid index or null section).
  static ProfileData? _calculatePermanentDeleteItem(
    ProfileData profile,
    String section,
    String itemType,
    int index,
  ) {
    switch (section) {
      case 'travel':
        if (profile.travel == null) return null;
        if (itemType == 'passport' &&
            index < profile.travel!.passports.length) {
          final updated = List<PassportData>.from(profile.travel!.passports)
            ..removeAt(index);
          return profile.copyWith(
            travel: profile.travel!.copyWith(passports: updated),
          );
        } else if (itemType == 'visa' && index < profile.travel!.visas.length) {
          final updated = List<VisaData>.from(profile.travel!.visas)
            ..removeAt(index);
          return profile.copyWith(
            travel: profile.travel!.copyWith(visas: updated),
          );
        } else if (itemType == 'travel_history' &&
            index < profile.travel!.travelHistory.length) {
          final updated = List<TravelHistoryData>.from(
            profile.travel!.travelHistory,
          )..removeAt(index);
          return profile.copyWith(
            travel: profile.travel!.copyWith(travelHistory: updated),
          );
        }
        return null;
      case 'financial':
        if (profile.financial == null) return null;
        if (itemType == 'bank_account' &&
            index < profile.financial!.bankAccounts.length) {
          final updated = List<BankAccountData>.from(
            profile.financial!.bankAccounts,
          )..removeAt(index);
          return profile.copyWith(
            financial: profile.financial!.copyWith(bankAccounts: updated),
          );
        } else if (itemType == 'card' &&
            index < profile.financial!.cards.length) {
          final updated = List<CardData>.from(profile.financial!.cards)
            ..removeAt(index);
          return profile.copyWith(
            financial: profile.financial!.copyWith(cards: updated),
          );
        } else if (itemType == 'tax_id' &&
            index < profile.financial!.taxIds.length) {
          final updated = List<TaxIdData>.from(profile.financial!.taxIds)
            ..removeAt(index);
          return profile.copyWith(
            financial: profile.financial!.copyWith(taxIds: updated),
          );
        }
        return null;
      case 'professional':
        if (profile.professional == null) return null;
        if (itemType == 'education' &&
            index < profile.professional!.education.length) {
          final updated = List<EducationData>.from(
            profile.professional!.education,
          )..removeAt(index);
          return profile.copyWith(
            professional: profile.professional!.copyWith(education: updated),
          );
        } else if (itemType == 'employment' &&
            index < profile.professional!.employment.length) {
          final updated = List<EmploymentData>.from(
            profile.professional!.employment,
          )..removeAt(index);
          return profile.copyWith(
            professional: profile.professional!.copyWith(employment: updated),
          );
        } else if (itemType == 'skill' &&
            index < profile.professional!.skills.length) {
          final updated = List<SkillData>.from(profile.professional!.skills)
            ..removeAt(index);
          return profile.copyWith(
            professional: profile.professional!.copyWith(skills: updated),
          );
        } else if (itemType == 'language' &&
            index < profile.professional!.languages.length) {
          final updated = List<LanguageData>.from(
            profile.professional!.languages,
          )..removeAt(index);
          return profile.copyWith(
            professional: profile.professional!.copyWith(languages: updated),
          );
        }
        return null;
      case 'profile':
        if (profile.identity == null) return null;
        if (itemType == 'contact' &&
            index < (profile.identity!.contact?.entries.length ?? 0)) {
          final entries = List<ContactEntry>.from(
            profile.identity!.contact!.entries,
          )..removeAt(index);
          return profile.copyWith(
            identity: profile.identity!.copyWith(
              contact: ContactData(entries: entries),
            ),
          );
        } else if (itemType == 'idCard' &&
            index < (profile.identity!.idCards?.length ?? 0)) {
          final idCards = List<IdCardData>.from(profile.identity!.idCards!)
            ..removeAt(index);
          return profile.copyWith(
            identity: profile.identity!.copyWith(idCards: idCards),
          );
        } else if (itemType == 'address' &&
            index < (profile.identity!.addresses?.length ?? 0)) {
          final addresses = List<AddressData>.from(profile.identity!.addresses!)
            ..removeAt(index);
          return profile.copyWith(
            identity: profile.identity!.copyWith(addresses: addresses),
          );
        }
        return null;
      default:
        return null;
    }
  }

  /// Permanently delete a specific item (removes from list completely)
  Future<void> permanentDeleteItem(
    ProfileData profile,
    String accountId,
    String section,
    String itemType,
    int index,
  ) async {
    _invalidateDeletedItemsCache();
    final updatedProfile = _calculatePermanentDeleteItem(
      profile,
      section,
      itemType,
      index,
    );
    if (updatedProfile == null) return;
    await saveProfile(accountId, updatedProfile);
  }

  /// Permanently delete items older than 30 days.
  /// Returns a new [ProfileData] with old deleted items removed (immutable).
  ProfileData purgeOldDeletedItems(ProfileData profile) {
    final cutoff = DateTime.now().subtract(const Duration(days: 30));
    bool isOld(dynamic item) =>
        item.isDeleted && item.deletedAt != null && item.deletedAt!.isBefore(cutoff);

    return profile.copyWith(
      travel: profile.travel?.copyWith(
        passports: profile.travel!.passports.where((p) => !isOld(p)).toList(),
        visas: profile.travel!.visas.where((v) => !isOld(v)).toList(),
        travelHistory: profile.travel!.travelHistory.where((t) => !isOld(t)).toList(),
      ),
      financial: profile.financial?.copyWith(
        bankAccounts: profile.financial!.bankAccounts.where((b) => !isOld(b)).toList(),
        cards: profile.financial!.cards.where((c) => !isOld(c)).toList(),
        taxIds: profile.financial!.taxIds.where((t) => !isOld(t)).toList(),
      ),
      professional: profile.professional?.copyWith(
        education: profile.professional!.education.where((e) => !isOld(e)).toList(),
        employment: profile.professional!.employment.where((emp) => !isOld(emp)).toList(),
        skills: profile.professional!.skills.where((s) => !isOld(s)).toList(),
        languages: profile.professional!.languages.where((l) => !isOld(l)).toList(),
        awards: profile.professional!.awards.where((a) => !isOld(a)).toList(),
      ),
      identity: profile.identity?.copyWith(
        idCards: profile.identity!.idCards?.where((c) => !isOld(c)).toList(),
        addresses: profile.identity!.addresses?.where((a) => !isOld(a)).toList(),
        contact: profile.identity!.contact?.copyWith(
          entries: profile.identity!.contact!.entries.where((e) => !isOld(e)).toList(),
        ),
      ),
      unifiedObjects: profile.unifiedObjects?.copyWith(
        objects: profile.unifiedObjects!.objects.where((o) => !isOld(o)).toList(),
      ),
    );
  }

  /// Check and purge old deleted items (called on app startup)
  ///
  /// If [existingProfile] is provided (already loaded), uses it instead of
  /// loading again to avoid redundant decryption.
  Future<void> purgeOldDeletedItemsIfNeeded(
    String accountId, {
    ProfileData? existingProfile,
  }) async {
    final profile = existingProfile ?? await loadProfile(accountId);
    if (profile == null) return;

    final cutoff = DateTime.now().subtract(const Duration(days: 30));
    bool hasOldItems = false;

    // Check if any deleted items are older than 30 days
    if (profile.travel != null) {
      hasOldItems =
          hasOldItems ||
          profile.travel!.passports.any(
            (p) =>
                p.isDeleted &&
                p.deletedAt != null &&
                p.deletedAt!.isBefore(cutoff),
          ) ||
          profile.travel!.visas.any(
            (v) =>
                v.isDeleted &&
                v.deletedAt != null &&
                v.deletedAt!.isBefore(cutoff),
          ) ||
          profile.travel!.travelHistory.any(
            (t) =>
                t.isDeleted &&
                t.deletedAt != null &&
                t.deletedAt!.isBefore(cutoff),
          );
    }
    if (profile.financial != null) {
      hasOldItems =
          hasOldItems ||
          profile.financial!.bankAccounts.any(
            (b) =>
                b.isDeleted &&
                b.deletedAt != null &&
                b.deletedAt!.isBefore(cutoff),
          ) ||
          profile.financial!.cards.any(
            (c) =>
                c.isDeleted &&
                c.deletedAt != null &&
                c.deletedAt!.isBefore(cutoff),
          ) ||
          profile.financial!.taxIds.any(
            (t) =>
                t.isDeleted &&
                t.deletedAt != null &&
                t.deletedAt!.isBefore(cutoff),
          );
    }
    if (profile.professional != null) {
      hasOldItems =
          hasOldItems ||
          profile.professional!.education.any(
            (e) =>
                e.isDeleted &&
                e.deletedAt != null &&
                e.deletedAt!.isBefore(cutoff),
          ) ||
          profile.professional!.employment.any(
            (emp) =>
                emp.isDeleted &&
                emp.deletedAt != null &&
                emp.deletedAt!.isBefore(cutoff),
          ) ||
          profile.professional!.skills.any(
            (s) =>
                s.isDeleted &&
                s.deletedAt != null &&
                s.deletedAt!.isBefore(cutoff),
          ) ||
          profile.professional!.languages.any(
            (l) =>
                l.isDeleted &&
                l.deletedAt != null &&
                l.deletedAt!.isBefore(cutoff),
          ) ||
          profile.professional!.awards.any(
            (a) =>
                a.isDeleted &&
                a.deletedAt != null &&
                a.deletedAt!.isBefore(cutoff),
          );
    }
    if (profile.identity != null) {
      hasOldItems =
          hasOldItems ||
          (profile.identity!.idCards?.any(
                (c) =>
                    c.isDeleted &&
                    c.deletedAt != null &&
                    c.deletedAt!.isBefore(cutoff),
              ) ??
              false) ||
          (profile.identity!.addresses?.any(
                (a) =>
                    a.isDeleted &&
                    a.deletedAt != null &&
                    a.deletedAt!.isBefore(cutoff),
              ) ??
              false) ||
          (profile.identity!.contact?.entries.any(
                (e) =>
                    e.isDeleted &&
                    e.deletedAt != null &&
                    e.deletedAt!.isBefore(cutoff),
              ) ??
              false);
    }
    if (profile.unifiedObjects != null) {
      hasOldItems =
          hasOldItems ||
          profile.unifiedObjects!.objects.any(
            (o) =>
                o.isDeleted &&
                o.deletedAt != null &&
                o.deletedAt!.isBefore(cutoff),
          );
    }

    if (hasOldItems) {
      final newProfile = purgeOldDeletedItems(profile);
      await saveProfile(accountId, newProfile);
    }
  }

  /// Manually empty all trash (permanent delete all soft-deleted items)
  Future<void> emptyAllTrash(ProfileData profile, String accountId) async {
    final newProfile = calculateEmptyTrash(profile);
    await saveProfile(accountId, newProfile);
  }

  /// Pure function: returns a new ProfileData with all soft-deleted items removed
  static ProfileData calculateEmptyTrash(ProfileData current) {
    // Travel section
    final newTravel = current.travel?.copyWith(
      passports: current.travel!.passports.where((p) => !p.isDeleted).toList(),
      visas: current.travel!.visas.where((v) => !v.isDeleted).toList(),
      travelHistory: current.travel!.travelHistory.where((t) => !t.isDeleted).toList(),
    );

    // Financial section
    final newFinancial = current.financial?.copyWith(
      bankAccounts: current.financial!.bankAccounts.where((b) => !b.isDeleted).toList(),
      cards: current.financial!.cards.where((c) => !c.isDeleted).toList(),
      taxIds: current.financial!.taxIds.where((t) => !t.isDeleted).toList(),
    );

    // Professional section
    final newProfessional = current.professional?.copyWith(
      education: current.professional!.education.where((e) => !e.isDeleted).toList(),
      employment: current.professional!.employment.where((emp) => !emp.isDeleted).toList(),
      skills: current.professional!.skills.where((s) => !s.isDeleted).toList(),
      languages: current.professional!.languages.where((l) => !l.isDeleted).toList(),
      awards: current.professional!.awards.where((a) => !a.isDeleted).toList(),
    );

    // Identity section
    final newIdentity = current.identity?.copyWith(
      idCards: current.identity!.idCards?.where((c) => !c.isDeleted).toList(),
      addresses: current.identity!.addresses?.where((a) => !a.isDeleted).toList(),
      contact: current.identity!.contact?.copyWith(
        entries: current.identity!.contact!.entries.where((e) => !e.isDeleted).toList(),
      ),
    );

    // UnifiedObject section
    final newUnifiedObjects = current.unifiedObjects?.copyWith(
      objects: current.unifiedObjects!.objects.where((o) => !o.isDeleted).toList(),
    );

    return current.copyWith(
      travel: newTravel,
      financial: newFinancial,
      professional: newProfessional,
      identity: newIdentity,
      unifiedObjects: newUnifiedObjects,
    );
  }
}
