# ADR-004: IPC 通信与状态管理方案

> **状态**: 已采纳 ✅  
> **决策日期**: 2026-06-04  
> **决策人**: SoloSoul 架构组  
> **影响范围**: 前后端通信方式、状态同步策略、开发体验

---

## 背景

当前 Flutter 使用 `flutter_rust_bridge`（FRB）实现 Dart ↔ Rust FFI 通信。迁移到 Tauri 后，通信方式变为 **Web 前端 ↔ Rust 后端** 的 IPC（Inter-Process Communication）。

需要决策：
1. IPC 调用方式（命令调用 vs 事件流 vs 两者结合）
2. 前端状态管理方案（替代 Riverpod）
3. 前后端状态同步策略

## IPC 通信方案

### Tauri IPC 机制概览

Tauri 提供三种 IPC 方式：

| 方式 | API | 方向 | 适用场景 |
|------|-----|------|---------|
| **Command** | `invoke('cmd', args)` | 前端 → 后端 | 请求-响应（CRUD、计算） |
| **Event** | `emit('event', payload)` / `listen('event', cb)` | 双向 | 通知、广播 |
| **Channel** | `Channel<T>` + `tauri::ipc::Channel` | 后端 → 前端 | 流式数据、进度推送 |

### 决策：Command + Event + Channel 三结合

```
┌─────────────────────────────────────────────────────────────┐
│                        IPC 策略矩阵                          │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Command (invoke) ──────────────────────────────→ 90% 场景  │
│  ├── 页面初始加载数据（Profile、Settings）                   │
│  ├── 用户操作（保存、删除、创建）                            │
│  ├── 计算任务（搜索、排序）                                  │
│  └── 配置读取（LLM、OCR）                                    │
│                                                             │
│  Event (listen/emit) ───────────────────────────→ 5% 场景   │
│  ├── 应用生命周期（前台/后台切换）                           │
│  ├── Vault 状态变更（锁定/解锁广播）                         │
│  ├── 主题切换通知                                            │
│  └── 全局错误广播                                            │
│                                                             │
│  Channel (流式) ────────────────────────────────→ 5% 场景   │
│  ├── OCR 识别进度                                            │
│  ├── 文件扫描进度                                            │
│  ├── LLM 流式响应                                            │
│  ├── 同步进度                                                │
│  └── 备份/导入进度                                           │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### IPC 类型安全

**使用 `tauri-specta` 自动生成 TypeScript 类型**:

```rust
// src-tauri/src/commands/profile.rs
use specta::Type;
use tauri_specta::{collect_commands, ts};

#[derive(Type, serde::Serialize)]
pub struct ProfileData {
    pub full_name: String,
    pub date_of_birth: Option<String>,
}

#[tauri::command]
#[specta::specta]  // 生成类型信息
pub async fn profile_get(
    state: tauri::State<'_, AppState>,
    account_id: String,
) -> Result<ProfileData, String> {
    // ...
}

// 构建时生成 TypeScript 绑定
fn main() {
    ts::export(
        collect_commands![profile_get, profile_update, ...],
        "../src/lib/ipc.ts",
    ).unwrap();
}
```

```typescript
// src/lib/ipc.ts（自动生成）
export const commands = {
  async profileGet(accountId: string): Promise<ProfileData> {
    return await invoke('profile_get', { accountId });
  },
  async profileUpdate(data: ProfileData): Promise<void> {
    return await invoke('profile_update', { data });
  },
};
```

**优势**:
- 类型安全：Rust 类型 ↔ TypeScript 类型自动同步
- 重构安全：修改 Rust 命令签名，TypeScript 编译报错
- 零维护：自动生成，无需手写 IPC 封装

---

## 前端状态管理方案

### 候选方案

#### 方案 A: Zustand（推荐）

```typescript
// src/stores/authStore.ts
import { create } from 'zustand';
import { immer } from 'zustand/middleware/immer';

interface AuthState {
  isAuthenticated: boolean;
  currentAccountId: string | null;
  isLoading: boolean;
  error: string | null;
  
  login: (accountId: string, password: string) => Promise<void>;
  logout: () => Promise<void>;
  lockVault: () => Promise<void>;
}

export const useAuthStore = create<AuthState>()(
  immer((set, get) => ({
    isAuthenticated: false,
    currentAccountId: null,
    isLoading: false,
    error: null,

    login: async (accountId, password) => {
      set({ isLoading: true, error: null });
      try {
        await commands.vaultUnlock(accountId, password);
        set({ isAuthenticated: true, currentAccountId: accountId });
      } catch (err) {
        set({ error: String(err) });
      } finally {
        set({ isLoading: false });
      }
    },

    logout: async () => {
      await commands.vaultLock();
      set({ isAuthenticated: false, currentAccountId: null });
    },

    lockVault: async () => {
      await commands.vaultLock();
      set({ isAuthenticated: false });
    },
  }))
);
```

**优势**:
- **极简 API**: 比 Redux 简单 10 倍
- **TypeScript 完美支持**: 类型推导完整
- **Immer 集成**: 可变语法，不可变数据
- **无 Provider 包裹**: 直接使用 hook
- **持久化插件**: 可持久化到 localStorage（非敏感数据）
- **DevTools 支持**: Redux DevTools 兼容
- **包体积极小**: ~1KB

**劣势**:
- 社区规模小于 Redux
- 某些高级场景（时间旅行）不如 Redux

---

#### 方案 B: Jotai

**优势**:
- 原子化状态，细粒度控制
- React 并发模式友好

**劣势**:
- 学习曲线较陡
- 团队无经验

---

#### 方案 C: Valtio

**优势**:
- 代理式状态管理，直接修改对象
- 非常直观

**劣势**:
- 类型支持不如 Zustand 完善
- 某些场景下响应式不够精确

---

#### 方案 D: Context API + useReducer

**优势**:
- React 内置，零依赖

**劣势**:
- 频繁渲染问题（Context 拆分复杂）
- 代码冗长
- 不适合高频更新

---

### 决策：Zustand

**选择原因**:
1. 团队有 Zustand 使用经验（来自已废弃的 Web UI 项目）
2. API 极简，与 Riverpod StateNotifier 概念最接近
3. TypeScript 支持最佳
4. 包体积极小，不影响 Tauri 包大小

### Store 划分

```
src/stores/
├── authStore.ts        # 认证状态（登录/登出/Vault 锁定）
├── vaultStore.ts       # Vault 状态（锁定/解锁/账户列表）
├── profileStore.ts     # Profile 数据（CRUD 操作）
├── uiStore.ts          # UI 状态（侧边栏、Toast、Modal、Loading）
├── settingsStore.ts    # 应用设置（主题、语言、安全选项）
├── pluginStore.ts      # 插件状态（安装/运行/Consent）
└── syncStore.ts        # 同步状态（发现设备、同步进度）
```

### Store 间依赖处理

```typescript
// 避免循环依赖：通过事件或回调解耦
// authStore.ts
import { useProfileStore } from './profileStore';

