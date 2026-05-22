# Phase 3: 默认页面与自定义页面对齐方案

> 目标：让默认页面（Profile/Travel/Financial/Professional）的分区与自定义页面对齐——
> 默认页面 = 自定义页面 + 预装分区。用户可自由增删改任何分区。

---

## 1. 现状诊断

### 1.1 默认页面 vs 自定义页面的关键差异

| 维度 | 默认页面 | 自定义页面 | 差异程度 |
|------|---------|-----------|---------|
| **页面定义** | 4 个独立文件（`profile_page.dart` 等），硬编码 `ObjectCategoryPage` + `PredefinedObjectSection` 列表 | `ObjectWorkspacePage`，动态从 `UnifiedObject` 树读取 | 🔴 大 |
| **路由** | 独立路由（`/profile`, `/travel`...） | 通用路由（`/objects/:id`） | 🟡 中 |
| **侧边栏** | 硬编码 `NavTile`（`app_sidebar.dart:143-171`） | 动态从 `UnifiedObject` 读取 | 🔴 大 |
| **首页快捷方式** | `_allAvailableActions` 硬编码 | 动态从 `UnifiedObject` 读取 | 🟡 中 |
| **分区渲染** | `PredefinedObjectSection`：带 `displayItemBuilder`/`customFormBuilder`/`title`/`icon` 等硬编码参数 | `ObjectCard`：通用渲染 | 🔴 大 |
| **分区过滤** | `CustomSectionsWidget` 需 `defaultSectionIds` 列表区分"默认"和"自定义" | 无此概念，所有子项一视同仁 | 🔴 大 |
| **分区创建** | `createDefaultSectionsForPage()` / `createDefaultItem()` 专用方法 | `createObject()` 通用方法 | 🟡 中 |
| **页面标题** | `l10n.profileTitle` 等硬编码 | 从 `UnifiedObject.name` 读取 | 🟡 中 |

### 1.2 核心问题

**问题不是"默认分区不能删"，而是"UI 层假设了这些分区一定存在"。**

即使底层数据已通过 Phase 1/2 支持删除默认分区，`ProfilePage` 仍会尝试渲染 4 个固定的 `PredefinedObjectSection`。如果用户删除了 identity section，页面上会留下空白或错误。

---

## 2. 方案选型

### 2.1 方案 A：激进统一（默认页面走 `ObjectWorkspacePage`）

- 删除 `profile_page.dart` / `travel_page.dart` / `financial_page.dart` / `professional_page.dart`
- 默认页面路由统一为 `/objects/:id`
- 侧边栏和首页快捷方式全部动态化
- **优点**：最彻底，零差异，代码最少
- **缺点**：路由变化影响大；`ObjectWorkspacePage` 的渲染质量低于当前默认页面（无 `EntryCardWidget` 特殊展示）

### 2.2 方案 B：渐进动态化（推荐 ✅）

- **保留**默认页面文件和独立路由（用户习惯）
- **动态化**页面内容：不再硬编码 `sections`，改为从 `UnifiedObject` 树读取子分区
- **统一**分区渲染：`PredefinedObjectSection` 的能力下沉到数据层/注册表
- **优点**：改动可控，保留 UX，核心目标达成
- **缺点**：需要设计渲染注册表来替代硬编码的 `displayItemBuilder`

### 2.3 方案 C：最小改动（仅条件渲染）

- 保留现有结构
- `PredefinedObjectSection` 增加条件渲染（分区不存在时显示 `SizedBox.shrink()`）
- 允许添加/删除但硬编码结构不变
- **优点**：改动最小
- **缺点**：不彻底，默认页面仍和自定义页面是两套代码，长期维护成本高

**推荐方案 B。**

---

## 3. 方案 B 详细设计

### 3.1 总体架构

