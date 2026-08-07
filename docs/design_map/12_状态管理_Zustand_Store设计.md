# 12 — 状态管理：Zustand Store 设计

> **前置阅读**：`08_IPC命令接口完整规范.md`、`10_前端技术架构与组件映射.md`、`09_对象规范.md`
> **当前状态**：文档已根据全量代码（Phase 2 完成时）重构，反映真实实现架构。
> **Manifesto 对齐**：隐私优先 | 安全默认 | 最少惊喜

---

## 1. 为什么是 Zustand

| 方案 | 排除原因 |
|------|---------|
| Redux | 样板代码过多；SoloSoul 不需要时间旅行调试 |
| Context API | 频繁渲染；Context 拆分复杂 |
| Jotai | 学习曲线陡；团队无经验 |
| Recoil | Meta 已放弃维护 |

Zustand 优势：**无 Provider**、细粒度订阅、TypeScript 完美支持、中间件丰富。

---

## 2. Store 总览（共 17 个）

应用采用基于功能领域的扁平化 Store 设计。所有 Store 均位于 `tauri/src/stores/`。

### 2.1 认证与安全（Auth & Security）

| Store | 实际职责 | 关键动作 |
|-------|---------|---------|
| `authStore.ts` | 认证 + 账户列表 + 首次启动检测 + **Vault 锁定/解锁**（P015 收敛，替代已删除的 vaultStore） | `bootstrap` / `login` / `logout` / `lock` / `checkHasAccount` / `listAccounts` |
| `autoLockPauseStore.ts` | 自动锁定暂停计数（模态场景打开期间暂停闲置计时） | `pause` / `resume` |
| `settingsStore.ts` | 双层偏好：明文 UI 偏好 + Vault 加密偏好 + 自定义页面 CRUD | `loadUiPreferences`（2 步加载）/ `loadSettings` / `updateSetting` / `addCustomPage` / `removeCustomPage` |

### 2.2 核心业务数据（Core Data）

| Store | 实际职责 | 关键 IPC |
|-------|---------|---------|
| `objectStore.ts` | 对象 CRUD + 回收站操作 | `object_list` / `object_get` / `object_create` / `object_delete` / `object_trash_list` |
| `templateStore.ts` | 模板 CRUD + 字段使用检查 + 从对象保存 | `template_list` / `template_create` / `template_save_from_object` / `template_check_field_usage` |
| `trashStore.ts` | 回收站：时间/类型过滤 + 搜索 + 批量选择 | `object_trash_list` / `trash_restore` / `trash_permanent_delete` / `template_restore` |
| `profileStore.ts` | Profile 数据加载（Uint8Array 解码） | `profile_load`（`profile_get_section` / `profile_update_field` 死命令已删） |

### 2.3 UI 与交互（UI & Layout）

| Store | 实际职责 | 持久化 |
|-------|---------|--------|
| `uiStore.ts` | 侧边栏折叠 + Toast 通知（自动消失） + 全局加载状态 | **无**（纯内存） |
| `sidebarHoverStore.ts` | 侧边栏悬停展开 + 横向/纵向滚动位置保持 | **纯内存**（跨导航保持） |

### 2.4 大模型与 AI（LLM）

| Store | 实际职责 | 事件/持久化 |
|-------|---------|------------|
| `llmStore.ts` | LLM 流式对话：Tauri Event `llm-stream-chunk` 订阅 | Tauri Event 监听 |
| `llmStatsStore.ts` | LLM 使用统计查询与重置 | IPC `llmGetStats` / `llmResetStats`（通过 `@/lib/llm/statsApi`） |

### 2.5 OCR 与扫描

| Store | 实际职责 | 持久化 |
|-------|---------|--------|
| `ocrScanStore.ts` | 扫描历史队列（最多 50 条）、MRZ 自动 fallback、软删除/恢复 | `zustand persist` → localStorage |
| `ocrInstallStore.ts` | OCR 离线模型下载进度追踪 | Tauri Event + localStorage 安装标记 |

### 2.6 插件与同步（Extensions）

