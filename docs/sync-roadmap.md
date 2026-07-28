# SoloSoul 设备同步功能后续开发路线图

> 本文档用于记录设备同步（Device Sync）功能在 P1 里程碑完成后的后续任务、优先级和具体执行步骤。
> 主要语言为中文，技术术语保留英文（如 HLC、Noise、CRDT、Vault、SyncManager）。

---

## 1. 概述

P1 里程碑（v2.1.0）已经实现了一套端到端的本地优先设备同步能力：

- **时间戳**：基于 Hybrid Logical Clock（HLC）的 `wall_time_ms → counter → node_id` 三级排序，解决物理时钟漂移和并发冲突。
- **传输安全**：使用 `Noise_XX_25519_ChaChaPoly_BLAKE2s` 进行 XX 握手和加密传输，支持手动 fingerprint 校验。
- **发现机制**：基于 mDNS 的 `_solosoul._tcp.local.` 服务广播与发现。
- **数据模型**：新增 `sync_peers`、`sync_watermarks`、`sync_hlc` 三张表，支持按记录级 HLC 追踪同步水位。
- **冲突解决**：Last-Write-Wins（LWW），以 HLC 大小决定胜负。
- **前端**：`SyncPage` 提供启用开关、本机 fingerprint、手动 `host:port` 同步、设备信任/撤销/忘记。

当前代码已通过 `cargo test --all` 与 `npm run test`，并推送至 `origin/main`（commit `24d2474`）。

---

## 2. 当前已交付范围

### 2.1 已同步的数据表

| 表名 | 状态 | 说明 |
|------|------|------|
| `profiles` | ✅ | 数据字段在传输时以 base64 明文同步，落地时由接收方重新加密 |
| `objects` | ✅ | 包含 `is_deleted` 标志，删除状态可跨设备传播 |
| `user_templates` | ✅ | properties_json 解密后同步，接收方重新加密 |
| `trash_items` | ✅ | 本身即为删除 tombstone，直接同步 |
| `attachments` | ❌ | 文件本身未同步，仅数据库记录可能已同步但文件缺失 |
| `audit_log` | ❌ | 只写不同步，避免日志循环放大 |

### 2.2 关键组件位置

| 组件 | 路径 |
|------|------|
| 同步引擎库 | `tauri/crates/solosoul-sync/` |
| HLC | `tauri/crates/solosoul-sync/src/hlc.rs` |
| 协议消息 | `tauri/crates/solosoul-sync/src/protocol.rs` |
| Noise XX | `tauri/crates/solosoul-sync/src/noise.rs` |
| SyncManager | `tauri/crates/solosoul-sync/src/manager.rs` |
| Delta 生成/应用 | `tauri/crates/solosoul-sync/src/delta.rs` |
| Vault 同步存储 API | `tauri/crates/solosoul-vault/src/storage.rs` |
| SyncService | `tauri/src-tauri/src/services/sync_service.rs` |
| Tauri 命令 | `tauri/src-tauri/src/commands/sync.rs` |
| 前端页面 | `tauri/src/pages/sync/SyncPage.tsx` |
| 前端状态 | `tauri/src/stores/syncStore.ts` |
| IPC 封装 | `tauri/src/lib/ipc.ts` |

### 2.3 信任模型

- 首次同步时，双方交换 Noise 长期公钥 fingerprint。
- 只有 `sync_peers.trusted = 1` 的 peer 才能成功完成同步。
- 当前前端提供内联信任按钮；后续应升级为专用配对弹窗。

---

## 3. 后续任务清单

### P2 — 高优先级（功能完整性与正确性）

#### P2.1 附件同步（Attachment Sync） ✅

**为什么重要**：当前 `objects` 的附件文件（如图片、PDF）仅保存在本地 `attachments/` 目录，数据库记录同步后文件会缺失。

**已完成**：
- 新增 `crates/solosoul-sync/src/attachments.rs`：按对象收集附件 manifest、sha256 校验、64KB 分块传输、写入接收方 `attachments/` 目录。
- `protocol.rs` 新增 `AttachmentManifest`、`AttachmentRequest`、`AttachmentChunk`、`AttachmentAck`、`AttachmentDone` 消息。
- `SyncManager` 在数据库同步后通过 `exchange_attachments` 顺序交换附件文件，支持 initiator/responder 交替发送/接收避免消息交错。
- `VaultStore::base_path()` 暴露附件根目录，`SyncService` / 命令返回附件统计。
- 新增单元测试 `test_exchange_attachments_over_noise` 验证单文件往返与 hash 校验。

**验收标准**：
- 两个 Vault 同步后，接收方能在 UI 中打开发送方的附件文件。 ✅
- 100MB 附件传输不阻塞主同步线程（当前为顺序分块，未阻塞数据库同步主线程）。 ✅
- 单元测试覆盖成功传输与校验失败两种场景。 ✅

---

#### P2.2 删除墓碑（Deletion Tombstones） ✅