```
Before (当前):
┌─────────────────────────────────────────┐
│ ProfilePage (硬编码)                     │
│  ├─ ObjectCategoryPage                   │
│     ├─ ScanDocumentButton (固定)         │
│     ├─ PredefinedObjectSection(identity) │  ← 硬编码参数
│     ├─ PredefinedObjectSection(contact)  │  ← 硬编码参数
│     ├─ PredefinedObjectSection(idCard)   │  ← 硬编码参数
│     ├─ PredefinedObjectSection(address)  │  ← 硬编码参数
│     └─ CustomSectionsWidget              │  ← 动态，但需过滤 defaultSectionIds
└─────────────────────────────────────────┘

After (目标):
┌─────────────────────────────────────────┐
│ ProfilePage (仅路由和标题映射)            │
│  ├─ UnifiedCategoryPage (通用)           │
│     ├─ PageHeader (ScanDocumentButton 等)│  ← 基于数据条件渲染
│     └─ DynamicSectionList               │  ← 全部子分区动态渲染
│        ├─ SectionRenderer(typeId)        │  ← 根据 typeId 自动选择模板
│        ├─ SectionRenderer(typeId)        │
│        └─ ...                            │
└─────────────────────────────────────────┘
```

### 3.2 实施步骤

#### Step 1: 分区渲染注册表（SectionRendererRegistry）

**目标**：将 `PredefinedObjectSection` 中硬编码的 `displayItemBuilder` / `title` / `icon` 等提取到可扩展的注册表中。

**新增文件**：`lib/presentation/widgets/section_renderer_registry.dart`

```dart
/// 分区渲染配置，替代 PredefinedObjectSection 的硬编码参数
class SectionRenderConfig {
  final String typeId;
  final IconData icon;
  final String Function(BuildContext) nameL10n;
  final Widget Function(BuildContext, UnifiedObject item, Map<String, String> itemMap) itemCardBuilder;
  final int maxVisibleItems;
  final String? scanDocumentTargetType; // OCR 扫描的目标类型，如 'contact'
  // ... 其他渲染参数
}

class SectionRendererRegistry {
  static final Map<String, SectionRenderConfig> _configs = {
    'profile_identity': SectionRenderConfig(...),
    'profile_contact': SectionRenderConfig(...),
    'travel_passport': SectionRenderConfig(...),
    // ... 所有 15 个默认类型
  };

  static SectionRenderConfig? getConfig(String typeId) => _configs[typeId];
}
```

**关键决策**：
- 注册表基于 `typeId` 查找，不是基于 `sectionId`
- 这意味着即使默认分区被删除后用户重新创建同名分区，只要 `typeId` 匹配，渲染就正确
- 自定义分区（`typeId='collection'`）无注册表配置，走通用 `ObjectCard`

#### Step 2: 统一分区列表组件（DynamicSectionList）

**目标**：替代 `ObjectCategoryPage.sections` + `CustomSectionsWidget` 的分割渲染。

**修改文件**：`lib/presentation/widgets/object_category_page.dart`

```dart
class ObjectCategoryPage extends ConsumerWidget {
  final String title;
  final String pageId;
  // 删除：List<Widget> sections
  // 删除：List<String> defaultSectionIds

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final allChildren = ref.watch(childrenProvider(pageId));
    final sections = allChildren
        .where((o) => o.typeId != 'page' && !o.isDeleted)
        .toList();

    return Scaffold(
      appBar: ...,
      body: SingleChildScrollView(
        child: Column(
          children: [
            // 页面级操作按钮（条件渲染）
            _PageActions(pageId: pageId),
            // 统一渲染所有子分区
            for (final section in sections)
              _SectionRenderer(section: section),
            // 空状态
            if (sections.isEmpty) _EmptyState(pageId: pageId),
          ],
        ),
      ),
    );
  }
}
```

**关键决策**：
- 不再区分"默认分区"和"自定义分区"
- 所有子分区统一按存储顺序渲染
- 空状态时显示"恢复默认"按钮（调用 `createDefaultSectionsForPage`）

#### Step 3: 分区渲染组件（_SectionRenderer）

**目标**：根据 `typeId` 自动选择渲染方式。

```dart
class _SectionRenderer extends ConsumerWidget {
  final UnifiedObject section;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final config = SectionRendererRegistry.getConfig(section.typeId);

    if (config != null) {
      // 默认类型：使用注册表配置渲染（保留 EntryCardWidget 的精美展示）
      return _RegisteredSectionRenderer(section: section, config: config);
    } else {
      // 自定义类型：使用通用 ObjectCard
      return _GenericSectionRenderer(section: section);
    }
  }
}
```

**关于 `customFormBuilder`**：
- `profile_contact` 的 `customFormBuilder`（email/phone dropdown）比较特殊
- 方案：在 `ObjectEditorPage` 中扩展 `PropertyType` 支持 `dropdown` 类型
- 将 contact type 定义为 Schema 中的一个 `dropdown` 字段
- 这样自定义表单就不需要了，统一走 `ObjectEditorPage`

