# 通用数据预取框架（Prefetch Runtime）设计文档

> 状态：设计评审稿（未实施）
> 目标：消除页面级异步数据加载的「骨架期/闪烁」，为所有页面提供统一的后台预取运行时。
> 适用范围：桌面端（macOS/Windows/Linux）+ 移动端（Android/iOS）——两端同机制，仅两处平台适配点（见 §5）。

---

## 1. 背景与目标

### 1.1 问题

页面挂载即发起 IPC 拉数据（约 20 处），表现为：

- **首次进入有加载期**：骨架/占位 → 内容，即使高度对齐（0px 位移）仍有「灰条→内容」的视觉突变；
- **来回切换重复 IPC**：`useOcrModelManager` 等 mount-only hook 每次进入重新拉取，页面间零共享；
- **闪烁感知**：移动端（JavaScriptCore 慢）与桌面端（WebView 冷启动）均存在，移动端更明显。

### 1.2 目标

1. **数据在进入页面时已就绪**——加载在后台完成，页面直接渲染内容，无骨架期、无闪烁；
2. **跨页面共享缓存**——来回切换零 IPC；
3. **通用**——所有页面数据走同一抽象；存量已 store 化数据包装接入，不重写；
4. **两端一致**——同一机制在桌面/移动端生效，平台差异收敛为两处显式适配点。

### 1.3 非目标

- 不做 SSR/预渲染（Tauri 本地应用，无此概念）；
- 不预取网络下载类数据（模型文件、APK 等大文件）；
- 不预取会话性/大数据集（导出范围树、导入预览、搜索）——只取缓存去重收益。

---

## 2. 现状盘点（迁移面）

### 2.1 已 store 化（天然单例 + 共享，包装接入即可）

| 数据 | Store | 消费者 |
|------|-------|--------|
| 模板列表 | `templateStore` | TemplateManagerPage 等 |
| 回收站 | `trashStore` | TrashPage、GlobalAttachmentManager |
| 同步状态 | `syncStore` | SyncPage、GlobalSyncIndicator |
| 档案/账户 | `profileStore` / `authStore` | 全局 |
| 设置 | `settingsStore` | 全局 |

### 2.2 裸 `useEffect + invoke`（本次主要迁移对象）

| 数据 | 位置 | 预热建议 |
|------|------|---------|
| Vault 统计 | DataManagementPage / SettingsPage | `afterAuth` |
| 备份列表 | BackupConfigPage | `afterAuth` |
| OCR 模型状态 | OcrPage / OcrSettingsPage（useOcrModelManager）| `afterAuth`（桌面专用，移动端门控）|
| 操作日志 | OperationLogPage / DebugLogPage | `never`（按需，仅共享）|
| 同步状态轮询 | useSyncPage | `afterAuth` |
| 搜索 | SearchPage | `never` |
| LLM 统计 | LlmStatsPage | `afterAuth` |
| 插件列表 | PluginDashboardPage | `afterAuth` |

---

## 3. 核心架构

```
┌─ 预热调度（后台 fire-and-forget）─────────────────────┐
│ App 挂载 → 登录/解锁完成 → 空闲批量（idle + timeout 降级）│
└──────────────────────┬───────────────────────────────┘
                       ▼
        ┌──────────────────────────────┐
        │  prefetchRegistry（模块级单例） │
        │  vaultStats / ocrModel /      │
        │  backups / templates / trash  │
        │  / syncStatus / logs / …      │
        └──────┬───────────┬────────────┘
                │读缓存      │ invalidate()
                ▼           ▼
     usePrefetchData(hook)   变更操作（安装/下载/删除/
     OcrPage / SettingsPage…  备份/对象增删/事件）
```

### 3.1 `createPrefetchStore<T>`（工厂，~60 行）

```ts
// lib/prefetch/createPrefetchStore.ts
interface PrefetchStoreOptions<T> {
  key: string;
  loader: () => Promise<T>;                 // 该数据的一次 IPC 加载
  ttlMs?: number;                           // 默认 5min
  warmupPolicy?: 'always' | 'afterAuth' | 'never'; // 默认 'never'
  enabledOnPlatform?: () => boolean;        // 平台门控（默认全平台）
}

interface PrefetchStore<T> {
  data: T | null;
  loading: boolean;
  error: string | null;
  lastLoadedAt: number | null;
  load: (opts?: { force?: boolean }) => Promise<T | null>;  // 幂等
  invalidate: () => Promise<T | null>;      // 清缓存 + 重载
  warmup: () => void;                       // 后台吞错
  reset: () => void;
}
```