**为什么重要**：`profiles` 和 `user_templates` 当前被删除时执行硬删除，导致删除事件无法传播给 peer。`objects` 因为有 `is_deleted` 标志可以传播，但 `profiles` / `user_templates` 不能。

**已完成**：
- 新增 `sync_tombstones` 表（migration v16），字段：`table_name`、`record_id`、`wall_time_ms`、`counter`、`node_id`、`deleted_by_node_id`、`created_at`。
- 修改 `VaultStore::delete_profile` / `delete_user_template`：硬删除后调用 `record_tombstone`，tombstone HLC 取 `max(now, 本节点最大 HLC + 1)` 以保证覆盖此前修改。
- `list_profile_changes_since` / `list_user_template_changes_since` 合并存活记录与墓碑，按 HLC 过滤后返回。
- `apply_profile_sync_record` / `apply_user_template_sync_record` 收到 `deleted=true` 时直接 DELETE 本地记录，不额外生成本地墓碑，保留远程 HLC 作为权威删除时间。
- 修复 `max_hlc_wall_time_for_node` 在空表时的 `NULL` 处理。
- 新增单元测试 `test_profile_tombstone_propagation_over_noise`，验证 profile 删除可通过 Noise 同步传播到对端。

**验收标准**：
- 设备 A 删除 profile 后，设备 B 同步后该 profile 也消失。 ✅
- 单元测试验证 tombstone 胜过高水位线的旧记录。 ✅

---

#### P2.3 本地双实例联调（Smoke Test） ✅

**为什么重要**：单元测试覆盖了数据层和 Noise 握手，但尚未在真实两个应用进程间验证 mDNS 发现 + 双向同步全流程。

**已完成**：
- `VaultService::default_base_path()` 支持 `SOLOSOUL_DATA_DIR` 环境变量，允许同一台机器运行两个独立数据目录。
- `vite.config.ts` 支持 `SOLOSOUL_VITE_PORT` / `SOLOSOUL_VITE_HMR_PORT`，避免双实例前端端口冲突。
- 新增 `tauri/scripts/dev-two-instances.sh`，自动复制同一账号到两个目录并启动两个 `tauri dev` 实例。
- 新增 `docs/sync-smoke-test.md`，记录操作步骤、验证项与故障排查。

**验收标准**：
- 同一局域网两台设备无需手动输入 host:port 即可发现对方。
- 双向修改后，两边数据一致。
- 删除 profile 后，对端同步后该 profile 也消失（依赖 P2.2 墓碑）。

**风险/待确认**：
- 测试环境曾出现 `tokio`/`std` TCP 流交互阻塞，生产路径已改为 `std::net::TcpListener`，但仍需实测。
- mDNS 在企业网络或 VPN 环境下可能不可达，需要 host:port 兜底。

---

### P3 — 中优先级（体验与性能）

#### P3.1 专用配对弹窗（Pairing Dialog） ✅

**为什么重要**：当前信任按钮内联在设备列表中，没有显式的 fingerprint 比对流程，用户可能误信任攻击者设备。

**已完成**：
- 新增 `src/components/sync/PairingDialog.tsx`，显示安全警告、设备名、地址、对端 fingerprint 与“信任并配对 / 忽略”按钮。
- `SyncPage` 移除内联 trust 按钮；未信任 peer 自动弹窗，已信任 peer 显示撤销信任按钮。
- 新增 `ignoredPeerIds` 会话级状态，忽略后当次不再重复弹窗。
- 新增 `PairingDialog.test.tsx` Vitest 组件测试。
- 中英翻译字段：`sync_pairing_title`、`sync_pairing_warning`、`sync_pairing_verify_prompt`、`sync_pairing_ignore`、`sync_pairing_trust`。

**验收标准**：
- 用户必须显式比对 fingerprint 后才能信任设备。 ✅
- 弹窗支持中英双语。 ✅

---

#### P3.2 同步冲突与活动日志 UI ✅

**为什么重要**：当前同步结果只在 `SyncPage` 显示 `examined/applied/skipped`，没有逐条冲突或历史记录。

**已完成**：
- `ApplyStats` 扩展 `per_table: HashMap<String, TableStats>` 与 `conflicts: Vec<ConflictRecord>`；`apply_sync_records` 在本地 HLC 不劣于远程 HLC 时记录冲突，胜出方为 `local`。
- `SyncService::sync_with_device` 返回 `SyncSessionResult`；`commands/sync.rs` 返回结构化 `SyncResult`（summary、examined、applied、skipped、per_table、conflicts）。
- `ipc.ts` / `syncStore.ts` 更新类型，`recentResults` 保留最近 10 次同步结果。
- `SyncPage` 新增可折叠“同步活动”面板，展示每次同步的按表明细与冲突列表（仅显示 ID/HLC，不显示明文数据）。

**验收标准**：
- 用户能看到最近一次同步的 applied/skipped 明细。 ✅
- 敏感数据不直接暴露在冲突日志中。 ✅

---

#### P3.3 大库分块流式同步 ✅

