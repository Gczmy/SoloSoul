# 12 — 状态管理：Zustand Store 设计

> **前置阅读**：`08_IPC命令接口完整规范.md`、`10_前端技术架构与组件映射.md`
> **Manifesto 对齐**：隐私优先 | 安全默认 | 最少惊喜
> **源文档**：`tauri_refactor/状态管理方案.md`

---

## 1. 为什么是 Zustand

| 方案 | 排除原因 |
|------|---------|
| Redux | 样板代码过多；SoloSoul 不需要时间旅行调试 |
| Context API | 频繁渲染；Context 拆分复杂 |
| Jotai | 学习曲线陡；团队无经验 |
| Recoil | Meta 已放弃维护 |

Zustand 优势：**无 Provider**、细粒度订阅、TypeScript 完美支持、Immer 集成、中间件丰富。

---

## 2. Store 总览（共 16 个）

应用采用基于功能领域的扁平化 Store 设计。

### 核心数据与账户（Core & Data）

| Store | 职责 | IPC 依赖 |
|-------|------|----------|
| `authStore.ts` | 认证 + 会话状态管理 | `authLogin`、`authLogout` |
| `vaultStore.ts` | Vault 锁定/解锁/账户切换 | `vaultLock`、`vaultUnlock` |
| `profileStore.ts` | Profile 数据 + 乐观更新 + 500ms debounce | `profileUpdateField` |
| `settingsStore.ts` | 双层偏好：明文 UI 偏好 + Vault 加密敏感偏好 | `loadUiPreferences`、`userDataGetPreferences` |
| `objectStore.ts` | 核心数据对象 CRUD（💡 由原 `unifiedObjectStore` 更名） | `object_list`、`object_get`、`object_create`、`object_update`、`object_delete` |
| `templateStore.ts` | 模板 CRUD、字段使用检查、从对象保存模板 | `template_list`、`template_create`、`template_update`、`template_delete`、`template_save_from_object` |
| `trashStore.ts` | 回收站状态（时间/类型过滤、搜索、批量选择/删除/恢复） | `object_trash_list`、`trash_restore`、`trash_permanent_delete` |

### 工具与外部通信（Tools & Events）

| Store | 职责 | 事件/持久化 |
|-------|------|------------|
| `llmStore.ts` | LLM 流式对话状态，订阅 Tauri Event `llm-stream-chunk` | Tauri Event 监听 |
| `llmStatsStore.ts` | LLM 使用统计与配额管理 | IPC `llmGetStats`/`llmResetStats` |
| `ocrInstallStore.ts` | OCR 离线模型下载进度追踪 | Tauri Event `ocr-install-progress` |
| `ocrScanStore.ts` | OCR 扫描历史队列（最多 50 条）、MRZ fallback、软删除/恢复 | `zustand persist` → localStorage |

### UI 交互与视图级状态（UI & Layout）

| Store | 职责 | 持久化 |
|-------|------|--------|
| `uiStore.ts` | 侧边栏基础状态、Toast、Modal、Loading | localStorage（仅非敏感 UI 状态） |
| `sidebarHoverStore.ts` | 侧边栏悬停展开 + 滚动位置，跨页面导航保持 | 纯内存（脱离组件树保持） |
| `pluginQuickStore.ts` | 插件快捷面板开闭与 Tab 切换（all/installed/running） | 无 |

### 插件与同步（Extensions）

| Store | 职责 |
|-------|------|
| `pluginStore.ts` | 核心插件生命周期管理与运行状态 |
| `syncStore.ts` | 设备同步状态指示（mDNS + Noise） |

> **废弃说明**：原计划的 `searchStore.ts` 在实现中未独立成 Store，搜索状态收敛至页面组件级 `useState`。`sensitivityStore.ts` 未独立，敏感度作为数据属性收敛至 IPC 层与 `objectStore`。

---

## 3. authStore

