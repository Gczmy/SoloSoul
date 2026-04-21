/// Extensions on List for common ID-based operations
extension ListIdUtils<T> on List<T> {
  /// Finds index of item with given [id] using [getId] extractor.
  /// Use when items lack == override (relies on object identity).
  ///
  /// Example: `_passports.indexById(passport.id, (p) => p.id)`
  int indexById(String id, String Function(T) getId) {
    return indexWhere((item) => getId(item) == id);
  }
}
