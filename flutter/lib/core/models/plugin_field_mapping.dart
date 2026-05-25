import 'dart:convert';
import 'package:flutter/material.dart';
import 'package:solosoul_flutter/core/services/fallback_secure_storage.dart';

/// 插件级字段映射配置。
///
/// 允许为每个已安装插件单独指定语义类型 → 机器 key 的映射关系。
/// 映射不会修改 Section 的 `__semanticTypes`，仅对该插件生效。
@immutable
class PluginFieldMapping {
  /// 插件 ID
  final String pluginId;

  /// 语义类型 → 机器 key 的映射
  /// 示例：{"pet.name": "auto_a3f7d2e1", "pet.breed": "auto_b2e18f4a"}
  final Map<String, String> semanticTypeToKey;

  /// Section ID 限定（可选）
  /// 插件可指定从哪个 section 读取语义类型字段
  final String? targetSectionId;

  const PluginFieldMapping({
    required this.pluginId,
    this.semanticTypeToKey = const {},
    this.targetSectionId,
  });

  Map<String, dynamic> toJson() => {
    'pluginId': pluginId,
    'semanticTypeToKey': semanticTypeToKey,
    'targetSectionId': targetSectionId,
  };

  factory PluginFieldMapping.fromJson(Map<String, dynamic> json) {
    return PluginFieldMapping(
      pluginId: json['pluginId'] as String,
      semanticTypeToKey: (json['semanticTypeToKey'] as Map<String, dynamic>? ?? {})
          .map((k, v) => MapEntry(k, v as String)),
      targetSectionId: json['targetSectionId'] as String?,
    );
  }

  PluginFieldMapping copyWith({
    String? pluginId,
    Map<String, String>? semanticTypeToKey,
    String? targetSectionId,
  }) {
    return PluginFieldMapping(
      pluginId: pluginId ?? this.pluginId,
      semanticTypeToKey: semanticTypeToKey ?? this.semanticTypeToKey,
      targetSectionId: targetSectionId ?? this.targetSectionId,
    );
  }
}

/// 所有插件字段映射的集合。
class PluginFieldMappingCollection {
  final Map<String, PluginFieldMapping> _mappings;

  PluginFieldMappingCollection({
    Map<String, PluginFieldMapping>? mappings,
  }) : _mappings = mappings ?? {};

  PluginFieldMapping? getMapping(String pluginId) => _mappings[pluginId];

  void setMapping(PluginFieldMapping mapping) {
    _mappings[mapping.pluginId] = mapping;
  }

  void removeMapping(String pluginId) {
    _mappings.remove(pluginId);
  }

  Map<String, dynamic> toJson() {
    return {
      for (final entry in _mappings.entries)
        entry.key: entry.value.toJson(),
    };
  }

  factory PluginFieldMappingCollection.fromJson(Map<String, dynamic> json) {
    final mappings = <String, PluginFieldMapping>{};
    for (final entry in json.entries) {
      if (entry.value is Map<String, dynamic>) {
        mappings[entry.key] = PluginFieldMapping.fromJson(
          entry.value as Map<String, dynamic>,
        );
      }
    }
    return PluginFieldMappingCollection(mappings: mappings);
  }
}

/// 插件字段映射持久化服务。
class PluginFieldMappingService {
  static const _storageKey = 'plugin_field_mappings';
  static final _instance = PluginFieldMappingService._internal();
  factory PluginFieldMappingService() => _instance;
  PluginFieldMappingService._internal();

  final _storage = FallbackSecureStorage();
  PluginFieldMappingCollection? _cached;

  Future<PluginFieldMappingCollection> loadMappings() async {
    if (_cached != null) return _cached!;
    try {
      final data = await _storage.read(key: _storageKey);
      if (data != null && data.isNotEmpty) {
        final json = jsonDecode(data) as Map<String, dynamic>;
        _cached = PluginFieldMappingCollection.fromJson(json);
        return _cached!;
      }
    } on Exception {
      // 忽略加载错误，返回空集合
    }
    _cached = PluginFieldMappingCollection();
    return _cached!;
  }

  Future<void> saveMappings(PluginFieldMappingCollection collection) async {
    _cached = collection;
    final json = jsonEncode(collection.toJson());
    await _storage.write(key: _storageKey, value: json);
  }

  Future<PluginFieldMapping?> getMapping(String pluginId) async {
    final collection = await loadMappings();
    return collection.getMapping(pluginId);
  }

  Future<void> setMapping(PluginFieldMapping mapping) async {
    final collection = await loadMappings();
    collection.setMapping(mapping);
    await saveMappings(collection);
  }

  Future<void> removeMapping(String pluginId) async {
    final collection = await loadMappings();
    collection.removeMapping(pluginId);
    await saveMappings(collection);
  }

  Future<void> setSemanticTypeMapping({
    required String pluginId,
    required String semanticType,
    required String machineKey,
    String? sectionId,
  }) async {
    final collection = await loadMappings();
    final existing = collection.getMapping(pluginId);
    final updatedMap = Map<String, String>.from(
      existing?.semanticTypeToKey ?? {},
    );
    updatedMap[semanticType] = machineKey;

    final updated = PluginFieldMapping(
      pluginId: pluginId,
      semanticTypeToKey: updatedMap,
      targetSectionId: sectionId ?? existing?.targetSectionId,
    );
    collection.setMapping(updated);
    await saveMappings(collection);
  }
}