```typescript
interface AuthState {
  isAuthenticated: boolean;
  isLoading: boolean;
  currentAccount: AccountInfo | null;
  error: string | null;

  checkHasAccount: () => Promise<boolean>;
  bootstrap: (name: string, password: string) => Promise<void>;
  login: (accountId: string, password: string) => Promise<void>;
  logout: () => Promise<void>;
  clearError: () => void;
}

export const useAuthStore = create<AuthState>()(
  immer((set) => ({
    isAuthenticated: false, isLoading: false,
    currentAccount: null, error: null,

    login: async (accountId, password) => {
      set({ isLoading: true, error: null });
      try {
        const account = await commands.authLogin(accountId, password);
        set({ isAuthenticated: true, currentAccount: account, isLoading: false });
      } catch (err) {
        set({ error: String(err), isLoading: false });
      }
    },

    logout: async () => {
      await commands.authLogout();
      set({ isAuthenticated: false, currentAccount: null });
    },
    // ...
  }))
);
```

---

## 4. profileStore（含乐观更新 + 回滚）

```typescript
interface ProfileStoreState {
  profile: ProfileData | null;
  isLoading: boolean;
  isSaving: boolean;
  error: string | null;

  loadProfile: (accountId: string) => Promise<void>;
  updateField: (sectionType: string, fieldKey: string, value: FieldValue) => Promise<void>;
  clearOnVaultLock: () => void;  // [正确] Vault 锁定时清空
}

// 乐观更新模式
updateField: async (sectionType, fieldKey, value) => {
  const current = get().profile;  // 快照当前状态
  if (!current) return;

  // 1. 乐观更新（立即更新 UI）
  set((state) => {
    state.profile!.sectionData[sectionType][fieldKey] = value;
    state.isSaving = true;
  });

  try {
    // 2. 通过 IPC 持久化到 Rust
    await commands.profileUpdateField(accountId, sectionType, fieldKey, value);
    set({ isSaving: false });
  } catch (err) {
    // 3. 失败时回滚
    set((state) => {
      state.profile = current;  // 恢复到快照
      state.isSaving = false;
      state.error = String(err);
    });
  }
}
```

---

## 5. settingsStore（关键变更）

### 5.1 双层存储设计

settingsStore 同时管理**非敏感 UI 偏好**（明文存储，登录前可读）和**敏感偏好**（Vault 加密存储，解锁后可读）。

```typescript
// 非敏感 UI 偏好：明文存储于 ui_preferences.json，登录前即可读取
interface UiPreferences {
  language: 'zh-CN' | 'en-US';
  theme: ThemeConfig;
  accentColor: 'ocean' | 'amber' | 'forest' | 'rose' | 'custom';
  customAccentHex?: string;
  sidebarWidth: number;
  sidebarCollapsed: boolean;
  windowSize: { width: number; height: number };
}

// 敏感偏好：加密存储于 Vault 内的 preferences.enc
interface SensitivePreferences {
  autoLockTimeout: number;           // 自动锁定超时（秒）
  biometricEnabled: boolean;         // 生物识别启用状态
  trashRetentionPeriod: TrashRetentionPeriod;
  historyRetentionPolicy: HistoryRetentionPolicy;
  passwordHint?: string;             // 密码提示词
  // ... 其他安全相关偏好
}

interface SettingsState {
  uiPreferences: UiPreferences;
  sensitivePreferences: SensitivePreferences | null;  // null = Vault 未解锁
  isLoading: boolean;
}
```

### 5.2 启动加载流程（修复 Bug 2 时序问题）

