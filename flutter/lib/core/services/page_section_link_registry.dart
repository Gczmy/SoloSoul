import 'package:solosoul_flutter/core/services/unified_object_service.dart';

// =============================================================================
// Page-Section Link Registry
// =============================================================================

/// 页面-分区链接注册表。
///
/// 定义每个默认页面包含哪些默认分区。这是**默认配置**，用户可以通过
/// UI 修改链接（将分区移动到不同页面）。插件可以通过此注册表查询
/// 默认的数据结构树。
///
/// 核心原则：分区是去页面化的独立实体，页面只是分区的容器。
/// 此注册表仅用于**默认初始化**和**重置默认布局**时参考。
/// 用户移动分区后的实际链接存储在 [UnifiedObject.parentId] 中。
class PageSectionLinkRegistry {
  /// 默认页面-分区链接。
  /// key: pageId, value: 该页面下默认分区的 sectionId 列表（有序）
  static const Map<String, List<String>> _defaultLinks = {
    DefaultPageIds.profile: [
      DefaultSectionIds.identity,
      DefaultSectionIds.contact,
      DefaultSectionIds.idCard,
      DefaultSectionIds.address,
    ],
    DefaultPageIds.travel: [
      DefaultSectionIds.passport,
      DefaultSectionIds.visa,
      DefaultSectionIds.travelHistory,
    ],
    DefaultPageIds.financial: [
      DefaultSectionIds.bankAccount,
      DefaultSectionIds.card,
      DefaultSectionIds.taxId,
    ],
    DefaultPageIds.professional: [
      DefaultSectionIds.education,
      DefaultSectionIds.employment,
      DefaultSectionIds.skill,
      DefaultSectionIds.language,
      DefaultSectionIds.award,
      DefaultSectionIds.article,
    ],
  };

  /// 获取指定页面下所有默认分区的 sectionId 列表。
  static List<String> getDefaultSectionIdsForPage(String pageId) {
    return List.unmodifiable(_defaultLinks[pageId] ?? []);
  }

  /// 获取所有默认页面-分区链接。
  static Map<String, List<String>> get allDefaultLinks {
    return Map.unmodifiable(_defaultLinks);
  }

  /// 获取指定分区所属的默认页面 ID。
  static String? getDefaultPageIdForSection(String sectionId) {
    for (final entry in _defaultLinks.entries) {
      if (entry.value.contains(sectionId)) return entry.key;
    }
    return null;
  }

  /// 获取默认数据结构树（供插件查询）。
  static List<Map<String, dynamic>> getDefaultStructureTree() {
    return _defaultLinks.entries.map((entry) => {
      'pageId': entry.key,
      'sectionIds': entry.value,
    }).toList();
  }
}