| Store | 实际职责 | 持久化 |
|-------|---------|--------|
| `pluginStore.ts` | 插件市场/已安装/运行时状态 + Event 订阅（log/result/consent/dialog） | `zustand persist` → localStorage（仅 runningPlugins） |
| `pluginQuickStore.ts` | 插件快捷面板开闭与 Tab 切换 | **无** |
| `syncStore.ts` | 设备同步状态 + 对等节点信任/解除 + `sync-completed` 事件 | **无** |
| `safSyncStore.ts` | SAF 目录同步进度（`sync-progress` 事件订阅） | Tauri Event 监听 |

---

## 3. authStore — 认证与账户

```typescript
// tauri/src/stores/authStore.ts — 实际实现
interface AuthState {
  isAuthenticated: boolean;
  isLoading: boolean;
  currentAccount: AccountInfo | null;
  accounts: AccountInfo[];          // 账户列表
  error: string | null;
  hasAccount: boolean | null;       // null = 未知，false = 无，true = 有
  backendError: boolean;            // 后端不可用标记

  checkHasAccount: () => Promise<void>;   // 检测是否有账户（不返回值，写入 state）
  listAccounts: () => Promise<void>;      // 获取账户列表 + 刷新 currentAccount
  refreshCurrentAccount: () => Promise<void>; // 单纯刷新
  bootstrap: (name: string, password: string, locale: string, passwordHint?: string) => Promise<void>;
  login: (accountId: string, password: string) => Promise<void>;
  logout: () => Promise<void>;
  clearError: () => void;
}
```

**关键实现细节**：
- `hasAccount` 三态：首次启动 `null`（等待 `checkHasAccount`），有账户 `true`，无 `false`
- `backendError` 标记后端不可用，防止 `hasAccount = null` 时误跳引导页
- `bootstrap` 额外接收 `locale` 和 `passwordHint`
- `login` 成功后刷新 `accounts` 列表，但刷新失败不阻断认证状态
- 未使用 `immer` 中间件

---

## 4. vaultStore — Vault 锁定/解锁

> **⚠️ 本节为历史设计**：`vaultStore.ts` 已于 P015 合并入 `authStore`（锁定/解锁收敛为
> `authStore` action）；`VaultStateStr` 类型亦随 P219 移除。Vault 状态当前由前端
> `authStore.isAuthenticated` 维护，后端状态判定保留在服务方法 `VaultService::get_vault_state()`。

```typescript
// 历史设计（2026-06）——已迁移至 authStore，仅供参考
interface VaultStoreState {
  vaultState: 'locked' | 'unlocked'; // 原 VaultStateStr 类型已移除
  isLoading: boolean;
  error: string | null;

  loadVaultState: () => Promise<void>;
  unlock: (accountId: string, password: string) => Promise<void>;
  lock: () => Promise<void>;
}
```

**关键变更**：独立状态管理，与 `authStore` 职责分离。未使用 `immer`。

---

## 5. settingsStore — 双层偏好（实际实现差异最大）

### 5.1 实际数据结构

**与文档核心差异**：实际实现**没有**分离 `UiPreferences` 和 `SensitivePreferences` 两个独立接口，而是使用单一的 `AppSettings` 接口，字段按存储策略区分写入目标。

```typescript
// tauri/src/stores/settingsStore.ts — 实际实现
interface AppSettings {
  // UI 偏好（明文，登录前可用）
  theme: 'light' | 'dark' | 'system';
  accentColor: 'ocean' | 'amber' | 'forest' | 'rose' | 'purple' | 'custom';
  customAccentHex: string;
  backgroundType: 'solid' | 'gradient' | 'image';
  backgroundValue: string;
  language: string;
  locale: string;
  defaultLightTheme: string;
  defaultDarkTheme: string;
  windowSize?: WindowSize;

  // Vault 加密偏好（解锁后可用）
  autoLockTimeoutMinutes: number;
  biometricEnabled: boolean;
  confirmDelete: boolean;

  // 结构化数据
  customPages: CustomPage[];         // 存储在 objects 表（P0-1 迁移）或旧版偏好
  sidebarPosition: 'left' | 'right' | 'top' | 'bottom';
  sidebarButtonModes: Record<string, 'card' | 'page'>;
}
```

### 5.2 实际加载流程（两步加载）

