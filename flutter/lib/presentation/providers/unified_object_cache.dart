part of 'unified_object_provider.dart';

// =============================================================================
// Section Metadata Helpers
// =============================================================================

/// Metadata for a default section with a fixed ID.
class SectionMeta {
  final String id;
  final String name;
  final String iconName;
  final String parentPageId;

  const SectionMeta({
    required this.id,
    required this.name,
    required this.iconName,
    required this.parentPageId,
  });
}

/// Get the section ID for a given item type, or null if the item type
/// does not belong to a default section.
String? getDefaultSectionIdForItemType(String typeId) {
  return switch (typeId) {
    // Legacy types (kept for backward compatibility)
    'profile_item' => 'section_profile_default',
    'travel_item' => 'section_travel_default',
    'financial_item' => 'section_financial_default',
    'professional_item' => 'section_professional_default',
    // Modern preset types
    '__preset_identity' => DefaultSectionIds.identity,
    '__preset_contact' => DefaultSectionIds.contact,
    '__preset_identity_document' => DefaultSectionIds.idCard,
    '__preset_address' => DefaultSectionIds.address,
    '__preset_passport' => DefaultSectionIds.passport,
    '__preset_visa' => DefaultSectionIds.visa,
    '__preset_travel_history' => DefaultSectionIds.travelHistory,
    '__preset_bank_account' => DefaultSectionIds.bankAccount,
    '__preset_payment_card' => DefaultSectionIds.card,
    '__preset_tax_id' => DefaultSectionIds.taxId,
    '__preset_education' => DefaultSectionIds.education,
    '__preset_employment' => DefaultSectionIds.employment,
    '__preset_skill' => DefaultSectionIds.skill,
    '__preset_language' => DefaultSectionIds.language,
    '__preset_award' => DefaultSectionIds.award,
    '__preset_article' => DefaultSectionIds.article,
    _ => null,
  };
}

/// Get metadata for a default section by its fixed ID.
SectionMeta? getSectionMeta(String sectionId) {
  return switch (sectionId) {
    'section_profile_default' => const SectionMeta(
        id: 'section_profile_default',
        name: 'Default',
        iconName: 'article',
        parentPageId: DefaultPageIds.profile,
      ),
    'section_travel_default' => const SectionMeta(
        id: 'section_travel_default',
        name: 'Default',
        iconName: 'article',
        parentPageId: DefaultPageIds.travel,
      ),
    'section_financial_default' => const SectionMeta(
        id: 'section_financial_default',
        name: 'Default',
        iconName: 'article',
        parentPageId: DefaultPageIds.financial,
      ),
    'section_professional_default' => const SectionMeta(
        id: 'section_professional_default',
        name: 'Default',
        iconName: 'article',
        parentPageId: DefaultPageIds.professional,
      ),
    _ => null,
  };
}

// =============================================================================
// Pre-computed Cache
// =============================================================================

/// 预计算缓存：所有对象的工作区内容一次性算好，点击页面时直接读取，无需现场遍历。
@immutable
class UnifiedObjectCache {
  final Map<String, UnifiedObject> objectById;

  /// parentId → 该 parent 下非 page 类型的子对象列表（workspace 显示用）
  final Map<String, List<UnifiedObject>> workspaceChildren;

  /// parentId → 该 parent 下 type=='item' 的子对象列表（ObjectCard 显示用）
  final Map<String, List<UnifiedObject>> itemChildren;

  /// 根级对象列表（parentId == null，未删除）
  final List<UnifiedObject> rootObjects;

  const UnifiedObjectCache({
    required this.objectById,
    required this.workspaceChildren,
    required this.itemChildren,
    required this.rootObjects,
  });
}

/// 全局预计算缓存 Provider：只监听 objects 列表，数据变化时一次性重建全部索引。
final unifiedObjectCacheProvider = Provider<UnifiedObjectCache>((ref) {
  final objects = ref.watch(unifiedObjectProvider.select((d) => d.objects));
  final map = {for (final o in objects) o.id: o};

  final objectById = <String, UnifiedObject>{};
  final workspaceChildren = <String, List<UnifiedObject>>{};
  final itemChildren = <String, List<UnifiedObject>>{};

  for (final obj in objects) {
    if (obj.isDeleted) continue;
    objectById[obj.id] = obj;

    final childList = obj.childrenIds
        .map((id) => map[id])
        .whereType<UnifiedObject>()
        .where((o) => !o.isDeleted)
        .toList();

    workspaceChildren[obj.id] = childList.where((c) => c.typeId != 'page').toList();
    itemChildren[obj.id] = childList.where((c) => c.typeId == 'item').toList();
  }

  final rootObjects = objects
      .where((o) => o.parentId == null && !o.isDeleted)
      .toList();

  // Diagnostic logging — always print, do not gate on DebugLogger.isActive
  final totalWsChildren = workspaceChildren.values.fold<int>(0, (sum, list) => sum + list.length);
  final totalItemChildren = itemChildren.values.fold<int>(0, (sum, list) => sum + list.length);

  // Show per-page children counts for debugging
  final pageChildrenCounts = <String, int>{};
  for (final entry in workspaceChildren.entries) {
    final keyName = objectById[entry.key]?.name ?? entry.key;
    pageChildrenCounts[keyName] = entry.value.length;
  }
  // Only show pages with children, sorted by count desc
  final sortedPages = pageChildrenCounts.entries.toList()
    ..sort((a, b) => b.value.compareTo(a.value));
  final topPages = sortedPages.take(10).map((e) => '${e.key}=${e.value}').join(', ');

  // ignore: avoid_print
  print('[DIAG-CACHE] Rebuilt: objects=${objects.length}, root=${rootObjects.length}, '
      'wsChildren=$totalWsChildren, itemChildren=$totalItemChildren');
  // ignore: avoid_print
  print('[DIAG-CACHE] topPages: $topPages');

  return UnifiedObjectCache(
    objectById: objectById,
    workspaceChildren: workspaceChildren,
    itemChildren: itemChildren,
    rootObjects: rootObjects,
  );
});
