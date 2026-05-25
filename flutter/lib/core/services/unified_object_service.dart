import 'package:flutter/material.dart';
import 'package:uuid/uuid.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/models/sensitivity_models.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';

const _uuid = Uuid();

/// Generate a new unique ID using UUID v4
String _generateId() => _uuid.v4();

/// Returns current timestamp in milliseconds since epoch
int _currentTimestamp() => DateTime.now().millisecondsSinceEpoch;

// =============================================================================
// Built-in Object Type Definitions
// =============================================================================

/// Registry of built-in object types shipped with the app.
/// User-defined types are stored in [UnifiedObjectData.customTypes].
class ObjectTypeRegistry {
  static final Map<String, ObjectTypeDefinition> _builtins = {
    for (final t in _kBuiltinTypes) t.id: t,
  };

  /// Get a type definition by ID. Checks custom types first (they override built-ins).
  static ObjectTypeDefinition? getType(
    String id, {
    List<ObjectTypeDefinition> customTypes = const [],
  }) {
    // Custom types take precedence over built-ins (allows overriding built-in types)
    try {
      return customTypes.firstWhere((t) => t.id == id);
    } on Object {
      // Fall back to built-in if no custom type with this id
      return _builtins[id];
    }
  }

  /// Get all available type definitions (built-in + custom).
  static List<ObjectTypeDefinition> getAllTypes({
    List<ObjectTypeDefinition> customTypes = const [],
  }) {
    return [..._builtins.values, ...customTypes];
  }

  /// Default type for generic objects.
  static ObjectTypeDefinition get defaultType => _builtins['note']!;

  /// Build a property-value map from a built-in type definition.
  /// Used to copy builtin schema into a section's properties.
  static Map<String, PropertyValue> buildPropertiesFromType(
    String typeId, {
    List<ObjectTypeDefinition> customTypes = const [],
  }) {
    final type = getType(typeId, customTypes: customTypes);
    if (type == null) return {};

    final prefix = fieldPrefixForTypeId(typeId);
    final result = <String, PropertyValue>{};
    for (final prop in type.properties) {
      final sensitivity = lookupFieldSensitivity('$prefix.${prop.id}');
      result[prop.id] = emptyPropertyValueForType(prop.type, sensitivity);
    }
    return result;
  }

  /// Build display labels map from a built-in type definition.
  /// Returns {propertyKey: displayLabel} for properties that have a non-empty name.
  static Map<String, String> buildPropertyLabelsFromType(
    String typeId, {
    List<ObjectTypeDefinition> customTypes = const [],
  }) {
    final type = getType(typeId, customTypes: customTypes);
    if (type == null) return {};

    final result = <String, String>{};
    for (final prop in type.properties) {
      if (prop.name.isNotEmpty && prop.name != prop.id) {
        result[prop.id] = prop.name;
      }
    }
    return result;
  }
}

// =============================================================================
// Default Page / Section Fixed IDs
// =============================================================================

/// Fixed IDs for default pages (Profile, Travel, Financial, Professional).
/// These are used to identify the built-in pages in the unified object tree.
class DefaultPageIds {
  static const profile = '__page_profile';
  static const travel = '__page_travel';
  static const financial = '__page_financial';
  static const professional = '__page_professional';
}

/// Fixed IDs for default sections under each page.
class DefaultSectionIds {
  // Profile page sections
  static const identity = '__section_identity';
  static const contact = '__section_contact';
  static const idCard = '__section_id_card';
  static const address = '__section_address';

  // Travel page sections
  static const passport = '__section_passport';
  static const visa = '__section_visa';
  static const travelHistory = '__section_travel_history';

  // Financial page sections
  static const bankAccount = '__section_bank_account';
  static const card = '__section_card';
  static const taxId = '__section_tax_id';

  // Professional page sections
  static const education = '__section_education';
  static const employment = '__section_employment';
  static const skill = '__section_skill';
  static const language = '__section_language';
  static const award = '__section_award';
  static const article = '__section_article';
}

/// Metadata for auto-creating a default section when it is missing.
class SectionMeta {
  final String name;
  final String iconName;
  final String parentPageId;
  const SectionMeta(this.name, this.iconName, this.parentPageId);
}

const Map<String, SectionMeta> _kSectionMeta = {
  DefaultSectionIds.identity: SectionMeta('Identity', 'person', DefaultPageIds.profile),
  DefaultSectionIds.contact: SectionMeta('Contact Information', 'contact_mail', DefaultPageIds.profile),
  DefaultSectionIds.idCard: SectionMeta('ID Cards', 'badge', DefaultPageIds.profile),
  DefaultSectionIds.address: SectionMeta('Addresses', 'home', DefaultPageIds.profile),
  DefaultSectionIds.passport: SectionMeta('Passports', 'flight', DefaultPageIds.travel),
  DefaultSectionIds.visa: SectionMeta('Visas', 'description', DefaultPageIds.travel),
  DefaultSectionIds.travelHistory: SectionMeta('Travel History', 'history', DefaultPageIds.travel),
  DefaultSectionIds.bankAccount: SectionMeta('Bank Accounts', 'account_balance', DefaultPageIds.financial),
  DefaultSectionIds.card: SectionMeta('Cards', 'credit_card', DefaultPageIds.financial),
  DefaultSectionIds.taxId: SectionMeta('Tax IDs', 'receipt', DefaultPageIds.financial),
  DefaultSectionIds.education: SectionMeta('Education', 'school', DefaultPageIds.professional),
  DefaultSectionIds.employment: SectionMeta('Employment', 'work', DefaultPageIds.professional),
  DefaultSectionIds.skill: SectionMeta('Skills', 'stars', DefaultPageIds.professional),
  DefaultSectionIds.language: SectionMeta('Languages', 'language', DefaultPageIds.professional),
  DefaultSectionIds.award: SectionMeta('Awards', 'emoji_events', DefaultPageIds.professional),
  DefaultSectionIds.article: SectionMeta('Articles', 'article', DefaultPageIds.professional),
};

