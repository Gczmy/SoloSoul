# Architecture Decision: Freezed Introduction Assessment

**Date:** 2026-04-23
**Status:** Rejected for Phase 1 (P3)
**Project:** SoloSoul Flutter

---

## Context

SoloSoul uses json_serializable for JSON serialization on all Entry classes. A P2 task was created to evaluate introducing Freezed as a potential improvement for immutable data classes and better copyWith semantics.

---

## Analysis Summary

### Entry Classes Analyzed: 14 total

| Class | Fields | Sentinel Pattern | FormattableEntry |
|-------|--------|-----------------|------------------|
| SkillData | 5 | No | Yes |
| LanguageData | 5 | No | Yes |
| ContactEntry | 7 | Yes | Yes |
| AddressData | 8 | Yes | Yes |
| IdCardData | 8 | Yes | Yes |
| TravelHistoryData | 11 | Yes | Yes |
| PassportData | 14 | Yes | Yes |
| VisaData | 8 | Yes | Yes |
| BankAccountData | 8 | Yes | Yes |
| CardData | 8 | Yes | Yes |
| TaxIdData | 7 | Yes | Yes |
| AwardData | 6 | Yes | Yes |
| EducationData | 8 | Yes | Yes |
| EmploymentData | 7 | Yes | Yes |

---

## Key Blockers Identified

### 1. FormattableEntry Mixin Conflict

All Entry classes use `FormattableEntry` mixin which defines:
- `String get entryType`
- `Map<String, dynamic> toMap()` (for formatting)
- `String toFormattedString()`

**Problem:** Freezed generates its own `toMap()` for JSON serialization. This creates a method signature conflict.

**Example:**
```dart
mixin FormattableEntry {
  String get entryType;
  Map<String, dynamic> toMap(); // For formatting
}
```

### 2. Sentinel Pattern for copyWith

10 of 14 Entry classes use a sentinel pattern to distinguish "not provided" from "explicitly null" for the `deletedAt` field:

```dart
static const _sentinel = _DeletedAtSentinel();

copyWith({
  String? id,
  DateTime? deletedAt = _sentinel,  // Sentinel, not nullable
}) {
  this.deletedAt = identical(deletedAt, _sentinel)
      ? this.deletedAt
      : deletedAt as DateTime?;
}
```

**Problem:** Freezed's copyWith doesn't support this sentinel pattern elegantly.

### 3. IdentifiableItem Interface

All Entry classes implement `IdentifiableItem { String get id; }`. While Freezed supports `@Implements<>()`, it creates complexity with the generated code.

---

## Pilot Attempt

A Phase 1 pilot was attempted with `SkillData` (the simplest class). The attempt revealed:

1. `@Implements<IdentifiableItem>()` on Freezed constructor doesn't properly expose the interface to consumers like `UnifiedFormSection<T extends IdentifiableItem>`
2. Even with complex workarounds (intermediate base classes), the generated code doesn't play well with existing generic constraints
3. The complexity required to make it work ("black magic") would create technical debt

---

## Decision

**Rejected for Phase 1 (P3)**

The current architecture relies on:
- `FormattableEntry` mixin for polymorphic formatting behavior
- `IdentifiableItem` interface for generic constraints
- Sentinel pattern for proper null handling in copyWith

These patterns are deeply embedded and not compatible with Freezed without significant architectural changes.

---

## Current Status Quo

The existing approach using `json_serializable` + manual `copyWith` methods continues to work reliably. The code is deterministic and maintainable.

**Benefits of staying:**
- Deterministic serialization
- No "magic" code generation
- Sentinel pattern works correctly
- All Entry classes use same pattern (consistency)
- Interface/mixin polymorphism preserved

---

## Future Consideration (Phase 3+)

If Freezed is revisited in the future, required changes:

1. **Extract formatting logic** from `FormattableEntry` into Extension methods
2. **Redesign sentinel pattern** - perhaps use a `RestoreData` wrapper class
3. **Separate interface implementation** from data class (maybe a mixin that Freezed can properly implement)
4. **Consider manual JSON serialization** alongside Freezed if `toMap()` conflict persists

**Or:** Accept Freezed's limitations and use it only for new, simple data classes without these patterns.

---

## Related Decisions

- [001_riverpod_v2_migration.md](./001_riverpod_v2_migration.md) - Riverpod v2 AsyncNotifier migration completed successfully