**关键机制**：
- **in-flight 去重**：模块级 `pending: Promise | null` 单例——并发 `load()` 共享同一 promise，杜绝重复 IPC；
- **TTL 复用**：`lastLoadedAt + ttlMs` 内命中直接返回缓存；
- **warmup 静默**：`catch(() => {})`，失败由页面挂载兜底；
- **平台门控**：`enabledOnPlatform()` 为 false 时 `load/warmup` 直接走 loader 兜底语义（页面按需），不预热。

### 3.2 `prefetchRegistry`（注册表，~40 行）

```ts
// lib/prefetch/registry.ts
export const prefetchRegistry = {
  vaultStats: createPrefetchStore({
    key: 'vault-stats',
    loader: () => invoke<VaultStats>('get_vault_stats'),
    ttlMs: 60_000,
    warmupPolicy: 'afterAuth',
  }),
  ocrModel: createPrefetchStore({
    key: 'ocr-model',
    loader: loadOcrModelStatus,             // 3 组 IPC 收敛为一次 loader
    ttlMs: 300_000,
    warmupPolicy: 'afterAuth',
    enabledOnPlatform: () => !isMobilePlatformSync(),  // 移动端 ML Kit，无需模型状态
  }),
  // … backups / templates / trash / syncStatus / logs
};
```

### 3.3 `usePrefetchData`（消费端 hook，~30 行）

```ts
// lib/prefetch/usePrefetchData.ts
export function usePrefetchData<T>(store: PrefetchStore<T>) {
  // 挂载：缓存命中直接用（0 加载期），缺失/过期才 load
  // 返回 { data, loading, error, reload: () => store.load({ force: true }) }
}
```

页面改造示例（OcrPage 语义等价替换）：
```tsx
const ocrModel = usePrefetchData(prefetchRegistry.ocrModel);
// 替代 useOcrModelManager 内的独立 IPC：进入页面命中缓存 → 无骨架期
```

### 3.4 预热调度（~30 行）

```ts
// lib/prefetch/warmup.ts
export function warmupPrefetchRegistry(phase: 'mount' | 'afterAuth') {
  const idle = (typeof requestIdleCallback === 'function' ? requestIdleCallback : (cb) => setTimeout(cb, 200)) as typeof requestIdleCallback;
  idle(() => {
    for (const store of Object.values(prefetchRegistry)) {
      if (store.options.warmupPolicy === phase || store.options.warmupPolicy === 'always') {
        store.warmup();
      }
    }
  }, { timeout: 2000 });
}
```

**触发点**：
- App 挂载（Bootstrap 阶段）→ `warmupPrefetchRegistry('mount')`；
- 登录/解锁完成（authStore 状态变化处）→ `warmupPrefetchRegistry('afterAuth')`。

---

## 4. 失效与一致性

| 机制 | 说明 |
|------|------|
| **显式 invalidate** | 变更操作后调用（模型安装/下载/删除、备份创建/删除、对象增删改）——现有全部变更调用点逐一接入 |
| **TTL 兜底** | 防外部变化（手动删文件等）导致的陈旧；大数据项 TTL 更短或 `never` |
| **事件失效** | 数据变更事件（模板同步、sync 完成）监听 → 对应 `invalidate()` |
| **进程重启** | 模块级缓存随进程消亡，启动重新预热，无残留 |

---

## 5. 平台适配（仅两处显式差异）

| 适配点 | 桌面端 | 移动端 | 落地方式 |
|--------|--------|--------|---------|
| **预热集合门控** | 全量预热 | 跳过桌面专用项（OCR 模型状态等）| `enabledOnPlatform` 选项，注册时判断 |
| **requestIdleCallback** | 原生支持 | iOS WKWebView 旧版缺失 | 降级 `setTimeout(200ms)`——复用 P015 预取的 `FALLBACK_TICK` 模式 |
| **内存策略（可选）** | 无需 | 大数据项（trash/backup）可缩短 TTL 或移出预热集合 | 注册表逐项配置 |

