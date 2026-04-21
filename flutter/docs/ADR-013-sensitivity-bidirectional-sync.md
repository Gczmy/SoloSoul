# ADR-013: Sensitive Settings Bidirectional Sync

**Status:** Implemented
**Date:** 2026-04-21
**Last Updated:** 2026-04-21

## Context

The sensitivity settings system has two synchronization requirements:

1. **Settings Page <- Form (Field List Sync):** Settings page needs to dynamically discover all registered fields
2. **Form <- Settings Page (Sensitivity Value Sync):** Forms need to react when users modify field sensitivity in settings

### Current Architecture

```
┌─────────────────────────────────────────────────────────────┐
│ FormFieldRegistry (static Map<String, FieldSensitivity>)   │
│ - Populated at runtime via FormFieldRegistry.registerAll() │
│ - Called in UnifiedFormSection.initState()                  │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│ FieldRegistry.defaultFields (hardcoded legacy fields)       │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│ SensitivityResolver.resolve()                               │
│ Priority: revealedFields > fieldSettings > tagDefaults >    │
│           FormFieldRegistry > FieldRegistry > public        │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│ accountStyleProvider.fieldSettings (user overrides)         │
└─────────────────────────────────────────────────────────────┘
```

### Current Problems

1. **Problem A:** `FormFieldRegistry` is a static singleton. When a form registers fields, the settings page may not rebuild to show them.

2. **Problem B:** Forms use `field.sensitivity` from `FormFieldDef` directly in widgets, rather than always resolving through `fieldLevelProvider`. This means user overrides in `accountStyleProvider.fieldSettings` don't propagate to forms until the form is rebuilt from scratch.

3. **Problem C:** `UnifiedFormSection._registerFields()` is called in `initState`, but if a parent widget rebuilds the form section with different `fieldDefs`, the registry may have stale entries.

## Decision

### 1. Single Source of Truth

**Decision:** Keep `FormFieldDef.sensitivity` as the canonical default, but ensure all runtime sensitivity decisions go through `effectiveSensitivityProvider`.

**Rationale:** The registry pattern is correct for discovery. The issue is that widgets bypass the resolver when rendering.

### 2. Reactive Update Mechanism

**Decision:** Introduce a `formFieldRegistryProvider` that widgets can watch. Forms register fields into a Riverpod StateNotifier, and consumers watch the provider.

**Before (static registry):**
```dart
FormFieldRegistry.registerAll(fields);  // static side-effect
```

**After (reactive registry):**
```dart
ref.read(formFieldRegistryProvider.notifier).registerAll(fields);  // state update
```

### 3. Field Registration Timing

**Decision:** Keep `initState` registration, but wrap in `WidgetsBinding.instance.addPostFrameCallback` to ensure the widget tree is ready.

**Additionally:** Add a `didUpdateWidget` handler to re-register when `fieldDefs` change.

### 4. Performance Optimization: Selective Watching

**Decision:** Use `select()` to narrow the watch scope per fieldId, avoiding unnecessary rebuilds.

**Rationale:** Without selective watching, any field registration would trigger all widgets watching the provider to rebuild.

```dart
// Narrow scope: only rebuild when THIS specific fieldId changes
final fieldDef = ref.watch(formFieldRegistryProvider.select((s) => s[fieldId]));
final userOverride = ref.watch(accountStyleProvider.select((s) => s.fieldSettings[fieldId]));
final revealedFields = ref.watch(accountStyleProvider.select((s) => s.revealedFields));
```

### 5. Provider Separation: Sensitivity vs Metadata

**Decision:** Split into two focused providers:

| Provider | Return Type | Purpose |
|----------|-------------|---------|
| `effectiveSensitivityProvider` | `SensitivityLevel` | Pure sensitivity value for UI decisions |
| `fieldMetadataProvider` | `FieldSensitivity?` | Field metadata (Chinese name, section, etc.) |

**Rationale:** Separation of concerns improves testability and avoids unnecessary rebuilds.

### 6. Race Condition Protection

**Decision:** Use Set deduplication in `getAllFields()` to handle concurrent page loading.

**Risk:** If `reset()` runs after `registerAll()` during fast page switching, field list could be lost.

**Mitigation:** `getAllFields()` returns deduplicated results, and `reset()` is only called on app lock (not on page exit).

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    FormFieldRegistryNotifier                     │
│  - extends StateNotifier<Map<String, FieldSensitivity>>          │
│  - register(FieldSensitivity field)                              │
│  - registerAll(List<FieldSensitivity> fields)                   │
│  - getField(String fieldId) -> FieldSensitivity?                │
│  - getAllFields() -> List<FieldSensitivity> (sorted, deduped)  │
│  - reset()                                                       │
│  - Debug logging: which page registered which fields             │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                 formFieldRegistryProvider                         │
│  - StateNotifierProvider<FormFieldRegistryNotifier,             │
│                        Map<String, FieldSensitivity>>            │
└─────────────────────────────────────────────────────────────────┘
                              │
            ┌────────────────┴────────────────┐
            ▼                                 ▼