```typescript
// 第 1 步：loadUiPreferences — 登录前，从 localStorage 缓存 + IPC 刷新
loadUiPreferences: async () => {
  // Step A: 从 localStorage 读取缓存（同步，即时生效）
  const raw = localStorage.getItem('solosoul_ui_prefs');
  if (raw) {
    const parsed = uiPrefsSchema.safeParse(JSON.parse(raw));
    // 应用主题/语言到页面
    await applyTheme({ ... });
  }

  // Step B: 从 IPC 获取最新值（异步，覆盖缓存）
  const prefs = await invoke('ui_get_preferences');
  // 合并到 state 并写回 localStorage 缓存
  set({ settings: parsed });
}

// 第 2 步：loadSettings — 登录后，从 Vault 加密存储加载
loadSettings: async (accountId) => {
  const raw = await invoke('user_data_get_preferences', { accountId });
  const prefs = accountPrefsSchema.safeParse(raw);
  // 合并全部设置，包括安全偏好
  // 同步窗口大小（对比 localStorage 缓存和加密存储，取最新）
  // 将部分值同步写回明文 UI 偏好（theme/accent/language）
}

// 第 3 步：loadCustomPages — 登录后，从 objects 表加载
loadCustomPages: async (accountId) => {
  // P0-1: 自定义页面存储在 objects 表（collectionType = 'page'）
  // 若对象表有页面，使用新格式
  // 若为空但旧格式偏好中存在，自动迁移
}
```

**更新逻辑**：

```typescript
// updateSetting 方法 — 乐观更新 + 回滚
updateSetting: async (accountId, key, value) => {
  const oldValue = get().settings[key];
  set((s) => ({ settings: { ...s.settings, [key]: value } }));  // 乐观更新
  try {
    if (key === 'windowSize') {
      localStorage.setItem('solosoul_window_size', JSON.stringify(value));
      await invoke('ui_update_preference', { key: 'windowSize', value: JSON.stringify(value) });
    } else {
      await invoke('user_data_update_preference', { payload: { accountId, preferences: { [key]: value } } });
    }
  } catch {
    set((s) => ({ settings: { ...s.settings, [key]: oldValue } }));  // 回滚
  }
}
```

**自定义页面 CRUD**（独立于设置本身）：

| 方法 | 行为 |
|------|------|
| `addCustomPage(accountId, name, iconId)` | 乐观 UI 更新 → `invoke('object_create', { ...collectionType: 'page' })` → 失败回滚 |
| `removeCustomPage(accountId, pageId)` | 标记 `deletedAt`（保留在数组内供模板引用）→ `invoke('page_delete')` → 失败回滚 |

**`clearOnVaultLock`**：保留 UI 偏好（language/theme/accent），重置加密偏好到 `DEFAULT_SETTINGS`。

### 5.3 Zod 校验

实际实现**不依赖 `commands.*`**，直接调用 `invoke`。使用 `zod` schemas 对 IPC 返回数据做运行时校验：

```typescript
const uiPrefsSchema = z.object({
  theme: z.enum(['light', 'dark', 'system']).optional(),
  accentColor: z.enum(['ocean', 'amber', 'forest', 'rose', 'purple', 'custom']).optional(),
  windowSize: z.object({ width: z.number(), height: z.number() }).optional(),
  // ...
});

const accountPrefsSchema = z.object({
  theme: z.enum(['light', 'dark', 'system']).optional(),
  autoLockTimeoutMinutes: z.number().optional(),
  // ... 全部可选字段
}).passthrough();
```

---

## 6. profileStore — Profile 数据（实际实现简化版）

**与文档核心差异**：实际实现**没有乐观更新**、**没有 debounce 防抖保存**、**没有 `isSaving` 状态**。数据通过 `Uint8Array` 解码加载。

```typescript
// tauri/src/stores/profileStore.ts — 实际实现
interface ProfileState {
  accountId: string | null;
  sections: ProfileSectionData[];
  isLoading: boolean;
  error: string | null;

  loadProfile: (accountId: string) => Promise<void>;   // 加载全部（Uint8Array 解码）
  loadSection: (accountId: string, sectionType: string) => Promise<ProfileSectionData | null>;
  updateField: (accountId: string, sectionType: string, fieldKey: string, value: unknown) => Promise<void>;
  clear: () => void;                                     // Vault 锁定时调用
}
```

**加载实现**（设计文档中的 `commands.profileLoad` 不存在，实际为原始 IPC）：