/// 根据 sectionId 获取其元数据，用于缺失时自动创建。
SectionMeta? getSectionMeta(String sectionId) => _kSectionMeta[sectionId];

/// 遍历所有默认 section 的元数据。
Iterable<MapEntry<String, SectionMeta>> get allSectionMeta => _kSectionMeta.entries;

/// 根据 pageId 获取该页面下的所有默认 section ID 列表。
List<String> getDefaultSectionIdsForPage(String pageId) {
  return _kSectionMeta.entries
      .where((e) => e.value.parentPageId == pageId)
      .map((e) => e.key)
      .toList();
}

/// Mapping from section ID to its item type ID.
/// Prefixes avoid collisions with generic built-in types (e.g. 'contact').
const Map<String, String> _kSectionItemTypes = {
  DefaultSectionIds.identity: 'profile_identity',
  DefaultSectionIds.contact: 'profile_contact',
  DefaultSectionIds.idCard: 'profile_id_card',
  DefaultSectionIds.address: 'profile_address',
  DefaultSectionIds.passport: 'travel_passport',
  DefaultSectionIds.visa: 'travel_visa',
  DefaultSectionIds.travelHistory: 'travel_history',
  DefaultSectionIds.bankAccount: 'financial_bank_account',
  DefaultSectionIds.card: 'financial_card',
  DefaultSectionIds.taxId: 'financial_tax_id',
  DefaultSectionIds.education: 'professional_education',
  DefaultSectionIds.employment: 'professional_employment',
  DefaultSectionIds.skill: 'professional_skill',
  DefaultSectionIds.language: 'professional_language',
  DefaultSectionIds.award: 'professional_award',
  DefaultSectionIds.article: 'professional_article',
};

/// 根据 item type ID 反向查找对应的默认 section ID。
String? getDefaultSectionIdForItemType(String itemTypeId) {
  for (final entry in _kSectionItemTypes.entries) {
    if (entry.value == itemTypeId) return entry.key;
  }
  return null;
}

/// 根据 section ID 获取对应的 item type ID。
String? getItemTypeIdForSection(String sectionId) => _kSectionItemTypes[sectionId];

/// Map typeId to the field-prefix used by FieldRegistry.
String fieldPrefixForTypeId(String typeId) {
  return switch (typeId) {
    'profile_identity' => 'identity',
    'profile_contact' => 'contact',
    'profile_id_card' => 'idCard',
    'profile_address' => 'address',
    'travel_passport' => 'passport',
    'travel_visa' => 'visa',
    'travel_history' => 'travel',
    'financial_bank_account' => 'bankAccount',
    'financial_card' => 'card',
    'financial_tax_id' => 'taxId',
    'professional_education' => 'education',
    'professional_employment' => 'employment',
    'professional_skill' => 'skill',
    'professional_language' => 'language',
    'professional_award' => 'award',
    'professional_article' => 'article',
    _ => typeId,
  };
}

/// Look up sensitivity from FieldRegistry defaults.
SensitivityLevel lookupFieldSensitivity(String fieldId) {
  try {
    return FieldRegistry.defaultFields
        .firstWhere((f) => f.fieldId == fieldId)
        .level;
  } on Object catch (_) {
    return SensitivityLevel.public;
  }
}

/// 根据 PropertyType 创建对应的空 PropertyValue。
PropertyValue emptyPropertyValueForType(PropertyType type, SensitivityLevel sensitivity) {
  return switch (type) {
    PropertyType.text => TextProperty(text: '', sensitivity: sensitivity),
    PropertyType.number => NumberProperty(value: null, sensitivity: sensitivity),
    PropertyType.date => DateProperty(isoDate: null, sensitivity: sensitivity),
    PropertyType.checkbox => CheckboxProperty(checked: false, sensitivity: sensitivity),
    PropertyType.select => SelectProperty(options: [], selectedId: null, sensitivity: sensitivity),
    PropertyType.multiSelect => MultiSelectProperty(options: [], selectedIds: [], sensitivity: sensitivity),
    PropertyType.relation => RelationProperty(targetObjectId: null, sensitivity: sensitivity),
    PropertyType.url => UrlProperty(url: null, sensitivity: sensitivity),
  };
}

// =============================================================================
// Built-in Object Type Definitions
// =============================================================================