┌───────────────────────────────┐  ┌───────────────────────────────────┐
│ effectiveSensitivityProvider  │  │ fieldMetadataProvider              │
│ Provider.family<SensitivityLevel, String>  │  │ Provider.family<FieldSensitivity?, String> │
│ - select(registry[fieldId])  │  │ - select(registry[fieldId])        │
│ - select(settings[fieldId])  │  │ - Returns full FieldSensitivity   │
│ - Returns pure SensitivityLevel  │  │ - For settings page display     │
└───────────────────────────────┘  └───────────────────────────────────┘
```

## Key Changes

### 1. Replace Static Registry with StateNotifier

```dart
// NEW: Reactive registry with Riverpod
class FormFieldRegistryNotifier extends StateNotifier<Map<String, FieldSensitivity>> {
  FormFieldRegistryNotifier() : super({});

  void register(FieldSensitivity field) {
    state = {...state, field.fieldId: field};
  }

  void registerAll(List<FieldSensitivity> fields) {
    debugPrint('[FormFieldRegistry] Registering ${fields.length} fields');
    state = {...state, for (var f in fields) f.fieldId: f};
  }

  void reset() => state = {};

  FieldSensitivity? getField(String fieldId) => state[fieldId];

  List<FieldSensitivity> getAllFields() {
    // Deduplicate and sort
    final deduped = state.values.toSet().toList();
    deduped.sort((a, b) {
      final sec = a.fieldSection.compareTo(b.fieldSection);
      return sec != 0 ? sec : a.fieldName.compareTo(b.fieldName);
    });
    return deduped;
  }
}

final formFieldRegistryProvider =
    StateNotifierProvider<FormFieldRegistryNotifier, Map<String, FieldSensitivity>>((ref) {
  return FormFieldRegistryNotifier();
});
```

### 2. Split Providers for Performance

```dart
// OPTIMIZED: Single sensitivity value - narrow watch scope
final effectiveSensitivityProvider =
    Provider.family<SensitivityLevel, String>((ref, fieldId) {
  // Only rebuild when THIS specific fieldId changes
  final fieldDef = ref.watch(
    formFieldRegistryProvider.select((s) => s[fieldId]),
  );
  final userOverride = ref.watch(
    accountStyleProvider.select((s) => s.fieldSettings[fieldId]),
  );
  final revealedFields = ref.watch(
    accountStyleProvider.select((s) => s.revealedFields),
  );

  // 1. Temporary reveal
  if (revealedFields.contains(fieldId)) {
    return SensitivityLevel.public;
  }

  // 2. User override
  if (userOverride != null) {
    return userOverride;
  }

  // 3. Registry default
  if (fieldDef != null) {
    return fieldDef.level;
  }

  // 4. Fallback
  return SensitivityLevel.public;
});

// For settings page metadata display
final fieldMetadataProvider =
    Provider.family<FieldSensitivity?, String>((ref, fieldId) {
  return ref.watch(
    formFieldRegistryProvider.select((s) => s[fieldId]),
  );
});
```

### 3. Update SensitivityResolver to Use Provider

```dart
// Modify SensitivityResolver.resolve to use the new provider-based registry
class SensitivityResolver {
  const SensitivityResolver();

  SensitivityLevel resolve({
    required String fieldId,
    required Map<String, SensitivityLevel> fieldSettings,
    required Set<String> revealedFields,
    List<String> tags = const [],
    Map<String, FieldSensitivity>? registryFields,  // NEW: injected for testing
  }) {
    // 1. Temporary reveal
    if (revealedFields.contains(fieldId)) {
      return SensitivityLevel.public;
    }

    // 2. User override
    final userLevel = fieldSettings[fieldId];
    if (userLevel != null) {
      return userLevel;
    }

    // 3. Tag-based default
    for (final tag in tags) {
      final tagLevel = _tagDefaults[tag];
      if (tagLevel != null) {
        return tagLevel;
      }
    }

    // 4. FormFieldRegistry (from provider)
    final formFieldLevel = registryFields?[fieldId]?.level;
    if (formFieldLevel != null) {
      return formFieldLevel;
    }

    // 5. Legacy FieldRegistry fallback
    // ... (unchanged)
  }
}
```

### 4. Update UnifiedFormSection to Register Reactively

```dart
void _registerFields() {
  final section = widget.title.toLowerCase().replaceAll(' ', '');
  final fields = widget.fieldDefs.map((def) {
    return FieldSensitivity(
      fieldId: def.fieldId,
      fieldName: def.label,
      fieldSection: section,
      level: def.sensitivity,
    );
  }).toList();

  // Use provider instead of static registry
  // Wrap in addPostFrameCallback to avoid "Provider update during build"
  WidgetsBinding.instance.addPostFrameCallback((_) {
    if (mounted) {
      ref.read(formFieldRegistryProvider.notifier).registerAll(fields);
    }
  });
}
```

### 5. Update Settings Page to Watch Registry

```dart
// In sensitivity_settings_page.dart
final registry = ref.watch(formFieldRegistryProvider);
final allFields = registry.values.toSet().toList()  // Deduplicate
  ..sort((a, b) {
    final sec = a.fieldSection.compareTo(b.fieldSection);
    return sec != 0 ? sec : a.fieldName.compareTo(b.fieldName);
  });