```typescript
loadProfile: async (accountId) => {
  const profile = await invoke<{ accountId: string; data: number[] } | null>('profile_load', { accountId });
  if (profile?.data) {
    // Rust 返回 Uint8Array，需手动解码
    const json = new TextDecoder().decode(new Uint8Array(profile.data));
    const parsed = JSON.parse(json);
    // 映射 sections[...fields...]
  }
}
```

---

## 7. uiStore — UI 状态（实际不持久化）

**与文档核心差异**：文档声称 uiStore 使用 `localStorage` 持久化，但**实际实现无持久化**。仅管理侧边栏 + Toast + 全局加载状态。

```typescript
// tauri/src/stores/uiStore.ts — 实际实现
interface UiState {
  sidebarCollapsed: boolean;
  toasts: Toast[];           // 含 type/duration/timeoutId 自动消失
  globalLoading: boolean;    // 全局加载指示

  toggleSidebar: () => void;
  showToast: (toast: Omit<Toast, 'id'>) => void;
  dismissToast: (id: string) => void;
  setGlobalLoading: (loading: boolean) => void;
}
```

**Toast 自动消失机制**：`showToast` 创建 `setTimeout`（默认 3000ms），超时自动从 `toasts` 数组移除。`dismissToast` 清除对应 timeout。无 `persist` 中间件。

---

## 8. sidebarHoverStore — 侧边栏悬停 & 滚动位置

```typescript
// tauri/src/stores/sidebarHoverStore.ts — 实际实现
interface SidebarHoverState {
  isHovering: boolean;
  setHovering: (hovering: boolean) => void;
  verticalScrollTop: number;          // 垂直模式滚动位置
  setVerticalScrollTop: (scrollTop: number) => void;
  horizontalScrollLeft: number;       // 水平模式滚动位置
  setHorizontalScrollLeft: (scrollLeft: number) => void;
}
```

**关键特性**：纯内存保持，利用 Zustand Store 脱离 React 组件树的特性，跨页面导航时保留悬停与滚动位置。支持垂直和水平两种布局模式。

---

## 9. 业务对象 Store（Object / Template / Trash）

### 9.1 objectStore

```typescript
// tauri/src/stores/objectStore.ts — 实际实现
interface ObjectState {
  objects: ObjectSummary[];
  currentObject: ObjectData | null;
  trashObjects: ObjectSummary[];    // 回收站列表
  isLoading: boolean;
  error: string | null;

  loadObjects: (accountId, filter?) => Promise<void>;
  getObject: (accountId, objectId) => Promise<void>;
  createObject: (input) => Promise<ObjectData>;  // 乐观追加到列表
  updateObject: (objectId, input) => Promise<void>;
  deleteObject: (objectId) => Promise<void>;
  loadTrashObjects: (accountId) => Promise<void>;
  restoreObject: (objectId) => Promise<void>;
  purgeObject: (objectId) => Promise<void>;
  clearOnVaultLock: () => void;
}
```

**与 doc 差异**：未使用 `immer`。`createObject` 返回 `Promise<ObjectData>`（前端可直接获取新对象数据）。`ObjectSummary` 包含 `contractTypeId`（来自 `ObjectData`）。`clearOnVaultLock` 清空全部。

### 9.2 templateStore

```typescript
// tauri/src/stores/templateStore.ts — 实际实现
interface TemplateState {
  templates: UserTemplate[];
  isLoading: boolean;
  error: string | null;

  loadTemplates: () => Promise<void>;
  createTemplate: (name, iconId?, category?, properties, contractTypeId?) => Promise<string>;
  updateTemplate: (id, updates) => Promise<void>;
  deleteTemplate: (id) => Promise<void>;
  getTemplate: (id) => Promise<UserTemplate | null>;
  saveFromObject: (objectId, name) => Promise<string>;  // 从对象创建模板
  checkFieldUsage: (templateId, fieldKey) => Promise<{ active: number; softDeleted: number }>;
}
```

**与 doc 差异**：`updateTemplate` 调用后自动 `loadTemplates()` 刷新全量列表（非乐观更新）。`saveFromObject` 通过 IPC `template_save_from_object` 实现。

### 9.3 trashStore