/// Built-in type definitions.
final List<ObjectTypeDefinition> _kBuiltinTypes = [
  // ---------------------------------------------------------------------------
  // Generic types (existing)
  // ---------------------------------------------------------------------------
  const ObjectTypeDefinition(
    id: 'page',
    name: 'Page',
    iconName: 'article',
    defaultLayout: ObjectLayout.document,
  ),
  const ObjectTypeDefinition(
    id: 'collection',
    name: 'Collection',
    iconName: 'folder',
    defaultLayout: ObjectLayout.collection,
  ),
  const ObjectTypeDefinition(
    id: 'note',
    name: 'Note',
    iconName: 'note',
    defaultLayout: ObjectLayout.document,
    properties: [
      PropertyDefinition(
        id: 'content',
        name: 'Content',
        type: PropertyType.text,
      ),
    ],
  ),
  const ObjectTypeDefinition(
    id: 'task',
    name: 'Task',
    iconName: 'check_circle',
    defaultLayout: ObjectLayout.document,
    properties: [
      PropertyDefinition(
        id: 'done',
        name: 'Done',
        type: PropertyType.checkbox,
      ),
      PropertyDefinition(
        id: 'dueDate',
        name: 'Due Date',
        type: PropertyType.date,
      ),
    ],
  ),
  const ObjectTypeDefinition(
    id: 'contact',
    name: 'Contact',
    iconName: 'person',
    defaultLayout: ObjectLayout.document,
    properties: [
      PropertyDefinition(
        id: 'phone',
        name: 'Phone',
        type: PropertyType.text,
      ),
      PropertyDefinition(
        id: 'email',
        name: 'Email',
        type: PropertyType.url,
      ),
    ],
  ),
  const ObjectTypeDefinition(
    id: 'item',
    name: 'Item',
    iconName: 'list_item',
    defaultLayout: ObjectLayout.collection,
  ),

  // ---------------------------------------------------------------------------
  // Default page item types (predefined schema — users cannot modify)
  // ---------------------------------------------------------------------------
  // ---------------------------------------------------------------------------
  // Default page item types (predefined schema — users cannot modify)
  // Prefixes avoid collisions with generic built-in types.
  // ---------------------------------------------------------------------------
  // Profile
  const ObjectTypeDefinition(
    id: 'profile_identity',
    name: 'Identity',
    iconName: 'person',
    defaultLayout: ObjectLayout.document,
    properties: [
      PropertyDefinition(id: 'fullName', name: 'Full Name', type: PropertyType.text),
      PropertyDefinition(id: 'givenName', name: 'Given Name', type: PropertyType.text),
      PropertyDefinition(id: 'familyName', name: 'Family Name', type: PropertyType.text),
      PropertyDefinition(id: 'dateOfBirth', name: 'Date of Birth', type: PropertyType.date),
      PropertyDefinition(id: 'gender', name: 'Gender', type: PropertyType.text),
      PropertyDefinition(id: 'nationality', name: 'Nationality', type: PropertyType.text),
    ],
  ),
  const ObjectTypeDefinition(
    id: 'profile_contact',
    name: 'Contact',
    iconName: 'contact_mail',
    defaultLayout: ObjectLayout.document,
    properties: [
      PropertyDefinition(id: 'title', name: 'Title', type: PropertyType.text),
      PropertyDefinition(id: 'type', name: 'Type', type: PropertyType.text),
      PropertyDefinition(id: 'value', name: 'Value', type: PropertyType.text),
    ],
  ),
  const ObjectTypeDefinition(
    id: 'profile_id_card',
    name: 'ID Card',
    iconName: 'badge',
    defaultLayout: ObjectLayout.document,
    properties: [
      PropertyDefinition(id: 'title', name: 'Title', type: PropertyType.text),
      PropertyDefinition(id: 'number', name: 'ID Card Number', type: PropertyType.text),
      PropertyDefinition(id: 'issueDate', name: 'Issue Date', type: PropertyType.date),
      PropertyDefinition(id: 'expiryDate', name: 'Expiry Date', type: PropertyType.date),
      PropertyDefinition(id: 'holderName', name: 'Holder Name', type: PropertyType.text),
      PropertyDefinition(id: 'country', name: 'Country', type: PropertyType.text),
    ],
  ),
  const ObjectTypeDefinition(
    id: 'profile_address',
    name: 'Address',
    iconName: 'home',
    defaultLayout: ObjectLayout.document,
    properties: [
      PropertyDefinition(id: 'title', name: 'Title', type: PropertyType.text),
      PropertyDefinition(id: 'street', name: 'Street', type: PropertyType.text),
      PropertyDefinition(id: 'city', name: 'City', type: PropertyType.text),
      PropertyDefinition(id: 'district', name: 'District', type: PropertyType.text),
      PropertyDefinition(id: 'state', name: 'State', type: PropertyType.text),
      PropertyDefinition(id: 'postalCode', name: 'Postal Code', type: PropertyType.text),
      PropertyDefinition(id: 'country', name: 'Country', type: PropertyType.text),
    ],
  ),

  // Travel
  const ObjectTypeDefinition(
    id: 'travel_passport',
    name: 'Passport',
    iconName: 'book',
    defaultLayout: ObjectLayout.document,
    properties: [
      PropertyDefinition(id: 'title', name: 'Type', type: PropertyType.text),
      PropertyDefinition(id: 'country', name: 'Country', type: PropertyType.text),
      PropertyDefinition(id: 'countryCode', name: 'Country Code', type: PropertyType.text),
      PropertyDefinition(id: 'number', name: 'Passport Number', type: PropertyType.text),
      PropertyDefinition(id: 'issueDate', name: 'Date of Issue', type: PropertyType.date),
      PropertyDefinition(id: 'placeOfIssue', name: 'Place of Issue', type: PropertyType.text),
      PropertyDefinition(id: 'expiryDate', name: 'Date of Expiry', type: PropertyType.date),
      PropertyDefinition(id: 'holderName', name: 'Holder Name', type: PropertyType.text),
      PropertyDefinition(id: 'dateOfBirth', name: 'Date of Birth', type: PropertyType.date),
      PropertyDefinition(id: 'placeOfBirth', name: 'Place of Birth', type: PropertyType.text),
      PropertyDefinition(id: 'sex', name: 'Sex', type: PropertyType.text),
      PropertyDefinition(id: 'nationality', name: 'Nationality', type: PropertyType.text),
      PropertyDefinition(id: 'authority', name: 'Authority', type: PropertyType.text),
    ],
  ),
  const ObjectTypeDefinition(
    id: 'travel_visa',
    name: 'Visa',
    iconName: 'assignment_ind',
    defaultLayout: ObjectLayout.document,
    properties: [
      PropertyDefinition(id: 'title', name: 'Title', type: PropertyType.text),
      PropertyDefinition(id: 'country', name: 'Country', type: PropertyType.text),
      PropertyDefinition(id: 'visaType', name: 'Visa Type', type: PropertyType.text),
      PropertyDefinition(id: 'number', name: 'Visa Number', type: PropertyType.text),
      PropertyDefinition(id: 'issueDate', name: 'Issue Date', type: PropertyType.date),
      PropertyDefinition(id: 'expiryDate', name: 'Expiry Date', type: PropertyType.date),
    ],
  ),
  const ObjectTypeDefinition(
    id: 'travel_history',
    name: 'Travel History',
    iconName: 'history',
    defaultLayout: ObjectLayout.document,
    properties: [
      PropertyDefinition(id: 'destination', name: 'Destination', type: PropertyType.text),
      PropertyDefinition(id: 'travelType', name: 'Travel Type', type: PropertyType.text),
      PropertyDefinition(id: 'date', name: 'Date', type: PropertyType.date),
      PropertyDefinition(id: 'departureCity', name: 'Departure City', type: PropertyType.text),
      PropertyDefinition(id: 'departureTime', name: 'Departure Time', type: PropertyType.text),
      PropertyDefinition(id: 'arrivalTime', name: 'Arrival Time', type: PropertyType.text),
      PropertyDefinition(id: 'flightNumber', name: 'Flight Number', type: PropertyType.text),
      PropertyDefinition(id: 'ticketPrice', name: 'Ticket Price', type: PropertyType.text),
      PropertyDefinition(id: 'airline', name: 'Airline', type: PropertyType.text),
    ],
  ),

  // Financial
  const ObjectTypeDefinition(
    id: 'financial_bank_account',
    name: 'Bank Account',
    iconName: 'account_balance',
    defaultLayout: ObjectLayout.document,
    properties: [
      PropertyDefinition(id: 'title', name: 'Title', type: PropertyType.text),
      PropertyDefinition(id: 'bankName', name: 'Bank Name', type: PropertyType.text),
      PropertyDefinition(id: 'accountNumber', name: 'Account Number', type: PropertyType.text),
      PropertyDefinition(id: 'currency', name: 'Currency', type: PropertyType.text),
      PropertyDefinition(id: 'swiftBic', name: 'SWIFT/BIC', type: PropertyType.text),
      PropertyDefinition(id: 'sortCode', name: 'Sort Code', type: PropertyType.text),
    ],
  ),
  const ObjectTypeDefinition(
    id: 'financial_card',
    name: 'Card',
    iconName: 'credit_card',
    defaultLayout: ObjectLayout.document,
    properties: [
      PropertyDefinition(id: 'title', name: 'Title', type: PropertyType.text),
      PropertyDefinition(id: 'cardNumber', name: 'Card Number', type: PropertyType.text),
      PropertyDefinition(id: 'cardType', name: 'Card Type', type: PropertyType.text),
      PropertyDefinition(id: 'expiryDate', name: 'Expiry Date', type: PropertyType.date),
      PropertyDefinition(id: 'holderName', name: 'Holder Name', type: PropertyType.text),
      PropertyDefinition(id: 'cvv', name: 'CVV', type: PropertyType.text),
    ],
  ),
  const ObjectTypeDefinition(
    id: 'financial_tax_id',
    name: 'Tax ID',
    iconName: 'description',
    defaultLayout: ObjectLayout.document,
    properties: [
      PropertyDefinition(id: 'title', name: 'Title', type: PropertyType.text),
      PropertyDefinition(id: 'taxIdNumber', name: 'Tax ID Number', type: PropertyType.text),
      PropertyDefinition(id: 'taxIdType', name: 'Tax ID Type', type: PropertyType.text),
      PropertyDefinition(id: 'issuingAuthority', name: 'Issuing Authority', type: PropertyType.text),
      PropertyDefinition(id: 'country', name: 'Country', type: PropertyType.text),
    ],
  ),

  // Professional
  const ObjectTypeDefinition(
    id: 'professional_education',
    name: 'Education',
    iconName: 'school',
    defaultLayout: ObjectLayout.document,
    properties: [
      PropertyDefinition(id: 'institution', name: 'Institution', type: PropertyType.text),
      PropertyDefinition(id: 'degree', name: 'Degree', type: PropertyType.text),
      PropertyDefinition(id: 'degreeCustom', name: 'Custom Degree', type: PropertyType.text),
      PropertyDefinition(id: 'field', name: 'Field of Study', type: PropertyType.text),
      PropertyDefinition(id: 'startDate', name: 'Start Date', type: PropertyType.date),
      PropertyDefinition(id: 'endDate', name: 'End Date', type: PropertyType.date),
    ],
  ),
  const ObjectTypeDefinition(
    id: 'professional_employment',
    name: 'Employment',
    iconName: 'work',
    defaultLayout: ObjectLayout.document,
    properties: [
      PropertyDefinition(id: 'company', name: 'Company', type: PropertyType.text),
      PropertyDefinition(id: 'position', name: 'Position', type: PropertyType.text),
      PropertyDefinition(id: 'responsibilities', name: 'Responsibilities', type: PropertyType.text),
      PropertyDefinition(id: 'startDate', name: 'Start Date', type: PropertyType.date),
      PropertyDefinition(id: 'endDate', name: 'End Date', type: PropertyType.date),
    ],
  ),
  const ObjectTypeDefinition(
    id: 'professional_skill',
    name: 'Skill',
    iconName: 'star',
    defaultLayout: ObjectLayout.document,
    properties: [
      PropertyDefinition(id: 'name', name: 'Skill Name', type: PropertyType.text),
      PropertyDefinition(id: 'level', name: 'Proficiency Level', type: PropertyType.text),
    ],
  ),
  const ObjectTypeDefinition(
    id: 'professional_language',
    name: 'Language',
    iconName: 'language',
    defaultLayout: ObjectLayout.document,
    properties: [
      PropertyDefinition(id: 'name', name: 'Language', type: PropertyType.text),
      PropertyDefinition(id: 'proficiency', name: 'Proficiency Level', type: PropertyType.text),
    ],
  ),
  const ObjectTypeDefinition(
    id: 'professional_award',
    name: 'Award',
    iconName: 'emoji_events',
    defaultLayout: ObjectLayout.document,
    properties: [
      PropertyDefinition(id: 'title', name: 'Title', type: PropertyType.text),
      PropertyDefinition(id: 'issuer', name: 'Issuer', type: PropertyType.text),
      PropertyDefinition(id: 'date', name: 'Date', type: PropertyType.date),
      PropertyDefinition(id: 'description', name: 'Description', type: PropertyType.text),
    ],
  ),
  const ObjectTypeDefinition(
    id: 'professional_article',
    name: 'Article',
    iconName: 'article',
    defaultLayout: ObjectLayout.document,
    properties: [
      PropertyDefinition(id: 'authors', name: 'Authors', type: PropertyType.text),
      PropertyDefinition(id: 'institution', name: 'Institution', type: PropertyType.text),
      PropertyDefinition(id: 'contact', name: 'Contact', type: PropertyType.text),
      PropertyDefinition(id: 'abstract', name: 'Abstract', type: PropertyType.text),
      PropertyDefinition(id: 'doi', name: 'DOI', type: PropertyType.text),
      PropertyDefinition(id: 'url', name: 'URL', type: PropertyType.url),
      PropertyDefinition(id: 'venue', name: 'Venue', type: PropertyType.text),
      PropertyDefinition(id: 'year', name: 'Year', type: PropertyType.text),
      PropertyDefinition(id: 'citation', name: 'Citation', type: PropertyType.text),
    ],
  ),
];

