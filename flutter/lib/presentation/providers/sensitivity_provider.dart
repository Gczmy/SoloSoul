import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';

// Re-export SensitivityLevel from sensitivity_enums for backward compatibility
export 'package:solosoul_flutter/core/constants/sensitivity_enums.dart'
    show SensitivityLevel, SensitivityLevelExtension;

// Re-export AccountStyle, SensitivityResolver, SensitivityDisplayMode
export 'package:solosoul_flutter/presentation/providers/account_style_provider.dart'
    show AccountStyle, AccountStyleNotifier, accountStyleProvider,
        SensitivityResolver, sensitivityResolver, SensitivityDisplayMode;

// Re-export field models from sensitivity_models.dart
export 'package:solosoul_flutter/presentation/models/sensitivity_models.dart'
    show FieldSensitivity, FieldRegistry, FormFieldRegistry, FormFieldRegistryNotifier,
        firstWhereOrNull, FieldIds;

// Import models for internal use
import 'package:solosoul_flutter/presentation/models/sensitivity_models.dart'
    show FieldSensitivity, FieldRegistry, FormFieldRegistryNotifier, firstWhereOrNull;

// Import accountStyleProvider for internal use within this file
import 'package:solosoul_flutter/presentation/providers/account_style_provider.dart'
    show accountStyleProvider;

/// Provider for reactive field registry.
/// Forms register fields via this provider, settings page watches it.
final formFieldRegistryProvider =
    StateNotifierProvider<FormFieldRegistryNotifier, Map<String, FieldSensitivity>>((ref) {
  return FormFieldRegistryNotifier();
});

/// OPTIMIZED: Effective sensitivity level for a specific field.
/// Uses select() to narrow watch scope - only rebuilds when THIS fieldId changes.
final effectiveSensitivityProvider =
    Provider.family<SensitivityLevel, String>((ref, fieldId) {
  // Only watch this specific fieldId's registry entry
  final fieldDef = ref.watch(
    formFieldRegistryProvider.select((s) => s[fieldId]),
  );
  // Only watch this specific fieldId's user override
  final userOverride = ref.watch(
    accountStyleProvider.select((s) => s.fieldSettings[fieldId]),
  );
  // Watch revealed fields set
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

  // 4. Legacy FieldRegistry fallback
  final legacyField = firstWhereOrNull(
    FieldRegistry.defaultFields,
    (f) => f.fieldId == fieldId,
  );
  if (legacyField != null) {
    return legacyField.level;
  }

  // 5. Fallback to public
  return SensitivityLevel.public;
});

/// Provider for field metadata (name, section, etc.) for settings page display.
final fieldMetadataProvider =
    Provider.family<FieldSensitivity?, String>((ref, fieldId) {
  return ref.watch(
    formFieldRegistryProvider.select((s) => s[fieldId]),
  );
});

