# Android SAF 自动同步技术方案

> 状态：设计阶段，评审已完成，等待实现
> 范围：Android 客户端、Rust 后端、前端状态反馈
> 关联文档：`docs/android-vault-directory-design.md`
> 评审日期：2026-07-25

---

## 1. 背景与问题

### 1.1 当前架构

SoloSoul Android 采用「本地工作目录 + SAF 外部目录镜像」的同步架构：

```
┌──────────────────────────────┐
│  应用私有目录 (app-private)   │
│  /data/user/0/com.solosoul.app/saf_vault_temp/
│        │                      │
│        │  VaultService 读写   │
│        ▼                      │
│  saf_vault_temp/              │
│  ├── accounts.json            │
│  ├── acc_xxx/                 │
│  │   ├── vault.db            │
│  │   ├── objects/            │
│  │   └── attachments/        │
│  └── ...                      │
└─────────────────────────────┘
               │ sync_to_remote / sync_from_remote
               ▼
        ┌──────────────┐
        │ SAF 外部目录  │
        │ (用户选择的   │
        │  文件目录)   │
        └──────────────┘
```

### 1.2 当前时序

```
1. onboarding：用户选择外部 SAF 目录
       ↓
2. initialize_vault(saf_uri)
       → try_init_saf_vault()：创建 saf_vault_temp/
       → init_saf_sync()：sync_from_remote() 从 SAF 拉取
       ⚠️ 新 SAF 目录为空 → 拉取到 0 个文件
       → 同步完成，进入应用
       ↓
3. BootstrapPage：用户创建账户
       → bootstrap() → create_account() 写入 saf_vault_temp/
       ⚠️ 未触发 sync_to_remote()
       ↓
4. 用户开始使用，创建对象、附件
       → 所有写入仍在 saf_vault_temp/
       ⚠️ 仍未触发 sync_to_remote()
       ↓
5. 用户手动进入「设置 > 数据管理 > 保险库目录」
       → 点击同步
       → sync_to_remote() 将数据推到 SAF
```

### 1.3 核心问题

| 问题 | 影响 | 风险等级 |
|------|------|----------|
| **首次创建账户后未自动同步** | 用户选择外部目录后，SAF 文件夹长时间为空 | 高 |
| **卸载即丢数据** | 在首次手动同步前卸载应用，所有账户数据丢失 | 高 |
| **无后台同步机制** | 日常使用中的数据变更不会自动镜像到 SAF | 中 |
| **手动同步门槛高** | 普通用户不会主动触发同步 | 中 |

---

## 2. 设计目标

1. **消除首次数据丢失窗口**：创建首个账户后，应尽快将数据同步到 SAF。
2. **最小化持续数据丢失窗口**：日常使用中的写入应自动、增量地同步到 SAF。
3. **避免阻塞用户操作**：同步在后台执行，不应冻结 UI。
4. **防止同步冲突/重叠**：同一时刻只能有一个 sync_to_remote 执行。
5. **授权失效可感知**：SAF 授权被撤销时，用户能收到明确提示。
6. **功耗友好**：避免频繁、重复的完整同步。

---

## 3. 候选策略对比

| 策略 | 优点 | 缺点 | 适用场景 |
|------|------|------|----------|
| **每次写操作后同步** | 数据最安全，丢失窗口接近零 | SAF I/O 延迟高，会严重拖慢写入；耗电；附件上传会阻塞保存 | ❌ 不适用 |
| **定时同步（如 5 分钟）** | 实现简单，可预测 | 首次同步仍要等到第一个周期；空闲时白白同步 | 辅助手段 |
| **切后台同步** | 仅在用户离开应用时触发，最自然 | Android 可能 kill 应用，同步无法完成；长时间不离开则不同步 | 主要触发器之一 |
| **写操作防抖同步（30s）** | 把密集写合并为一次同步，平衡安全与性能 | 需维护脏标记和任务调度 | 主要触发器之二 |
| **关键里程碑同步** | 账户创建、模板导入等关键节点立即同步 | 无法覆盖日常增量写入 | 与防抖结合使用 |

**推荐方案：关键里程碑同步 + 写操作防抖同步 + 切后台同步 的混合策略。**

---

## 4. 推荐方案详情

### 4.1 总体架构与与 init_saf_sync 的对齐

新增一个 Rust 层 `AutoSyncManager`，作为所有自动同步的唯一调度器：

