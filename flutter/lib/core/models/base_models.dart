/// Interface for entries that can format themselves for sharing/copying.
abstract class FormattableEntry {
  String toShareableString();
  String get entryType;
}
