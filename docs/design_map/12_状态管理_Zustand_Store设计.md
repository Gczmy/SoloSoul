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

## 2. Store 总览

```
src/stores/
├── authStore.ts           # 认证 + 会话状态
├── vaultStore.ts          # Vault 锁定/解锁/账户
├── profileStore.ts        # Profile 数据 + CRUD + 乐观更新
├── unifiedObjectStore.ts  # UnifiedObject 数据
├── uiStore.ts             # 侧边栏、Toast、Modal、Loading
├── settingsStore.ts       # 用户偏好（[错误] 不用 localStorage）
├── pluginStore.ts         # 插件状态
├── syncStore.ts           # 同步状态
├── searchStore.ts         # 搜索状态
└── sensitivityStore.ts    # 敏感度管理状态
```

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

| Store | 持久化方式 | 理由 |
|-------|-----------|------|
| authStore | [错误] 不持久化 | 敏感状态，每次启动重新验证 |
| vaultStore | [错误] 不持久化 | Vault 始终从 Locked 开始 |
| profileStore | [错误] 不持久化 | 从后端加载 |
| **uiStore** | [正确] localStorage（仅非敏感部分） | sidebarCollapsed 等纯 UI 状态 |
| **settingsStore** | [错误] 不持久化到 localStorage | 通过 IPC → Rust Vault 加密存储 |
| **searchStore** | [错误] 不持久化到 localStorage | 搜索历史是敏感数据，通过 IPC 加密存储 |

---

## 7. Vault 锁定事件处理

```typescript
// App.tsx
useEffect(() => {
  const unsubscribe = listen('vault-locked', () => {
    // 清空所有敏感 Store 的内存状态
    useSettingsStore.getState().clearOnVaultLock();
    useSearchStore.getState().clearOnVaultLock();
    useProfileStore.getState().clearOnVaultLock();
    navigate('/login');
  });
  return () => { unsubscribe.then(f => f()); };
}, []);
```

---

## 8. Store 间通信

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

## 9. Flutter Riverpod → Zustand 映射

| Riverpod | Zustand |
|----------|---------|
| `StateNotifier` | `create()` + `immer` |
| `StateNotifierProvider` | `useStore` hook |
| `ref.watch(provider)` | `useStore(selector)` |
| `ref.read(provider)` | `useStore.getState()` |
| `ProviderScope` | 无需包裹（Zustand 无 Provider） |

---

## 10. 完成标准

### P0（必须）
- [ ] settingsStore/searchStore 不写入 localStorage
- [ ] Vault 锁定事件触发所有敏感 Store 清空
- [ ] 无 Store 间循环依赖

### P1（重要）
- [ ] 所有 Store 单元测试通过
- [ ] 乐观更新 + 回滚逻辑正确（模拟 IPC 失败）
- [ ] profileStore 的防抖保存正确（500ms delay）

---

*文档版本：v1.1 (priority-refactored)*
*创建日期：2026-06-05*
*最后更新：2026-06-07*
*对应开发阶段：Phase 2（状态管理）*