```
┌─────────────────────────────────────────
│           Frontend (React)              │
│  - 显示同步状态微指示器                  │
│  - 监听 saf-sync-status 事件            │
└──────────────┬──────────────────────────┘
               │ Tauri events / commands
┌──────────────▼──────────────────────────┐
│           AutoSyncManager (Rust)        │
│  - 单任务队列 + 防抖定时器                │
│  - 接收 SyncEvent::Immediate / Debounce   │
│  - 调度 vault_service.sync_to_remote()  │
└──────────────┬──────────────────────────┘
               │ invoke
┌──────────────▼──────────────────────────┐
│     VaultService / SafVaultFileSystem   │
│     - sync_to_remote()                  │
│     - sync_from_remote()                │
└─────────────────────────────────────────┘
```

### 4.2 触发器定义

| 触发器 | 触发条件 | 同步类型 | 说明 |
|--------|----------|----------|------|
| `TriggerImmediate` | 首个账户创建成功 | 立即同步 | 最高优先级，消除首次数据丢失窗口 |
| `TriggerDebounce` | 任意 Vault 写入操作 | 防抖 30s 后同步 | 合并密集写，避免重复同步 |
| `TriggerBackground` | 应用切后台 / 失去焦点 | 立即同步 | 用户离开时尽快落盘 |
| `TriggerManual` | 用户在设置页点击同步 | 立即同步 | 与自动调度共享同一队列 |

### 4.3 并发模型

采用 **Single-Flight + 待处理标记** 模式：

- `AutoSyncManager` 维护一个内部状态：
  - `running: bool` — 是否正在执行同步
  - `pending: bool` — 是否有新的同步请求在等待
  - `last_request: Instant` — 上次请求时间，用于防抖

- 当收到 `TriggerDebounce`：
  1. 设置 `pending = true`
  2. 重置 30s 防抖定时器
  3. 定时器到期后，若 `running == false`，启动同步；否则保持 `pending = true`

- 当收到 `TriggerImmediate` / `TriggerBackground`：
  1. 取消当前防抖定时器
  2. 若 `running == false`，立即启动同步
  3. 若 `running == true`，设置 `pending = true`，当前同步完成后立即启动下一次

- 同步完成后：
  - 若 `pending == true`，立即再次同步
  - 否则进入空闲状态

### 4.4 幂等性与增量同步

`sync_to_remote()` 底层已实现增量策略：
- 比较本地与远端文件的 mtime + size
- 仅上传变更文件
- 使用 `.tmp` 临时文件 + 原子重命名

因此频繁触发同步不会导致大量重复 I/O，只要文件未变更即可快速通过。

### 4.5 错误处理与重试

| 错误类型 | 处理策略 | 重试 |
|----------|----------|------|
| SAF 授权被撤销 | 暂停自动同步，发送 `saf-auth-revoked` 事件到前端 | 否，等待用户重新授权 |
| 临时 I/O 错误 | 指数退避重试，最多 3 次 | 是 |
| 磁盘空间不足 | 记录错误，通知用户 | 否 |
| 同步被系统中断 | 下次启动时从上次状态继续（增量） | 是 |

### 4.6 失败通知与 UX

- **同步中**：顶部状态栏显示「正在同步: xxx 已同步 N 个文件」（复用 onboarding 已有事件）。
- **同步完成**：短暂显示 ✅ 同步完成，3 秒后消失。
- **授权失效**：显示常驻横幅「外部保险库访问权限已失效，请重新选择目录」。
- **同步失败**：显示可关闭提示「同步失败：xxx，将在 N 秒后重试」。

---

## 5. 实现步骤

### Phase 1：核心调度器

1. **新增 `tauri/src-tauri/src/sync/auto_sync.rs`**
   - 定义 `SyncEvent` 枚举：`Immediate`、`Debounce`
   - 定义 `AutoSyncManager` 结构体，持有 `mpsc::Sender<SyncEvent>`
   - 实现 `start()` 方法启动后台任务循环
   - 实现 `trigger_immediate()`、`trigger_debounce()` 发送事件

2. **修改 `tauri/src-tauri/src/state/app_state.rs`**
   - 在 `AppState` 中新增 `auto_sync_tx: mpsc::Sender<SyncEvent>`
   - 在 `AppState::new()` 中初始化 `AutoSyncManager`
   - 提供 `pub fn auto_sync(&self) -> AutoSyncManagerHandle` 之类访问方法

### Phase 2：关键触发点接入

3. **修改 `tauri/src-tauri/src/commands/auth.rs` 的 `bootstrap`**
   - 在 `create_account()` 成功后：
     - 若当前使用 SAF 外部目录，发送 `TriggerImmediate`
   - 同步失败不阻塞账户创建，仅记录日志