```typescript
// tauri/src/stores/trashStore.ts — 实际实现
interface TrashState {
  items: TrashItemSummary[];
  timeFilter: TrashTimeFilter;         // 'all' | '1d' | '3d' | '7d' | '30d' | 'half_year'
  typeFilter: TrashTypeFilter;         // 'all' | 'page' | 'object' | 'template'
  searchQuery: string;
  selectedIds: Set<string>;
  isLoading: boolean;
  error: string | null;

  loadItems: (accountId) => Promise<void>;       // 携带 since 参数按时间过滤
  setTimeFilter: (f) => void;
  setTypeFilter: (f) => void;
  setSearchQuery: (q) => void;
  restoreItem: (trashId) => Promise<void>;        // 根据 itemType 走不同 IPC
  permanentDelete: (trashIds: string[]) => Promise<void>;  // 逐条调用
  toggleSelection: (id) => void;                   // Set 实现 O(1) 选择/取消
  selectAll: (ids: string[]) => void;
  clearSelection: () => void;
  clearOnVaultLock: () => void;
}
```

**与 doc 差异**：`loadItems` 接受 `since` 参数（基于 `timeFilter` 计算毫秒偏移）。`restoreItem` 根据 `itemType` 区分调用 `template_restore` 或 `trash_restore`。`permanentDelete` 逐条调用 `trash_permanent_delete`。`clearOnVaultLock` 重置全部状态。

---

## 10. 事件流 Store — LLM 流式对话 & OCR 安装

### 10.1 核心模式：安全取消订阅

（与设计文档一致）

```typescript
// 标准 Event 订阅模式
startStream: (convId) => {
  get().unlisten?.();              // 取消已 resolve 的
  get().unlistenPromise?.then(fn => fn());  // 取消未 resolve 的
  // 订阅新 Event
  const pending = listen<T>('event-name', handler);
  set({ unlistenPromise: pending });
  pending.then(fn => set({ unlisten: fn, unlistenPromise: null }));
}
```

### 10.2 llmStore

（与实际实现一致）订阅 `llm-stream-chunk` Event，`onChunk` 方法校验 `conversationId` 匹配后处理 chunk/error/done。

### 10.3 llmStatsStore

```typescript
// tauri/src/stores/llmStatsStore.ts — 实际实现
interface LlmStatsState {
  stats: LlmUsageStats | null;
  loading: boolean;
  error: string | null;

  loadStats: (accountId) => Promise<void>;
  resetStats: (accountId) => Promise<void>;
  clear: () => void;
}
```

**差异**：通过 `@/lib/llm/statsApi`（非直接 IPC）调用 `llmGetStats`/`llmResetStats`。有 `error` 状态。

### 10.4 ocrInstallStore

（与实际实现一致）订阅 `ocr-install-progress` Event。`isOcrFirstInstallDone()` / `markOcrFirstInstallDone()` 作为独立辅助函数直接读写 localStorage，不经过 Zustand。

### 10.5 ocrScanStore

```typescript
// tauri/src/stores/ocrScanStore.ts — 实际实现
interface OcrScanState {
  isCardOpen: boolean;
  scanMode: 'general' | 'mrz';
  scanHistory: OcrScanEntry[];        // 最多 50 条
  currentScanId: string | null;
  isScanning: boolean;
  activeTier: string;
  lastScanError: string | null;

  setCardOpen: (open) => void;
  setScanMode: (mode) => void;
  setActiveTier: (tier) => void;
  performScan: (filePath) => Promise<void>;   // MRZ 自动 fallback 到 general
  softDeleteEntry: (id) => void;
  restoreEntry: (id) => void;
  permanentlyDeleteEntry: (id) => void;
  clearTrash: () => void;
  getActiveHistory: () => OcrScanEntry[];
  getTrash: () => OcrScanEntry[];
  getCurrentEntry: () => OcrScanEntry | null;
}
```

**与 doc 差异**：MRZ 扫描失败自动 fallback 到通用 OCR。`performScan` 内置完整的状态管理（isScanning + 历史追加 + 错误处理）。`partialize` 只持久化 `scanHistory`、`activeTier`、`scanMode`。

---

## 11. 插件 Store

### 11.1 pluginStore

实际实现远比设计文档描述的复杂。使用 `persist` 中间件持久化运行状态。