**一致性保证**：
- 预取数据全部为本地 IPC（非网络下载）→ 移动端无流量/省流顾虑；
- 解锁流程两端都有延迟（桌面输密码 / 移动 PIN/生物识别）→ `afterAuth` 预热窗口两端均充分；
- invalidate 路径两端同一套变更调用点。

---

## 6. 迁移清单

### P1 — 试点（验证方案，1-2 天）

| 项 | 内容 | 验收 |
|----|------|------|
| `lib/prefetch/` 三件套 | createPrefetchStore + registry + usePrefetchData + warmup | tsc / 单测（去重、TTL、invalidate、门控）|
| OCR 模型状态迁移 | OcrPage + OcrSettingsPage 读 `prefetchRegistry.ocrModel` | 实测进入 OCR 页按钮位移 0px 且**无骨架期**（Playwright 测量脚本）|
| 预热接入 | App 挂载 + 登录/解锁完成触发 | 登录期间后台完成，进入页面直接渲染 |

### P2 — 高频小数据（3-5 天）

| 项 | 说明 |
|----|------|
| vault stats / backups / templates / trash / sync status / settings / LLM 统计 / 插件列表 | 逐个迁移（每页 5-10 行）；存量 store（template/trash/sync）**包装进 registry，loader 指向现有 fetch 方法，页面零改动** |
| 变更点 invalidate 接入 | 备份创建/删除、对象增删、模板变更等 |

### P3 — 按需接入（1-2 天）

| 项 | 说明 |
|----|------|
| 导出范围树 / 导入预览 / 日志 / 搜索 | `warmupPolicy: 'never'`，仅取跨页面缓存去重收益（如导出页与设置页复用同一范围数据）|

---

## 7. 实施计划

### 阶段 0：基础设施（0.5 天）
- 新建 `lib/prefetch/`：createPrefetchStore、registry、usePrefetchData、warmup；
- 单测：in-flight 去重（并发 load 仅一次 loader）、TTL 命中、force 重载、invalidate、平台门控、idle 降级。

### 阶段 1：P1 试点（1 天）
- OCR 模型状态迁移（OcrPage + OcrSettingsPage）；
- 预热挂载点（App 挂载 + 登录/解锁完成）；
- 验收：Playwright 实测进入 OCR 页**无骨架期**（数据就绪时骨架条件不触发）+ 位移 0px；全量测试通过。

### 阶段 2：P2 高频数据（3-5 天）
- 按 §6 逐个迁移 + invalidate 接入；
- 每批验收：tsc / vitest / eslint / prettier + 手动冒烟（进入/切换页面无加载期）。

### 阶段 3：P3 按需数据 + 收尾（1-2 天）
- 大数据接入（只拿去重）；
- 数据变更事件失效监听；
- 文档同步（本文件标记已实施 + CHANGELOG）。

### 验收总标准
1. 进入 P2 列表内任意页面：无骨架期（数据已就绪），首次进入与后续进入行为一致；
2. 来回切换页面：零新增 IPC（缓存命中，可经 E2E 计数验证）；
3. 模型安装/下载/删除后：页面数据实时刷新（invalidate 生效）；
4. 移动端：预热集合不含桌面专用项；iOS 无 requestIdleCallback 环境降级正常。

---

## 8. 风险与回退

| 风险 | 缓解 |
|------|------|
| 陈旧数据（TTL 窗口内外部变化）| 显式 invalidate + 事件失效，窗口极小；数据敏感性低（非认证/非实时）|
| 预热过度（无谓 IPC）| 只注册「进入频繁 + 数据小」项；`never` 项不预热 |
| 迁移回归（页面行为变化）| 每个 store 有单测；页面改造保持「语义等价替换」，骨架逻辑保留作兜底 |
| 平台差异遗漏 | 平台适配收敛为 §5 两处，单测覆盖门控与降级 |

**回退**：框架层失败不影响页面（warmup 吞错 + 页面挂载兜底走原 loader）；单页迁移回退 = 改回 `usePrefetchData` → 原 hook，改动局部。

---

## 附：与现有代码的关系

- **不重写**：`templateStore` / `trashStore` / `syncStore` 保持内部实现，仅注册进 registry；
- **不删除**：`useOcrModelManager` 等 hook 改为内部读 registry（保留其安装/下载/删除回调语义），或保留为薄壳；
- **沿用先例**：`isMobilePlatformSync` 门控、`FALLBACK_TICK` idle 降级、authStore 登录态事件——均已有实现可复用。