// =============================================================================
// UnifiedObjectService
// =============================================================================

/// Service for managing unified objects.
/// All operations are pure functions returning new instances (immutable style).
class UnifiedObjectService {
  static UnifiedObjectService? _instance;
  UnifiedObjectService._();
  static UnifiedObjectService get instance {
    _instance ??= UnifiedObjectService._();
    return _instance!;
  }

  // ---------------------------------------------------------------------------
  // Create
  // ---------------------------------------------------------------------------

  /// Create a new object.
  UnifiedObject createObject({
    required String name,
    String? typeId,
    String? parentId,
    String? iconName,
    Map<String, PropertyValue>? properties,
    Map<String, String>? propertyLabels,
    Map<String, String>? semanticTypes,
    List<String>? propertyOrder,
    List<String>? childrenIds,
  }) {
    final now = _currentTimestamp();
    final effectiveIcon = iconName ??
        ObjectTypeRegistry.getType(typeId ?? 'note')?.iconName ??
        'folder';
    return UnifiedObject(
      id: _generateId(),
      typeId: typeId,
      name: name,
      iconName: effectiveIcon,
      parentId: parentId,
      childrenIds: childrenIds ?? const [],
      properties: properties ?? const {},
      propertyLabels: propertyLabels,
      semanticTypes: semanticTypes,
      propertyOrder: propertyOrder ?? const [],
      createdAt: now,
      updatedAt: now,
    );
  }