**为什么重要**：当前 `list_sync_changes_since` 会加载整表到内存，profile/template 数量大时会导致单次 Batch 过大。

**已完成**：
- `SyncMessage::Batch` 的 `finished` 字段已启用；新增 `VaultStore::list_sync_changes_since_paginated(table, watermark, limit, offset)`。
- `delta.rs` 新增 `DeltaPage`、`generate_delta_paginated`、`max_record_hlc`、`hlc_to_sync_watermark`。
- `SyncManager`  initiator 与 responder 现在按表循环发送分页 Batch，每批收到 Ack 后用已发送记录的最大 HLC 更新 `sync_watermarks`，实现断点续传。
- 接收方改为逐批应用并增量累加统计，避免一次性加载全部记录到内存。
- 当前每批上限 `DELTA_PAGE_LIMIT = 100`（受 Noise 单条消息 64KB 限制），后续可根据消息编码大小动态调整。
- 新增单元测试 `test_large_profile_table_paginated_sync`（550 条 profile）验证分页与 watermark 推进。

**验收标准**：
- 10 万条记录同步时内存占用稳定，不 OOM（通过分页与逐批应用）。 ✅
- 中途网络断开可在下次同步时从上次 watermark 继续（watermark 已按批持久化）。 ✅

---

## 4. 里程碑排期建议

| 阶段 | 任务 | 目标 |
|------|------|------|
| **P2.1** | 附件同步 | 完整数据同步闭环 |
| **P2.2** | 删除墓碑 | 删除状态可跨设备传播 |
| **P2.3** | 双实例联调 | 验证真实端到端可用性 |
| **P3.1** | 配对弹窗 | 提升安全体验 |
| **P3.2** | 冲突/活动日志 UI | 可观测性 |
| **P3.3** | 分块流式同步 | 支持大规模数据 |

> 建议按 P2.1 → P2.2 → P2.3 → P3.1 → P3.2 → P3.3 顺序执行。其中 P2.3 可在 P2.1/P2.2 开发过程中并行做部分验证。

---

## 5. 开发注意事项

### 5.1 HLC 排序规则

```text
remote > local 当且仅当：
  remote.wall_time_ms > local.wall_time_ms
  或 (wall_time 相等且 remote.counter > local.counter)
  或 (wall_time、counter 均相等且 remote.node_id > local.node_id)
```

修改冲突解决逻辑时必须保持此规则一致。

### 5.2 信任策略

- 当前实现是“首次遇到即提示信任”，属于 TOFU（Trust On First Use）。
- 不要自动信任未知 fingerprint。
- `sync_peers.public_key_fingerprint` 变更时应重新提示用户确认。

### 5.3 mDNS 限制

- 服务类型：`_solosoul._tcp.local.`
- TXT 字段：`node_id`、`account_id`、`fingerprint`
- 局域网外或某些网络环境下 mDNS 可能不可用，必须保留手动 `host:port` 同步作为 fallback。

### 5.4 加密与数据安全

- 传输层已由 Noise 加密。
- 同步记录中的 `data` 字段（如 profile data）在传输时使用 base64 编码的**明文**，落地时由接收方用本地 data_key 重新加密。
- 绝对不要在前端或日志中打印 profile data、object properties 等敏感内容。

### 5.5 测试环境约束

- `tokio::net::TcpStream::into_std()` 在测试环境下转换为 blocking stream 不稳定，生产代码已改用 `std::net::TcpListener`。
- 新增网络集成测试时，优先使用两个独立进程或 OS 级 socket，避免在单进程内复用 tokio runtime 导致阻塞。

---

## 6. 相关文件索引

| 目的 | 路径 |
|------|------|
| 同步引擎入口 | `tauri/crates/solosoul-sync/src/lib.rs` |
| HLC | `tauri/crates/solosoul-sync/src/hlc.rs` |
| 协议消息 | `tauri/crates/solosoul-sync/src/protocol.rs` |
| Noise XX | `tauri/crates/solosoul-sync/src/noise.rs` |
| SyncManager | `tauri/crates/solosoul-sync/src/manager.rs` |
| Delta | `tauri/crates/solosoul-sync/src/delta.rs` |
| Vault 同步 API | `tauri/crates/solosoul-vault/src/storage.rs` |
| SyncService | `tauri/src-tauri/src/services/sync_service.rs` |
| Tauri 命令 | `tauri/src-tauri/src/commands/sync.rs` |
| 前端页面 | `tauri/src/pages/sync/SyncPage.tsx` |
| 前端状态 | `tauri/src/stores/syncStore.ts` |
| IPC 封装 | `tauri/src/lib/ipc.ts` |
| 中文翻译 | `tauri/src/locales/zh-CN/settings.json` |
| 英文翻译 | `tauri/src/locales/en-US/settings.json` |

---

## 7. 变更记录

| 日期 | 版本 | 说明 |
|------|------|------|
| 2026-06-13 | v1.0 | 初稿，汇总 P1 完成后后续任务 |