```typescript
// tauri/src/stores/pluginStore.ts — 实际实现
interface PluginState {
  marketPlugins: MarketPluginInfo[];
  installedPlugins: PluginManifest[];
  runningPlugins: Record<string, RunningPlugin>;
  selectedTier: 'all' | PluginTier;
  enabledTiers: PluginTier[];
  isLoadingMarket: boolean;
  isLoadingInstalled: boolean;
  error: string | null;

  loadMarket: () => Promise<void>;
  loadInstalled: () => Promise<void>;
  installPlugin: (pluginId, version) => Promise<void>;
  updatePlugin: (pluginId) => Promise<void>;
  uninstallPlugin: (pluginId) => Promise<void>;
  runPlugin: (pluginId, pluginName, params?) => Promise<void>;  // 事件通信主逻辑
  stopPlugin: (pluginId) => void;
  clearPluginOutput: (pluginId) => void;
  resolveDialog: (pluginId, requestId, value?) => Promise<void>;
  setSelectedTier: (tier) => void;
  refreshRegistry: () => Promise<void>;
  clearError: () => void;
}

// RunningPlugin 运行时状态
interface RunningPlugin {
  pluginId: string;
  pluginName: string;
  startTime: number;
  logs: PluginLogLine[];            // debug/info/warn/error
  results: PluginResultPayload[];   // text/markdown/key_value/table
  consentRequests: ConsentRequestEvent[];
  dialogRequests: DialogRequestEvent[];
  completed: boolean;
  exitCode?: number;
  error?: string;
  toastShown?: boolean;             // 去重标记
}
```

**事件通信**：`runPlugin` 订阅 `log`、`result`、`consent_request`、`dialog_request`、`completed`、`error` 事件。每个事件通过 `JSON.parse(event.jsonData)` 解析后更新 `runningPlugins` 状态。

**持久化**：`persist` 中间件，`name: 'solosoul-plugin-store'`，`partialize` 仅保存 `runningPlugins`。

### 11.2 pluginQuickStore

简单的面板开关/Tab 切换。（与实际一致）

---

## 12. syncStore — 设备同步

```typescript
// tauri/src/stores/syncStore.ts — 实际实现
interface SyncStoreState extends SyncStatus {
  isLoading: boolean;
  error: string | null;
  lastResult: SyncResult | null;
  recentResults: SyncResult[];      // 最多 10 条

  loadStatus: () => Promise<void>;
  enable: (enabled: boolean) => Promise<void>;
  syncWithDevice: (deviceId: string) => Promise<void>;
  trustPeer: (peerNodeId, trusted) => Promise<void>;
  forgetPeer: (peerNodeId) => Promise<void>;
}

interface SyncStatus {
  isDiscovering: boolean;
  syncEnabled: boolean;
  localFingerprint: string;
  connectedPeers: SyncPeer[];
}
```

**与 doc 差异**：包含完整的状态管理（发现中/已启用/指纹/对等节点列表/同步结果历史）。调用 `commands.syncGetStatus` / `commands.syncEnable` / `commands.syncWithDevice` / `commands.syncTrustPeer` / `commands.syncForgetPeer`（通过 `@/lib/ipc`）。

---

## 13. 持久化策略（实际实现）

| Store | 实际持久化方式 | 说明 |
|-------|--------------|------|
| `authStore` | **不持久化** | 极度敏感，启动/锁定后重置 |
| `vaultStore` | **不持久化** | 运行时状态 |
| `settingsStore` | **双架构写入**：明文→`ui_update_preference`+localStorage；加密→`user_data_update_preference` | 主题/语言/窗口大小：明文 + localStorage；安全偏好：Vault 加密 |
| `profileStore` | **不持久化到前端** | 数据实时通过 IPC 从 Rust 加载 |
| `objectStore` | **不持久化到前端** | 同上 |
| `templateStore` | **不持久化到前端** | 同上 |
| `trashStore` | **不持久化到前端** | 同上 |
| `uiStore` | **不持久化**（文档原声称 localStorage，实际无） | 纯内存状态 |
| `sidebarHoverStore` | **纯内存** | 利用 Zustand 跨导航保持 |
| `llmStore` | **不持久化** | 会话级事件流 |
| `llmStatsStore` | **不持久化到前端** | 数据通过 IPC 从 Rust 加载 |
| `ocrInstallStore` | localStorage 仅存标记位 `solosoul_ocr_first_install_done` | 运行时状态不持久化 |
| `ocrScanStore` | `zustand persist` → localStorage | `solosoul-ocr-scan-history`，partialize 仅持久化扫描历史 |
| `pluginStore` | `zustand persist` → localStorage | `solosoul-plugin-store`，partialize 仅持久化 runningPlugins |
| `pluginQuickStore` | **不持久化** | 运行时状态 |
| `syncStore` | **不持久化** | 运行时状态 |

