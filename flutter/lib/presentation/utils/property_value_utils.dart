import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/presentation/utils/format_field_label.dart';

/// Convert a PropertyValue to its string representation.
String propValueToString(PropertyValue value, {String yesLabel = 'Yes', String noLabel = 'No'}) {
  return switch (value) {
    TextProperty(:final text) => text,
    NumberProperty(:final value) => value?.toString() ?? '',
    DateProperty(:final isoDate) => isoDate ?? '',
    CheckboxProperty(:final checked) => checked ? yesLabel : noLabel,
    SelectProperty(:final selectedId) => selectedId ?? '',
    MultiSelectProperty(:final selectedIds) => selectedIds.join(', '),
    RelationProperty(:final targetObjectId) => targetObjectId ?? '',
    UrlProperty(:final url) => url ?? '',
  };
}

/// camelCase / snake_case → Title Case (e.g. "givenName" → "Given Name", "visa_type" → "Visa Type")
String formatLabel(String key) => formatFieldLabel(key);

/// Wrap [text] with a newline every [n] characters, appending ': '.
String wrapEveryNChars(String text, int n) {
  if (text.length <= n) return '$text: ';
  final buffer = StringBuffer();
  for (var i = 0; i < text.length; i += n) {
    if (i > 0) buffer.write('\n');
    buffer.write(text.substring(i, i + n > text.length ? text.length : i + n));
  }
  buffer.write(': ');
  return buffer.toString();
}

/// Get the display title for an item.
/// Uses [nameExtractor] if provided, otherwise falls back to [titlePropertyKey]
/// and finally [item.name].
String objectItemDisplayTitle(
  UnifiedObject item, {
  String Function(Map<String, String>)? nameExtractor,
  String titlePropertyKey = 'Title',
}) {
  if (nameExtractor != null) {
    final props = <String, String>{
      for (final entry in item.properties.entries)
        entry.key: propValueToString(entry.value),
    };
    final extracted = nameExtractor(props);
    if (extracted.isNotEmpty && extracted != 'Untitled') {
      return extracted;
    }
  }
  final titleProp = item.properties[titlePropertyKey];
  if (titleProp is TextProperty && titleProp.text.isNotEmpty) {
    return titleProp.text;
  }
  // Legacy fallback for old 'Item Name' property
  final oldNameProp = item.properties['Item Name'];
  if (oldNameProp is TextProperty && oldNameProp.text.isNotEmpty) {
    return oldNameProp.text;
  }
  return item.name;
}

/// Map typeId to field-prefix used by FieldRegistry.
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
    _ => typeId,
  };
}