export const useAuthStore = create<AuthState>((set, get) => ({
  // ...
  login: async (accountId, password) => {
    await commands.vaultUnlock(accountId, password);
    // 登录成功后加载 Profile
    await useProfileStore.getState().loadProfile(accountId);
    set({ isAuthenticated: true });
  },
}));
```

---

## 前后端状态同步策略

### 原则：Rust 为唯一真理来源

```
┌─────────────────────────────────────────────┐
│            Rust 后端（唯一真理来源）           │
│  ┌─────────┐  ┌─────────┐  ┌─────────────┐ │
│  │  Vault  │  │ Profile │  │  Settings   │ │
│  │  State  │  │  Data   │  │   Data      │ │
│  └────┬────┘  └────┬────┘  └──────┬──────┘ │
│       └─────────────┴──────────────┘        │
│                      │                      │
│              ┌───────┴───────┐              │
│              │  IPC (invoke) │              │
│              └───────┬───────┘              │
└──────────────────────┼──────────────────────┘
                       │
              ┌────────┴────────┐
              │  Zustand Stores │
              │  （前端缓存）    │
              └────────┬────────┘
                       │
              ┌────────┴────────┐
              │  React 组件树    │
              └─────────────────┘
```

### 同步模式

#### 模式 1: 请求-响应（Query）

```typescript
// 页面加载时获取数据
function SettingsPage() {
  const { settings, isLoading, loadSettings } = useSettingsStore();
  
  useEffect(() => {
    loadSettings();
  }, []);
  
  if (isLoading) return <Loading />;
  return <SettingsForm settings={settings} />;
}

// store
loadSettings: async () => {
  set({ isLoading: true });
  const settings = await commands.settingsGet();
  set({ settings, isLoading: false });
}
```

#### 模式 2: 乐观更新（Optimistic Update）

```typescript
// 用户操作后立即更新 UI，后台同步
updateSetting: async (key, value) => {
  const oldValue = get().settings[key];
  
  // 乐观更新
  set(state => {
    state.settings[key] = value;
  });
  
  try {
    await commands.settingsUpdate(key, value);
  } catch (err) {
    // 失败回滚
    set(state => {
      state.settings[key] = oldValue;
    });
    showToast('更新失败');
  }
}
```

#### 模式 3: 事件广播（Event）

```typescript
// Rust 端广播 Vault 锁定事件
// src-tauri/src/state/vault_state.rs
tauri::Manager::emit_all(&app_handle, "vault-locked", ()).unwrap();

// 前端监听
useEffect(() => {
  const unlisten = listen('vault-locked', () => {
    useAuthStore.getState().lockVault();
    useProfileStore.getState().clear();
    navigate('/login');
  });
  return () => unlisten.then(f => f());
}, []);
```

#### 模式 4: 流式更新（Channel）

```typescript
// OCR 识别进度
async function scanDocument(filePath: string) {
  const channel = new Channel<OcrProgress>();
  
  channel.onmessage = (progress) => {
    useOcrStore.getState().setProgress(progress);
  };
  
  await commands.ocrScan({ filePath, channel });
}
```

---

## 安全注意事项

### 密钥绝不通过 IPC

```rust
// ❌ 错误：返回密钥到前端
#[tauri::command]
fn get_encryption_key() -> Vec<u8> {  // 绝对禁止！
    // ...
}

// ✅ 正确：所有密码学运算在 Rust 端完成
#[tauri::command]
fn decrypt_data(ciphertext: Vec<u8>) -> Result<String, String> {
    let key = get_key_from_memory();  // Rust 端持有
    let plaintext = cipher::decrypt(&key, &ciphertext)?;
    Ok(plaintext)
}
```

### 密码验证后立即擦除

```rust
#[tauri::command]
fn vault_unlock(account_id: String, password: String) -> Result<(), String> {
    let result = vault::unlock(&account_id, &password);
    // password 在函数返回后由 Drop 自动擦除
    // （需确保 String 使用 zeroize::Zeroizing<String>）
    result.map_err(|e| e.to_string())
}
```

---

## 相关文档

- `tauri_refactor/状态管理方案.md` — Zustand 具体实施
- `tauri_refactor/IPC命令接口设计.md` — 完整命令列表
- `ADR-001-前端框架选型分析.md` — React 19 选型

---

*文档版本：v1.0*  
*创建日期：2026-06-04*
