# 10 — 状态管理：Zustand Store 设计

> **前置阅读**：`07_IPC命令接口完整规范.md`、`08_前端技术架构与组件映射.md`
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

### [错误] 禁止：localStorage 持久化用户偏好

```typescript
// [错误] 错误：
export const useSettingsStore = create(
  persist(immer(...), {
    name: 'solosoul-settings',
    storage: createJSONStorage(() => localStorage),  // 明文暴露！
  })
);

// [正确] 正确：通过 IPC → Rust Vault 加密存储
export const useSettingsStore = create<SettingsState>()(
  immer((set, get) => ({
    settings: DEFAULT_SETTINGS,
    isLoading: false,

    loadSettings: async () => {
      // 从 Rust Vault 读取加密的偏好
      const settings = await commands.userDataGetPreferences();
      set({ settings: { ...DEFAULT_SETTINGS, ...settings } });
    },

    updateSetting: async (key, value) => {
      const oldValue = get().settings[key];
      set((state) => { state.settings[key] = value; });
      try {
        await commands.userDataUpdatePreference(key, value);
      } catch (err) {
        set((state) => { state.settings[key] = oldValue; });
        throw err;
      }
    },

    clearOnVaultLock: () => {
      set({ settings: DEFAULT_SETTINGS });  // Vault 锁定后清空
    },
  }))
);
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

- [ ] 所有 Store 单元测试通过
- [ ] 乐观更新 + 回滚逻辑正确（模拟 IPC 失败）
- [ ] settingsStore/searchStore 不写入 localStorage
- [ ] Vault 锁定事件触发所有敏感 Store 清空
- [ ] 无 Store 间循环依赖
- [ ] profileStore 的防抖保存正确（500ms delay）

---

*文档版本：v1.0*
*创建日期：2026-06-05*
*对应开发阶段：Phase 2（状态管理）*