  // ---------------------------------------------------------------------------
  // Update
  // ---------------------------------------------------------------------------

  UnifiedObject updateObject(
    UnifiedObject object, {
    String? name,
    String? typeId,
    String? iconName,
    String? parentId,
    Map<String, PropertyValue>? properties,
    Map<String, String>? propertyLabels,
    Map<String, String>? semanticTypes,
    List<String>? propertyOrder,
    List<String>? childrenIds,
    List<Attachment>? attachments,
    int? schemaVersionWhenSaved,
  }) {
    return object.copyWith(
      name: name,
      typeId: typeId,
      iconName: iconName,
      parentId: parentId,
      properties: properties,
      propertyLabels: propertyLabels,
      semanticTypes: semanticTypes,
      propertyOrder: propertyOrder,
      childrenIds: childrenIds,
      attachments: attachments,
      schemaVersionWhenSaved: schemaVersionWhenSaved,
      updatedAt: _currentTimestamp(),
    );
  }

  // ---------------------------------------------------------------------------
  // Soft Delete / Restore
  // ---------------------------------------------------------------------------

  UnifiedObject deleteObject(UnifiedObject object) {
    return object.copyWith(
      isDeleted: true,
      deletedAt: DateTime.now(),
      updatedAt: _currentTimestamp(),
    );
  }