4. **修改核心写入命令**
   - 在 `object_create`、`object_update`、`attachment_save`、`profile_save` 等命令后：
     - 若使用 SAF，发送 `TriggerDebounce`
   - 失败不阻塞写入，仅记录日志

### Phase 3：切后台触发

5. **前端 `tauri/src/hooks/useAutoLock.ts` 或新增 `useBackgroundSync.ts`**
   - 监听 `document.visibilitychange` 或原生切后台事件
   - 当应用进入后台时，调用 `vault_sync_to_remote()` 或发送触发事件
   - 避免与 `AutoSyncManager` 冲突：后端使用同一队列

### Phase 4：状态反馈

6. **新增 Tauri 事件**
   - `saf-sync-status`：携带 `{ phase: 'idle' | 'syncing' | 'done' | 'error', fileCount, message }`
   - `saf-auth-revoked`：授权失效时广播

7. **前端状态组件**
   - 在 `App.tsx` 或 `Layout` 中监听 `saf-sync-status`
   - 添加顶部/右上角同步状态微指示器
   - 授权失效时显示常驻横幅

### Phase 5：边界与兼容

8. **SAF 未启用时**：`AutoSyncManager` 存在但所有事件为 no-op，不增加开销。
9. **桌面端**：`AutoSyncManager` 存在，但仅作为 no-op 或未来扩展点。
10. **iOS 移植**：触发器接口保持抽象，iOS 背景任务实现独立封装。

---

## 6. 数据结构

### 6.1 Rust

```rust
// tauri/src-tauri/src/sync/auto_sync.rs

pub enum SyncEvent {
    Immediate,
    Debounce,
}

pub struct AutoSyncManager {
    tx: mpsc::Sender<SyncEvent>,
}

impl AutoSyncManager {
    pub fn new(vault_service: Arc<RwLock<VaultService>>, app_handle: AppHandle) -> Self;
    pub fn trigger_immediate(&self);
    pub fn trigger_debounce(&self);
}
```

### 6.2 前端事件

```ts
// 监听同步状态
listen<SafSyncStatus>('saf-sync-status', (event) => {
  const { phase, fileCount, message } = event.payload;
  // 更新 UI
});

// 授权被撤销
listen<void>('saf-auth-revoked', () => {
  // 显示重新授权提示
});
```

---

## 7. 测试计划

### 7.1 单元测试（Rust）

- `AutoSyncManager` 防抖逻辑：多次 Debounce 事件只触发一次同步
- `AutoSyncManager` 即时优先：Immediate 事件取消当前防抖并立即执行
- 同步失败重试：模拟失败 2 次，第 3 次成功

### 7.2 集成测试（Android 模拟器/真机）

| 用例 | 步骤 | 预期结果 |
|------|------|----------|
| 首次账户创建自动同步 | 安装 → 选择 SAF → 创建账户 → 立即检查 SAF 目录 | `accounts.json` 已出现在 SAF 目录 |
| 防抖合并写入 | 10 秒内创建 5 个对象 | 只触发一次 sync_to_remote（约 30s 后） |
| 切后台同步 | 创建一个对象 → 立即回到桌面 | 切后台后立即触发同步 |
| 授权失效处理 | 在系统设置中撤销 SAF 授权 → 等待下一次同步 | 应用显示授权失效提示，不崩溃 |
| 卸载前数据保留 | 创建账户后等待首次同步完成 → 卸载 → 重装 → 选择同一 SAF 目录 | 账户可恢复 |

### 7.3 性能测试

- 1000 个小文件场景下同步耗时
- 大附件（>50MB）场景下同步对 UI 的影响
- 电池消耗对比（开启/关闭自动同步）

---

## 8. 风险与缓解

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| Android 在切后台后 kill 应用，同步中断 | 数据部分丢失或残留 .tmp | 已在 Kotlin 层实现原子写入；下次启动继续增量同步 |
| 频繁触发同步耗电 | 电池消耗增加 | 30s 防抖 + mtime/size 增量过滤 |
| 同步失败重试风暴 | 反复失败导致 CPU/IO 占用 | 指数退避，最多 3 次，授权失效停止重试 |
| 前端状态与后端实际状态不一致 | 用户看到错误同步状态 | 通过 `saf-sync-status` 事件保持同步 |
| 多账户创建时触发多次同步 | 冗余同步 | Single-Flight 队列，pending 标记合并 |

---

## 9. 评审决策结论

### 9.1 防抖时间：30s 固定静态防抖

**决策**：采用 **30s 固定静态防抖**。