#### Step 4: 默认页面文件瘦身

**修改 4 个页面文件**：`profile_page.dart`, `travel_page.dart`, `financial_page.dart`, `professional_page.dart`

从：
```dart
class ProfilePage extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    return ObjectCategoryPage(
      title: l10n.profileTitle,
      pageId: DefaultPageIds.profile,
      defaultSectionIds: [DefaultSectionIds.identity, ...],
      sections: [
        ScanDocumentButton(...),
        _IdentitySection(...),
        _ContactSection(...),  // 含 customFormBuilder
        _IdentityDocumentsSection(...),
        _AddressesSection(...),
      ],
    );
  }
}
```

变为：
```dart
class ProfilePage extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    return UnifiedCategoryPage(
      title: l10n.profileTitle,
      pageId: DefaultPageIds.profile,
    );
  }
}
```

**删除**：
- `_IdentitySection`, `_ContactSection`, `_IdentityDocumentsSection`, `_AddressesSection` 等内部类
- `_ContactForm`, `_CountedTextField` 等自定义表单（迁移到 `ObjectEditorPage` 的 dropdown 支持）

#### Step 5: 侧边栏动态化

**修改文件**：`lib/presentation/widgets/app_sidebar.dart`

**从**：
```dart
// Default pages (硬编码)
NavTile(icon: Icons.person_outline, label: 'Profile', ...),
NavTile(icon: Icons.flight_outlined, label: 'Travel', ...),
NavTile(icon: Icons.account_balance_outlined, label: 'Financial', ...),
NavTile(icon: Icons.work_outline, label: 'Professional', ...),
// Custom pages (动态)
...dynamic pages...
```

**到**：
```dart
// 所有 page 类型统一动态读取，按固定顺序排列
// 默认页面始终在前，自定义页面在后
final allPages = ref.watch(allPagesProvider); // 包含默认和自定义
for (final page in allPages) {
  NavTile(
    icon: UnifiedObjectService.getIconFromName(page.iconName),
    label: page.name,
    onTap: () => _navigateToPage(page),
  );
}
```

**关键决策**：
- 默认页面和自定义页面使用相同的导航方式
- 但默认页面使用独立路由（`/profile` 等）而非 `/objects/:id`
- 需要维护 pageId → route 的映射表

#### Step 6: 首页快捷方式动态化

**修改文件**：`lib/presentation/pages/home_page.dart`

**从**：`_allAvailableActions` 硬编码默认页面

**到**：默认页面从 `UnifiedObject` 动态生成，与自定义页面统一逻辑

```dart
final allPages = ref.read(unifiedObjectProvider).objects
    .where((o) => o.typeId == 'page' && !o.isDeleted)
    .toList();

final pageActions = allPages.map((page) => QuickAction(
  icon: UnifiedObjectService.getIconFromName(page.iconName),
  label: page.name,
  route: _routeForPage(page), // pageId → route 映射
  color: _colorForPage(page.name),
)).toList();
```

#### Step 7: 数据迁移

**现有用户数据状态**：Phase 2 已确保默认分区作为 `UnifiedObject` 存在于树中。

**需要确认**：
- 默认页面的 `UnifiedObject` 是否已存在（Phase 2 的 `_createDefaultStructure()` 已创建）
- 默认分区的 `typeId` 是否正确（如 `profile_identity`）
- 自定义分区的 `typeId='collection'` 不受影响

**无需额外迁移。**

#### Step 8: 清理默认/自定义差异代码

**删除/简化**：

| 文件/符号 | 操作 | 说明 |
|-----------|------|------|
| `CustomSectionsWidget` | 删除 | 不再需要"自定义分区"的特殊渲染 |
| `ObjectCategoryPage.defaultSectionIds` | 删除 | 不再区分默认/自定义 |
| `ObjectCategoryPage.sections` | 删除 | 改为动态读取 |
| `_RestoreDefaultsWidget` | 保留但移动 | 空状态时显示 |
| `PredefinedObjectSection` | 保留但重构 | 能力下沉到注册表 |
| `createDefaultSectionsForPage()` | 保留 | 恢复默认按钮仍需调用 |
| `createDefaultItem()` | 合并到 `createObject()` | 统一创建路径 |
| `defaultPageProvider` / `defaultPageItemsProvider` | 删除 | 使用通用 provider |