  UnifiedObject restoreObject(UnifiedObject object) {
    return object.copyWith(
      isDeleted: false,
      deletedAt: null,
      updatedAt: _currentTimestamp(),
    );
  }

  // ---------------------------------------------------------------------------
  // Tree Operations
  // ---------------------------------------------------------------------------

  /// Get direct children of [parentId] in the order defined by parent's [childrenIds].
  List<UnifiedObject> getChildren(
    List<UnifiedObject> objects,
    String parentId,
  ) {
    final parent = getObjectById(objects, parentId);
    if (parent == null) return [];
    final map = {for (final o in objects) o.id: o};
    return parent.childrenIds
        .where((id) => map.containsKey(id))
        .map((id) => map[id]!)
        .where((o) => !o.isDeleted)
        .toList();
  }

  /// Get all root-level objects (parentId == null).
  List<UnifiedObject> getRootObjects(List<UnifiedObject> objects) {
    return objects
        .where((o) => o.parentId == null && !o.isDeleted)
        .toList();
  }

  /// Find object by ID.
  UnifiedObject? getObjectById(List<UnifiedObject> objects, String id) {
    for (final o in objects) {
      if (o.id == id) return o;
    }
    return null;
  }

  /// Recursively collect all descendant IDs of [objectId].
  Set<String> getDescendantIds(List<UnifiedObject> objects, String objectId) {
    final result = <String>{};
    final map = {for (final o in objects) o.id: o};
    void collect(String id) {
      final obj = map[id];
      if (obj == null) return;
      for (final childId in obj.childrenIds) {
        if (!result.contains(childId)) {
          result.add(childId);
          collect(childId);
        }
      }
    }
    collect(objectId);
    return result;
  }

  /// Move [objectId] to a new parent. Updates both old and new parent's childrenIds.
  List<UnifiedObject> moveObject(
    List<UnifiedObject> objects,
    String objectId,
    String? newParentId,
  ) {
    final object = getObjectById(objects, objectId);
    if (object == null) return objects;

    // Prevent moving an object into its own descendant
    if (newParentId != null) {
      final descendants = getDescendantIds(objects, objectId);
      if (newParentId == objectId || descendants.contains(newParentId)) {
        return objects;
      }
    }

    final oldParentId = object.parentId;
    final result = List<UnifiedObject>.from(objects);
    final now = _currentTimestamp();

    // If moving within the same parent (reorder), no parent-child relationship changes needed
    if (oldParentId == newParentId) {
      return result;
    }

    // Single pass: update old parent, new parent, and the object itself
    for (var i = 0; i < result.length; i++) {
      final o = result[i];
      if (oldParentId != null && o.id == oldParentId) {
        result[i] = o.copyWith(
          childrenIds: o.childrenIds.where((id) => id != objectId).toList(),
          updatedAt: now,
        );
      } else if (newParentId != null &&
          o.id == newParentId &&
          !o.childrenIds.contains(objectId)) {
        result[i] = o.copyWith(
          childrenIds: [...o.childrenIds, objectId],
          updatedAt: now,
        );
      } else if (o.id == objectId) {
        result[i] = o.copyWith(
          parentId: newParentId,
          updatedAt: now,
        );
      }
    }

    return result;
  }