```typescript
export const useSettingsStore = create<SettingsState>()(
  immer((set, get) => ({
    uiPreferences: DEFAULT_UI_PREFERENCES,
    sensitivePreferences: null,
    isLoading: false,

    // 第 1 步：应用启动时立即加载（登录前）
    loadUiPreferences: async () => {
      try {
        const prefs = await commands.loadUiPreferences();
        set({ uiPreferences: { ...DEFAULT_UI_PREFERENCES, ...prefs } });
        // 立即应用主题和语言
        applyTheme(prefs.theme);
        await i18next.changeLanguage(prefs.language || detectSystemLanguage());
      } catch {
        // 首次启动：ui_preferences.json 不存在，使用系统检测
        const systemLang = detectSystemLanguage();
        set({
          uiPreferences: { ...DEFAULT_UI_PREFERENCES, language: systemLang }
        });
        applyTheme(DEFAULT_UI_PREFERENCES.theme);
        await i18next.changeLanguage(systemLang);
      }
    },

    // 第 2 步：用户登录后加载（Vault 解锁后）
    loadSensitivePreferences: async () => {
      const prefs = await commands.userDataGetPreferences();
      set({ sensitivePreferences: prefs });
    },

    // 更新非敏感 UI 偏好
    updateUiPreference: async (key, value) => {
      const oldValue = get().uiPreferences[key];
      set((state) => { state.uiPreferences[key] = value; });
      try {
        await commands.saveUiPreference(key, value);
        // 即时生效
        if (key === 'language') await i18next.changeLanguage(value);
        if (key === 'theme') applyTheme(value);
      } catch (err) {
        set((state) => { state.uiPreferences[key] = oldValue; });
        throw err;
      }
    },

    // 更新敏感偏好
    updateSensitivePreference: async (key, value) => {
      const oldValue = get().sensitivePreferences?.[key];
      set((state) => { if (state.sensitivePreferences) state.sensitivePreferences[key] = value; });
      try {
        await commands.userDataUpdatePreference(key, value);
      } catch (err) {
        set((state) => { if (state.sensitivePreferences) state.sensitivePreferences[key] = oldValue; });
        throw err;
      }
    },

    clearOnVaultLock: () => {
      set({ sensitivePreferences: null });  // 仅清空敏感偏好，UI 偏好保留
    },
  }))
);
```

### 5.3 [错误] 禁止：localStorage 持久化用户偏好

```typescript
// [错误] 错误：所有偏好都存 localStorage
export const useSettingsStore = create(
  persist(immer(...), {
    name: 'solosoul-settings',
    storage: createJSONStorage(() => localStorage),  // 敏感偏好明文暴露！
  })
);

// [错误] 错误：所有偏好都存 Vault（导致登录页无法读取主题/语言）
loadSettings: async () => {
  const settings = await commands.userDataGetPreferences(); // ❌ 需要 Vault 解锁
  applyTheme(settings.theme);  // ❌ 登录页主题无法应用
}

// [正确] 正确：非敏感偏好明文存储，敏感偏好 Vault 加密
loadUiPreferences: async () => {
  const prefs = await commands.loadUiPreferences();  // ✅ 无需解锁 Vault
  applyTheme(prefs.theme);  // ✅ 登录页即可正确显示主题
}
```

---

## 6. 持久化策略

所有 Store 根据数据敏感度采用严格分级的持久化策略：

| Store | 持久化方式 | 理由与规范 |
|-------|-----------|------|
| `authStore` / `vaultStore` | **不持久化** | 极度敏感状态，每次启动或锁定后重置 |
| `profileStore` / `objectStore` / `templateStore` / `trashStore` / `llmStatsStore` | **不持久化到前端** | 数据实时经过 Rust IPC；Vault 锁定时通过 `clearOnVaultLock` 清理前端内存快照 |
| `settingsStore` | **双架构存储** | 非敏感（语言/主题）→ `ui_preferences.json`；敏感偏好（自动锁定超时/生物识别/回收站保留期）→ Vault 加密；Zustand 本身禁用中间件持久化 |
| `uiStore` | `localStorage` | 保存侧边栏折叠等纯 UI 状态（非敏感） |
| `ocrScanStore` | `localStorage`（`zustand persist`） | `solosoul-ocr-scan-history`：仅保存最多 50 条本地扫描历史，非敏感数据 |
| `ocrInstallStore` | `localStorage`（原生 API） | 仅存标记位 `solosoul_ocr_first_install_done` 判别首次下载，不完整持久化状态 |
| `llmStore` / `ocrInstallStore` | **不持久化** | 事件流状态（streaming/progress）会话级别临时数据，重启后无需恢复 |
| `sidebarHoverStore` | **纯内存保持** | 不写入磁盘；利用 Zustand Store 脱离 React 组件树的特性，跨页面导航时保留悬停与滚动位置 |
| `pluginQuickStore` / `syncStore` / `pluginStore` | **不持久化** | 运行时状态，会话级别临时数据 |

---

## 7. Vault 锁定事件处理

