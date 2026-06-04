import 'package:json_annotation/json_annotation.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';

part 'profile_data.g.dart';

// Re-export for backward compatibility
/// Maximum character limits for form fields
const int kMaxFieldLength = 32;

/// Top-level profile container.
/// All data is stored in [unifiedObjects] as a tree of [UnifiedObject].
/// Legacy fields (identity/travel/financial/professional) have been removed.
@JsonSerializable(explicitToJson: true)
class ProfileData {
  @JsonKey(name: 'unified_objects')
  final UnifiedObjectData? unifiedObjects;
  @JsonKey(name: 'schema_version')
  final int? schemaVersion;

  const ProfileData({
    this.unifiedObjects,
    this.schemaVersion,
  });

  factory ProfileData.fromJson(Map<String, dynamic> json) {
    // 兼容 Rust 旧版本序列化：尝试 unified_objects，回退到 unifiedObjects
    final unifiedRaw = json['unified_objects'] ?? json['unifiedObjects'];
    return ProfileData(
      unifiedObjects: unifiedRaw == null
          ? null
          : UnifiedObjectData.fromJson(unifiedRaw as Map<String, dynamic>),
      schemaVersion: (json['schema_version'] as num?)?.toInt(),
    );
  }

  Map<String, dynamic> toJson() => _$ProfileDataToJson(this);

  /// Collect all item IDs from the unified object tree.
  /// Used for orphan history cleanup and integrity validation.
  Set<String> collectAllItemIds() {
    final ids = <String>{};
    final objects = unifiedObjects;
    if (objects != null) {
      for (final obj in objects.objects) {
        ids.add(obj.id);
      }
    }
    return ids;
  }

  ProfileData copyWith({
    UnifiedObjectData? unifiedObjects,
    int? schemaVersion,
  }) {
    return ProfileData(
      unifiedObjects: unifiedObjects ?? this.unifiedObjects,
      schemaVersion: schemaVersion ?? this.schemaVersion,
    );
  }
}