  /// Reorder children within a parent by moving from [oldIndex] to [newIndex].
  List<UnifiedObject> reorderChildren(
    List<UnifiedObject> objects,
    String parentId,
    int oldIndex,
    int newIndex,
  ) {
    final parent = getObjectById(objects, parentId);
    if (parent == null) return objects;
    final children = List<String>.from(parent.childrenIds);
    if (oldIndex < 0 || oldIndex >= children.length) return objects;
    if (newIndex < 0 || newIndex >= children.length) return objects;
    if (oldIndex == newIndex) return objects;

    final item = children.removeAt(oldIndex);
    children.insert(newIndex, item);

    final idx = objects.indexWhere((o) => o.id == parentId);
    if (idx < 0) return objects;
    final result = List<UnifiedObject>.from(objects);
    result[idx] = parent.copyWith(
      childrenIds: children,
      updatedAt: _currentTimestamp(),
    );
    return result;
  }

  /// Add a child reference to a parent's childrenIds.
  List<UnifiedObject> addChild(
    List<UnifiedObject> objects,
    String parentId,
    String childId,
  ) {
    final idx = objects.indexWhere((o) => o.id == parentId);
    if (idx < 0) return objects;
    final parent = objects[idx];
    if (parent.childrenIds.contains(childId)) return objects;
    final result = List<UnifiedObject>.from(objects);
    result[idx] = parent.copyWith(
      childrenIds: [...parent.childrenIds, childId],
      updatedAt: _currentTimestamp(),
    );
    return result;
  }

  /// Remove a child reference from a parent's childrenIds.
  List<UnifiedObject> removeChild(
    List<UnifiedObject> objects,
    String parentId,
    String childId,
  ) {
    final idx = objects.indexWhere((o) => o.id == parentId);
    if (idx < 0) return objects;
    final parent = objects[idx];
    final result = List<UnifiedObject>.from(objects);
    result[idx] = parent.copyWith(
      childrenIds: parent.childrenIds.where((id) => id != childId).toList(),
      updatedAt: _currentTimestamp(),
    );
    return result;
  }

  // ---------------------------------------------------------------------------
  // Batch / List Helpers
  // ---------------------------------------------------------------------------

  List<UnifiedObject> addObject(
    List<UnifiedObject> objects,
    UnifiedObject newObject,
  ) {
    return [...objects, newObject];
  }

  List<UnifiedObject> replaceObject(
    List<UnifiedObject> objects,
    UnifiedObject updatedObject,
  ) {
    final idx = objects.indexWhere((o) => o.id == updatedObject.id);
    if (idx < 0) return objects;
    final result = List<UnifiedObject>.from(objects);
    result[idx] = updatedObject;
    return result;
  }

  List<UnifiedObject> removeObject(
    List<UnifiedObject> objects,
    String id,
  ) {
    return objects.where((o) => o.id != id).toList();
  }

  // ---------------------------------------------------------------------------
  // Icon Mapping
  // ---------------------------------------------------------------------------