```

### 6. Problem B Solution: SensitiveValueWidget

```dart
// BEFORE: Direct field.sensitivity access (bypasses user overrides)
SensitivityTag(level: field.sensitivity)

// AFTER: Through effectiveSensitivityProvider (respects overrides)
SensitivityTag(level: ref.watch(effectiveSensitivityProvider(field.fieldId)))
```

**Rule:** Once `FormFieldDef.sensitivity` is registered into the registry, UI should NEVER read `FormFieldDef.sensitivity` directly.

## Migration Path

### Phase 1: Introduce New Infrastructure
1. Create `FormFieldRegistryNotifier` and `formFieldRegistryProvider`
2. Create `effectiveSensitivityProvider` with selective watching
3. Create `fieldMetadataProvider`
4. Keep old `FormFieldRegistry` static methods as forwarding calls to new notifier (backwards compat)

### Phase 2: Migrate Consumers
1. Update `SensitiveValueWidget` to use `effectiveSensitivityProvider`
2. Update `SensitivityTag` in form fields to use `effectiveSensitivityProvider`
3. Update `sensitivity_settings_page.dart` to watch `formFieldRegistryProvider`
4. Update all `fieldLevelProvider` usages to `effectiveSensitivityProvider`
5. Use `fieldMetadataProvider` for settings page field list display

### Phase 3: Remove Legacy
1. Remove static `FormFieldRegistry` class (only keep `FieldRegistry.defaultFields` as fallback)
2. Remove `fieldLevelProvider` - consolidate to `effectiveSensitivityProvider`

## Consequences

### Positive
- Settings page automatically updates when forms register new fields
- Forms automatically reflect user sensitivity overrides without rebuilding
- Testability improves (can mock registry state)
- Single source of truth for sensitivity resolution
- Performance: selective watching prevents unnecessary rebuilds

### Negative
- Breaking change: `fieldLevelProvider` replaced with `effectiveSensitivityProvider`
- Requires updating all call sites
- Registry state persists across form rebuilds (mitigated by deduplication)

## Alternative Considered

**Option A: Keep Static Registry + Add Event Bus**
- Use an event bus to notify settings page when fields register
- Rejected: adds coupling, harder to test

**Option B: Always Rebuild Forms on Settings Change**
- Force form rebuilds when `accountStyleProvider` changes
- Rejected: inefficient, loses form state

**Option C: Keep Both Registries**
- Maintain both static `FormFieldRegistry` and new `formFieldRegistryProvider`
- Rejected: confusion about which to use

## Debugging Support

Add logging to `FormFieldRegistryNotifier`:

```dart
void registerAll(List<FieldSensitivity> fields) {
  debugPrint('[FormFieldRegistry] Registering ${fields.length} fields: '
    '${fields.map((f) => f.fieldId).join(', ')}');
  state = {...state, for (var f in fields) f.fieldId: f};
}

void reset() {
  debugPrint('[FormFieldRegistry] Reset called');
  state = {};
}
```

This helps diagnose "why isn't field X appearing in settings page".

## Fallback Strategy

`FieldRegistry.defaultFields` serves as fallback for scenarios where forms aren't rendered (e.g., search result summary display). This ensures sensitivity resolution always has a valid answer.

## Reviewers

- [x] Architecture Review
- [x] Security Review (sensitivity data handling)
- [x] Frontend Review

## Implementation Status

| Phase | Description | Status |
|-------|-------------|--------|
| Phase 1 | New reactive infrastructure | ✅ Complete |
| Phase 2 | Consumer migration | ✅ Complete |
| Phase 3 | Legacy cleanup | ⏳ Deferred (FieldRegistry.defaultFields kept as fallback) |