```typescript
// App.tsx
useEffect(() => {
  const unsubscribe = listen('vault-locked', () => {
    // 清空所有敏感 Store 的内存状态
    useSettingsStore.getState().clearOnVaultLock();
    useProfileStore.getState().clearOnVaultLock();
    useObjectStore.getState().clearOnVaultLock();
    useTrashStore.getState().clearOnVaultLock();
    useLlmStatsStore.getState().clear();
    navigate('/login');
  });
  return () => { unsubscribe.then(f => f()); };
}, []);
```

## 8. 事件流与长连接 Store（LLM / OCR 安装）

`llmStore` 和 `ocrInstallStore` 通过 Tauri Event 订阅后端推送，是事件驱动的特殊 Store 模式。

### 8.1 核心模式：安全取消订阅

Event 订阅必须同时管理 `unlistenFn`（同步句柄）和 `unlistenPromise`（异步 Promise），防止热重载或重复调用导致监听器叠加：

```typescript
interface EventStreamState {
  isStreaming: boolean;
  unlisten: UnlistenFn | null;        // 已 resolve 的取消句柄
  unlistenPromise: Promise<UnlistenFn> | null;  // 尚未 resolve 的 Promise

  startListening: () => void;
  stopListening: () => void;
}

// startStream 必须先取消旧监听再订阅新监听
startStream: (convId) => {
  const state = get();
  state.unlisten?.();                   // 取消已 resolve 的
  state.unlistenPromise?.then(fn => fn());  // 取消未 resolve 的
  
  const pending = listen<T>('event-name', handler);
  set({ unlistenPromise: pending });
  pending.then(fn => set({ unlisten: fn, unlistenPromise: null }));
}
```

### 8.2 llmStore — 流式 AI 对话

```typescript
interface LlmState {
  isStreaming: boolean;
  streamingConvId: string | null;
  streamBuffer: string;          // 累积的流式文本
  streamError: string | null;

  startStream: (conversationId: string) => void;
  onChunk: (payload: LlmStreamPayload) => void;
  stopStream: () => void;
  reset: () => void;
}
```

- 订阅 Tauri Event `llm-stream-chunk`，payload 携带 `{ conversationId, chunk, isDone, error? }`
- `onChunk` 校验 `conversationId` 匹配后追加 chunk 或处理 error/done
- `stopStream`/`reset` 均需显式调用 `unlisten?.()` 清理监听器

### 8.3 llmStatsStore — 使用统计

```typescript
interface LlmStatsState {
  stats: LlmUsageStats | null;
  loading: boolean;
  loadStats: (accountId: string) => Promise<void>;
  resetStats: (accountId: string) => Promise<void>;
  clear: () => void;  // Vault 锁定时调用
}
```

### 8.4 ocrInstallStore — 模型下载进度

```typescript
interface OcrInstallState {
  isInstalling: boolean;
  progress: number;  // 0–100
  error: string | null;
  startListening: () => void;
  stopListening: () => void;
  reset: () => void;
}
```

- 订阅 Tauri Event `ocr-install-progress`，payload 携带 `{ tier, progress, done, error? }`
- 下载完成时 `markOcrFirstInstallDone()` 写入 localStorage 标记位，供首次启动引导判断
- 辅助函数 `isOcrFirstInstallDone()` / `markOcrFirstInstallDone()` 直接从 localStorage 读取（不经过 Zustand）

---

## 9. 业务对象 Store（Object / Template / Trash）

### 9.1 objectStore — 核心数据对象 CRUD

> ⚠️ 由原 `unifiedObjectStore` 更名而来。

```typescript
interface ObjectState {
  objects: ObjectSummary[];        // 列表视图摘要
  currentObject: ObjectData | null; // 当前打开的完整对象
  trashObjects: ObjectSummary[];   // 回收站列表
  isLoading: boolean;
  error: string | null;

  loadObjects: (accountId, filter?) => Promise<void>;
  getObject: (accountId, objectId) => Promise<void>;
  createObject: (input) => Promise<ObjectData>;
  updateObject: (objectId, input) => Promise<void>;
  deleteObject: (objectId) => Promise<void>;
  loadTrashObjects: (accountId) => Promise<void>;
  restoreObject: (objectId) => Promise<void>;
  purgeObject: (objectId) => Promise<void>;
  clearOnVaultLock: () => void;
}
```