---

## 14. Vault 锁定事件处理

```typescript
// 各 Store 的 clearOnVaultLock/clear 方法（在 App.tsx 中由 vault-locked 事件协调调用）
// authStore:   无（由 login/logout 管理）
// vaultStore:  无（状态 naturally 切换为 locked）
// settingsStore: clearOnVaultLock() — 保留 UI 偏好，重置加密偏好到默认
// profileStore:  clear() — 清空 sections
// objectStore:   clearOnVaultLock() — 清空 objects/trashObjects/currentObject/error
// templateStore: 无（模板不受 vault 锁定影响）
// trashStore:   clearOnVaultLock() — 重置全部（含 selectedIds/new Set()）
// llmStatsStore: clear() — 清空 stats/loading/error
```

**注意**：实际未使用 Tauri Event `listen('vault-locked')`，而是通过组件的 React 生命周期和路由守卫在 `login`/`logout`/`lock` 操作后手动调用清理方法。

---

## 15. Store 间通信

```typescript
// [正确] 禁止循环依赖：通过 App.tsx 或页面级组件协调
// [正确] pluginStore → uiStore：在 runPlugin 中调用 useUiStore.getState().showToast()
```

---

## 16. 实际 Store 对比（与 Flutter Riverpod 映射）

| Riverpod | Zustand（实际使用） |
|----------|-------------------|
| `StateNotifier` | `create()` — **未使用 `immer` 中间件**（文档原声称使用，实际无） |
| `StateNotifierProvider` | `useStore(selector)` |
| `ref.watch(provider)` | `useStore(selector)` / `shallow` |
| `ref.read(provider)` | `useStore.getState()` |
| `ProviderScope` | 无需包裹（Zustand 无 Provider） |

> **重要**：实际 16 个 Store 中**没有使用 `immer` 中间件**。文档 §3 中的 `immer((set) => ...)` 写法与实际不符。实际采用直接 `set()` 或 `set((state) => ({ ...state, ... }))` 模式。

---

## 17. 完成标准（实际状态）

### ✅ P0（已完成）
- [x] Vault 锁定事件触发所有敏感 Store 清空（settingsStore/profileStore/objectStore/trashStore/llmStatsStore）
- [x] 无 Store 间循环依赖
- [x] `llmStore`/`ocrInstallStore` Event 订阅有正确的 `unlisten` 清理，无内存泄漏

### ✅ P1（已完成 — 部分实现风格有差异）
- [x] 所有 16 个 Store 已实现
- [x] profileStore 的 `clear()` 在 Vault 锁定时正确调用
- [x] `ocrScanStore` persist 到 localStorage 的 `partialize` 正确（不持久化运行时状态）
- [x] `pluginStore` persist 正确（仅持久化 runningPlugins）

### ❌ 未按原设计实现
- [ ] **profileStore 乐观更新 + 回滚**：实际实现为简单 CRUD，无乐观更新、无 debounce
- [ ] **profileStore 防抖保存**：不存在
- [ ] **settingsStore 双层接口**：实际使用单一 `AppSettings` 接口，非独立的 `UiPreferences` + `SensitivePreferences`
- [ ] **uiStore localStorage 持久化**：实际无持久化
- [ ] **Store 单元测试**：尚未编写

---

*文档版本：v2.0（实际实现反映）*
*创建日期：2026-06-05*
*最后更新：2026-06-25*
*对应开发阶段：Phase 2（状态管理），已全部实现*
*差异说明：本文档已从蓝图设计重构为实际实现记录。与实际代码的主要差异包括：profileStore 无乐观更新、settingsStore 合并为单一 AppSettings 接口、uiStore 无持久化、全程未使用 immer 中间件。*