| 方案 | 结论 | 原因 |
|------|------|------|
| 5–10s | ❌ 不推荐 | 过于频繁，增加 SAF IPC 和电池开销，低-end 设备可能 UI 卡顿 |
| 30s | ✅ 采用 | 能合并密集写入，平衡安全与性能；用户离开前未超时则由切后台触发兜底 |
| 动态（按文件大小/类型） | ❌ 不推荐 | 复杂度远高于收益，V1 不引入 |

**说明**：SAF 同步是本地 ContentProvider IPC，不是网络传输，30s 已足够覆盖用户连续创建/编辑对象的场景。如果用户在 30s 内离开应用，切后台触发器会立即取消防抖并执行同步。`sync_to_remote` 内部基于 mtime/size 做增量过滤，因此 30s 触发一次也不会造成大量重复 I/O。

**可调整项**：真机测试后可根据低端设备表现微调到 20s 或 45s，但 V1 先保持 30s。

---

### 9.2 WorkManager：V1 不使用，依赖 Tauri 生命周期钩子

**决策**：**V1 不引入 WorkManager**，通过 `visibilitychange` / Android `onPause` 等生命周期事件触发同步。

| 方案 | 结论 | 原因 |
|------|------|------|
| 引入 WorkManager | ❌ V1 不做 | 需要新增 native Worker、前台服务、通知渠道、JNI 桥接，复杂度高；Tauri v2 WebView 与 WorkManager 不在同一进程，Rust runtime 重启成本大 |
| Tauri 生命周期钩子 | ✅ V1 采用 | 实现简单，复用现有 `autoLockPauseStore` 和 `useAutoLock` 的 visibilitychange 机制；SAF 同步为本地 I/O，通常秒级完成 |

**风险与缓解**：

| 风险 | 缓解措施 |
|------|----------|
| 切后台后应用被 kill，同步中断 | Kotlin 层已使用 `.tmp` + 原子重命名，中断不会产生损坏数据；下次启动后增量同步会继续 |
| 大文件同步期间用户离开 | 切后台仍有一定存活时间（通常数秒到数十秒）；V1 建议限制大附件场景下的同步或拆分为后台通知 |

**后续扩展**：若真机测试发现大量「切后台导致同步未完成」案例，可在 V2 引入 WorkManager 兜底，届时将 Rust 同步逻辑封装为独立任务并通过 `WorkManager` + 前台服务调用。

---

### 9.3 日常启动 sync_from_remote 拉取：V1 不开启

**决策**：**V1 不在日常启动时自动 `sync_from_remote`**，仅在 onboarding 阶段（`accounts.json` 不存在时）执行一次性的远端拉取。

| 方案 | 结论 | 原因 |
|------|------|------|
| 冷启动自动 pull | ❌ V1 不做 | 当前 `sync_from_remote`/`sync_to_remote` 是基于 mtime/size 的单向镜像，缺乏冲突解决能力；自动 pull 可能覆盖本地未同步数据 |
| 仅在 onboarding 拉取 | ✅ V1 采用 | 保持 SAF 作为「备份/镜像目标」的语义，本地数据始终优先 |
| 双向合并 | ❌ V1 不做 | 需要引入向量时钟、CRDT 或文件级冲突解决，超出当前范围 |

**说明**：本阶段 SAF 的核心定位是「连续备份」，不是「多设备同步」。

**例外场景**：
- 用户卸载重装后选择同一 SAF 目录 → onboarding 阶段已会执行 `sync_from_remote` 恢复账户。
- 用户从其他设备复制 SAF 目录 → 目前建议通过「导入备份」流程处理，而不是自动合并。

**后续扩展**：当明确需要多设备同步能力时，再引入双向同步与冲突解决，届时再评估是否开启启动拉取。

---

### 9.4 未决策/待后续讨论事项

1. **iOS 适配优先级**：当前设计 Android 专用，iOS 若后续支持外部目录需单独评估。
2. **冲突解决策略**：仅在 V2 多设备同步场景下重新设计。
3. **WorkManager 兜底**：作为 V2 候选，若 V1 真机测试发现切后台丢数据严重，则提前启动。

---

## 10. 后续开发 checklist

- [ ] 创建 `tauri/src-tauri/src/sync/auto_sync.rs`
- [ ] 在 `AppState` 中集成 `AutoSyncManager`
- [ ] 在 `bootstrap` 成功后触发首次同步
- [ ] 在核心写入命令后触发防抖同步
- [ ] 前端切后台时触发同步
- [ ] 前端监听 `saf-sync-status` 并显示状态指示器
- [ ] 处理 `saf-auth-revoked` 事件
- [ ] 补充 Rust 单元测试
- [ ] Android 真机回归测试

---

*最后更新：2026-07-25*