---

## 4. 风险与缓解

| 风险 | 影响 | 缓解措施 |
|------|------|---------|
| **渲染质量退化** | `EntryCardWidget` 的精美展示（如 identity 的 fullName 突出显示）可能丢失 | 注册表保留 `itemCardBuilder`，默认类型仍使用 `EntryCardWidget`，只是调用方式从硬编码改为查表 |
| **Contact 自定义表单丢失** | Contact 的 email/phone dropdown 是硬编码表单 | 在 `ObjectEditorPage` 中新增 `PropertyType.dropdown` 支持，将 contact type 定义为 dropdown Schema 字段 |
| **ScanDocumentButton 错位** | 当前 ScanDocumentButton 在固定位置（如 Contact 分区上方） | 改为基于数据条件渲染：如果目标分区存在则显示，否则显示通用"扫描文档"按钮让用户选择目标分区 |
| **OCR/MRZ 硬编码 parentId** | `MrzVaultService` / `ScanDocumentButton` 硬编码 `DefaultSectionIds.*` | OCR 导入前检查目标分区是否存在；不存在时弹出分区选择对话框 |
| **动画效果丢失** | 当前各 Section 有 staggered fadeIn/slideX 动画 | 在 `DynamicSectionList` 中统一添加 staggered 动画 |
| **侧边栏默认页面路由** | 默认页面路由（`/profile`）和自定义页面（`/objects/:id`）不同 | 保留映射表：`DefaultPageIds.profile → AppRoutes.profile`；导航时根据 pageId 查表选择路由 |
| **l10n 名称变化** | 默认页面标题从硬编码 l10n 改为 `UnifiedObject.name` | 首次创建时仍使用 l10n 名称写入 `UnifiedObject.name`；用户可重命名 |

---

## 5. 实施顺序建议

```
Step 1: SectionRendererRegistry（渲染注册表）
   ↓
Step 2: UnifiedCategoryPage（统一页面组件）
   ↓
Step 3: ProfilePage 试点（先改一个页面验证）
   ↓
Step 4: 其余 3 个默认页面
   ↓
Step 5: 侧边栏动态化
   ↓
Step 6: 首页快捷方式动态化
   ↓
Step 7: ObjectEditorPage dropdown 支持（替代 Contact 自定义表单）
   ↓
Step 8: 清理旧代码（删除 CustomSectionsWidget、defaultSectionIds 等）
   ↓
Step 9: 测试与回归验证
```

---

## 6. 工作量评估

| 步骤 | 预计文件数 | 复杂度 | 预计时间 |
|------|-----------|--------|---------|
| Step 1: 渲染注册表 | 1 新文件 | 🟡 中 | 0.5 天 |
| Step 2: 统一页面组件 | 1 文件大改 | 🟡 中 | 1 天 |
| Step 3-4: 4 个页面迁移 | 4 文件大改 | 🟡 中 | 1 天 |
| Step 5: 侧边栏动态化 | 1 文件 | 🟢 低 | 0.5 天 |
| Step 6: 首页动态化 | 1 文件 | 🟢 低 | 0.5 天 |
| Step 7: ObjectEditor dropdown | 1-2 文件 | 🟡 中 | 1 天 |
| Step 8: 清理旧代码 | 多个文件 | 🟢 低 | 0.5 天 |
| Step 9: 测试 | - | 🔴 高 | 1-2 天 |
| **总计** | **~10 文件** | | **5-6 天** |

---

## 7. 验收标准

- [ ] Profile 页面可以删除所有 4 个默认分区，页面正常显示为空（或恢复默认按钮）
- [ ] 删除后可以添加自定义分区，功能与自定义页面完全一致
- [ ] 可以恢复默认分区，恢复后的分区 Schema 正确
- [ ] Travel/Financial/Professional 页面同样可自由增删改分区
- [ ] 侧边栏中默认页面和自定义页面统一展示，无视觉差异
- [ ] 首页快捷方式中默认页面和自定义页面统一展示
- [ ] OCR 扫描在目标分区被删除时有优雅降级（提示选择目标分区）
- [ ] 所有现有单元测试通过
