import 'dart:convert';

import 'package:solosoul_flutter/core/models/unified_object_model.dart';

// =============================================================================
// Plugin Data Structure Service
// =============================================================================

/// 数据结构树节点（供插件查询，只读）。
///
/// 包含页面/分区/字段的元数据，不包含字段值。
class DataStructureNode {
  final String id;

  /// 'page' | 'section' | 'field'
  final String type;

  /// 显示名称（英文 fallback，UI 层应通过 [getLocalizedObjectName] 本地化）
  final String name;

  /// 字段类型（仅 type == 'field' 时有效）
  final String? fieldType;

  /// 子节点
  final List<DataStructureNode> children;

  const DataStructureNode({
    required this.id,
    required this.type,
    required this.name,
    this.fieldType,
    this.children = const [],
  });

  Map<String, dynamic> toJson() => {
        'id': id,
        'type': type,
        'name': name,
        if (fieldType != null) 'fieldType': fieldType,
        if (children.isNotEmpty)
          'children': children.map((c) => c.toJson()).toList(),
      };
}

/// 插件数据结构树服务。
///
/// 提供只读查询，返回当前账户的完整数据结构树（页面 → 分区 → 字段）。
/// 数据直接从 Dart 端的 [ProfileData.unifiedObjects] 构建，无需通过 Rust FFI。
class PluginDataStructureService {
  static List<DataStructureNode>? _cachedTree;
  static String? _cachedProfileId;

  /// 获取当前账户的完整数据结构树。
  ///
  /// [profileId] 用于缓存失效判断。如果传入的 profileId 与缓存一致且缓存未失效，
  /// 直接返回缓存结果。
  static List<DataStructureNode> getStructureTree(
    List<UnifiedObject> objects, {
    String? profileId,
  }) {
    // 缓存命中：同一账户、缓存未失效时直接返回
    if (_cachedTree != null && _cachedProfileId == profileId) {
      return _cachedTree!;
    }

    final result = <DataStructureNode>[];

    // 1. 获取所有页面（parentId == null 且 typeId == 'page'）
    final pages = objects.where(
      (o) => o.parentId == null && o.typeId == 'page' && !o.isDeleted,
    );

    for (final page in pages) {
      // 2. 获取该页面下的直接子分区
      final sections = objects.where(
        (o) =>
            o.parentId == page.id &&
            o.typeId != 'page' &&
            !o.isDeleted,
      );

      final sectionNodes = <DataStructureNode>[];
      for (final section in sections) {
        // 3. 获取分区的 schema（properties 的 key 列表）
        final fieldNodes = <DataStructureNode>[];
        final props = section.properties;
        final labels = section.propertyLabels ?? {};

        for (final entry in props.entries) {
          final key = entry.key;
          final value = entry.value;
          fieldNodes.add(DataStructureNode(
            id: key,
            type: 'field',
            name: labels[key] ?? key,
            fieldType: _propertyTypeName(value),
          ));
        }

        sectionNodes.add(DataStructureNode(
          id: section.id,
          type: 'section',
          name: section.name,
          children: fieldNodes,
        ));
      }

      result.add(DataStructureNode(
        id: page.id,
        type: 'page',
        name: page.name,
        children: sectionNodes,
      ));
    }

    // 更新缓存
    _cachedTree = result;
    _cachedProfileId = profileId;
    return result;
  }

  /// 将数据结构树序列化为 JSON 字符串。
  ///
  /// 供 Rust Host Function 调用时返回给插件。
  static String toJson(List<DataStructureNode> tree) {
    final list = tree.map((n) => n.toJson()).toList();
    return const JsonEncoder.withIndent('  ').convert(list);
  }

  /// 当数据结构发生变化时，调用此方法使缓存失效。
  static void invalidateCache() {
    _cachedTree = null;
    _cachedProfileId = null;
  }
}

/// 将 [PropertyValue] 子类映射为字符串类型名。
String _propertyTypeName(PropertyValue value) {
  return switch (value) {
    TextProperty() => 'text',
    NumberProperty() => 'number',
    DateProperty() => 'date',
    CheckboxProperty() => 'checkbox',
    SelectProperty() => 'select',
    MultiSelectProperty() => 'multiSelect',
    RelationProperty() => 'relation',
    UrlProperty() => 'url',
  };
}
