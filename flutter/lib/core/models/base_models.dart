/// Interface for items that have a unique identifier.
/// Used by UnifiedFormSection to properly type its generic parameter.
abstract class IdentifiableItem {
  String get id;
}

/// Mixin for entries that can format themselves for sharing/copying.
mixin FormattableEntry {
  String get entryType;
  Map<String, dynamic> toMap();

  String toFormattedString() {
    final data = toMap();
    final ignoreKeys = {'id', 'createdAt', 'updatedAt', 'fieldHistories'};

    return data.entries
        .where((e) => !ignoreKeys.contains(e.key) && e.value != null && e.value.toString().isNotEmpty)
        .map((e) => '${_capitalize(e.key)}: ${e.value}')
        .join('\n');
  }

  String _capitalize(String s) => s.isEmpty ? s : '${s[0].toUpperCase()}${s.substring(1)}';
}