  static IconData getIconFromName(String iconName) {
    return switch (iconName) {
      // 工作与学习
      'article' => Icons.article_outlined,
      'folder' => Icons.folder_outlined,
      'note' => Icons.note_outlined,
      'description' => Icons.description,
      'book' => Icons.book,
      'work' => Icons.work,
      'school' => Icons.school,
      'assignment' => Icons.assignment,
      'checklist' => Icons.checklist,
      'menu_book' => Icons.menu_book,
      'library_books' => Icons.library_books,
      'edit_document' => Icons.edit_document,
      // 人物与身份
      'person' => Icons.person_outlined,
      'badge' => Icons.badge,
      'people' => Icons.people,
      'family_restroom' => Icons.family_restroom,
      'child_care' => Icons.child_care,
      'face' => Icons.face,
      'fingerprint' => Icons.fingerprint,
      'verified_user' => Icons.verified_user,
      'account_circle' => Icons.account_circle,
      'person_outline' => Icons.person_outline,
      'groups' => Icons.groups,
      'diversity_3' => Icons.diversity_3,
      // 旅行与出行
      'flight' => Icons.flight,
      'hotel' => Icons.hotel,
      'car_rental' => Icons.car_rental,
      'luggage' => Icons.luggage,
      'map' => Icons.map,
      'place' => Icons.place,
      'location_on' => Icons.location_on,
      'train' => Icons.train,
      'directions_car' => Icons.directions_car,
      'commute' => Icons.commute,
      'beach_access' => Icons.beach_access,
      'public' => Icons.public,
      // 财务与商业
      'account_balance' => Icons.account_balance,
      'credit_card' => Icons.credit_card,
      'savings' => Icons.savings,
      'payments' => Icons.payments,
      'receipt_long' => Icons.receipt_long,
      'attach_money' => Icons.attach_money,
      'account_balance_wallet' => Icons.account_balance_wallet,
      'monetization_on' => Icons.monetization_on,
      'trending_up' => Icons.trending_up,
      'show_chart' => Icons.show_chart,
      'shopping_cart' => Icons.shopping_cart,
      'store' => Icons.store,
      // 生活与健康
      'home' => Icons.home,
      'favorite' => Icons.favorite,
      'star' => Icons.star,
      'restaurant' => Icons.restaurant,
      'sports' => Icons.sports,
      'fitness_center' => Icons.fitness_center,
      'medical_services' => Icons.medical_services,
      'local_hospital' => Icons.local_hospital,
      'spa' => Icons.spa,
      'self_improvement' => Icons.self_improvement,
      'healing' => Icons.healing,
      'health_and_safety' => Icons.health_and_safety,
      // 科技与设备
      'laptop' => Icons.laptop,
      'computer' => Icons.computer,
      'code' => Icons.code,
      'developer_mode' => Icons.developer_mode,
      'phone_android' => Icons.phone_android,
      'memory' => Icons.memory,
      'wifi' => Icons.wifi,
      'bluetooth' => Icons.bluetooth,
      'gamepad' => Icons.gamepad,
      'mouse' => Icons.mouse,
      'keyboard' => Icons.keyboard,
      'cable' => Icons.cable,
      // 创意与艺术
      'palette' => Icons.palette,
      'brush' => Icons.brush,
      'photo_camera' => Icons.photo_camera,
      'movie' => Icons.movie,
      'music_note' => Icons.music_note,
      'theaters' => Icons.theaters,
      'museum' => Icons.museum,
      'emoji_emotions' => Icons.emoji_emotions,
      'lightbulb' => Icons.lightbulb,
      'design_services' => Icons.design_services,
      'architecture' => Icons.architecture,
      'auto_stories' => Icons.auto_stories,
      // 通用
      'language' => Icons.language,
      'security' => Icons.security,
      'link' => Icons.link,
      'phone' => Icons.phone,
      'email' => Icons.email,
      'check_circle' => Icons.check_circle_outlined,
      'vpn_key' => Icons.vpn_key,
      'lock' => Icons.lock,
      'warning' => Icons.warning,
      'error' => Icons.error,
      'help' => Icons.help,
      'info' => Icons.info,
      // 旧版兼容
      'history' => Icons.history,
      'shopping_bag' => Icons.shopping_bag,
      'camera' => Icons.camera_alt,
      'pets' => Icons.pets,
      'add' => Icons.add,
      _ => Icons.folder_outlined,
    };
  }
}


// =============================================================================
// Icon Categories (top-level, outside UnifiedObjectService)
// =============================================================================

/// A named group of icon names for categorized display in the icon picker.
class IconCategory {
  final String nameKey;
  final List<String> iconNames;

  const IconCategory({required this.nameKey, required this.iconNames});
}

/// All available icon names grouped by category for the icon picker UI.
const List<IconCategory> kIconCategories = [
  IconCategory(
    nameKey: 'iconCategoryWork',
    iconNames: [
      'article', 'folder', 'note', 'description', 'book',
      'work', 'school', 'assignment', 'checklist', 'menu_book',
      'library_books', 'edit_document',
    ],
  ),
  IconCategory(
    nameKey: 'iconCategoryPeople',
    iconNames: [
      'person', 'badge', 'people', 'family_restroom', 'child_care',
      'face', 'fingerprint', 'verified_user', 'account_circle',
      'person_outline', 'groups', 'diversity_3',
    ],
  ),
  IconCategory(
    nameKey: 'iconCategoryTravel',
    iconNames: [
      'flight', 'hotel', 'car_rental', 'luggage', 'map',
      'place', 'location_on', 'train', 'directions_car',
      'commute', 'beach_access', 'public',
    ],
  ),
  IconCategory(
    nameKey: 'iconCategoryFinance',
    iconNames: [
      'account_balance', 'credit_card', 'savings', 'payments',
      'receipt_long', 'attach_money', 'account_balance_wallet',
      'monetization_on', 'trending_up', 'show_chart',
      'shopping_cart', 'store',
    ],
  ),
  IconCategory(
    nameKey: 'iconCategoryLife',
    iconNames: [
      'home', 'favorite', 'star', 'restaurant', 'sports',
      'fitness_center', 'medical_services', 'local_hospital',
      'spa', 'self_improvement', 'healing', 'health_and_safety',
    ],
  ),
  IconCategory(
    nameKey: 'iconCategoryTech',
    iconNames: [
      'laptop', 'computer', 'code', 'developer_mode', 'phone_android',
      'memory', 'wifi', 'bluetooth', 'gamepad', 'mouse',
      'keyboard', 'cable',
    ],
  ),
  IconCategory(
    nameKey: 'iconCategoryCreative',
    iconNames: [
      'palette', 'brush', 'photo_camera', 'movie', 'music_note',
      'theaters', 'museum', 'emoji_emotions', 'lightbulb',
      'design_services', 'architecture', 'auto_stories',
    ],
  ),
  IconCategory(
    nameKey: 'iconCategoryGeneral',
    iconNames: [
      'language', 'security', 'link', 'phone', 'email',
      'check_circle', 'vpn_key', 'lock', 'warning', 'error',
      'help', 'info',
    ],
  ),
];

/// Flat list of all icon names for backward compatibility.
List<String> get kAllIconNames {
  return kIconCategories.expand((c) => c.iconNames).toList();
}