- `createObject` 采用乐观插入：返回新对象后立即追加到 `objects` 列表头
- `clearOnVaultLock` 清空 `objects`、`trashObjects`、`currentObject`、`error`
- 对象摘要 `ObjectSummary` 包含 `contractTypeId`（插件合约类型 ID）

### 9.2 templateStore — 模板 CRUD 与字段检查

```typescript
interface TemplateState {
  templates: UserTemplate[];
  isLoading: boolean;
  error: string | null;

  loadTemplates: () => Promise<void>;
  createTemplate: (name, iconId?, category?, properties, contractTypeId?) => Promise<string>;
  updateTemplate: (id, updates) => Promise<void>;
  deleteTemplate: (id) => Promise<void>;
  getTemplate: (id) => Promise<UserTemplate | null>;
  saveFromObject: (objectId, name) => Promise<string>;  // 从已有对象反生成模板
  checkFieldUsage: (templateId, fieldKey) => Promise<{ active: number; softDeleted: number }>;
}
```

- CRUD 操作后自动调用 `loadTemplates()` 刷新列表（除 `deleteTemplate` 用乐观删除）
- `checkFieldUsage` 统计字段在活跃对象和软删除对象中的使用次数，用于安全删除字段前的确认提示
- 模板属性 `TemplateProperty` 定义字段类型（text/number/date/select/multiselect/url/email/phone/file）、是否必填、选项列表等

### 9.3 trashStore — 回收站管理

```typescript
interface TrashState {
  items: TrashItemSummary[];
  timeFilter: TrashTimeFilter;      // 'all' | '1d' | '3d' | '7d' | '30d' | 'half_year'
  typeFilter: TrashTypeFilter;      // 'all' | 'page' | 'object' | 'template'
  searchQuery: string;
  selectedIds: Set<string>;         // Zustand 管理批量选择集
  isLoading: boolean;

  loadItems: (accountId) => Promise<void>;
  restoreItem: (trashId) => Promise<void>;
  permanentDelete: (trashIds: string[]) => Promise<void>;
  toggleSelection: (id) => void;
  selectAll: (ids: string[]) => void;
  clearSelection: () => void;
  clearOnVaultLock: () => void;
}
```

- `selectedIds` 使用 `Set<string>` 实现 O(1) 选择/取消，由 Zustand `set()` 创建新 Set 实例触发重渲染
- `restoreItem` 根据 `itemType` 走不同 IPC：`template_restore` 或 `trash_restore`

---

## 10. Store 间通信

```typescript
// [错误] 禁止循环依赖：authStore import profileStore，profileStore import authStore
// [正确] 正确：在 App.tsx 中协调
useEffect(() => {
  if (isAuthenticated && currentAccount) {
    useProfileStore.getState().loadProfile(currentAccount.id);
    useSettingsStore.getState().loadSettings();
  }
}, [isAuthenticated, currentAccount]);
```

---

## 11. Flutter Riverpod → Zustand 映射

| Riverpod | Zustand |
|----------|---------|
| `StateNotifier` | `create()` + `immer` |
| `StateNotifierProvider` | `useStore` hook |
| `ref.watch(provider)` | `useStore(selector)` |
| `ref.read(provider)` | `useStore.getState()` |
| `ProviderScope` | 无需包裹（Zustand 无 Provider） |

---

## 12. 完成标准

### P0（必须）
- [ ] Vault 锁定事件触发所有敏感 Store 清空（settingsStore/profileStore/objectStore/trashStore/llmStatsStore）
- [ ] 无 Store 间循环依赖
- [ ] `llmStore`/`ocrInstallStore` Event 订阅有正确的 `unlisten` 清理，无内存泄漏

### P1（重要）
- [ ] 所有 16 个 Store 单元测试通过
- [ ] 乐观更新 + 回滚逻辑正确（模拟 IPC 失败）
- [ ] profileStore 的防抖保存正确（500ms delay）
- [ ] `ocrScanStore` persist 到 localStorage 的 partialize 正确（不持久化运行时状态）

---

*文档版本：v2.0 (实现后补充)*
*创建日期：2026-06-05*
*最后更新：2026-06-25*
*对应开发阶段：Phase 2（状态管理），已全部实现*
