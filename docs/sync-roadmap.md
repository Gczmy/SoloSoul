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

当前代码已通过 `cargo test --all` 与 `npm run test`，并推送至 `origin/master`（commit `24d2474`）。

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

#### P2.1 附件同步（Attachment Sync）

**为什么重要**：当前 `objects` 的附件文件（如图片、PDF）仅保存在本地 `attachments/` 目录，数据库记录同步后文件会缺失。

**具体步骤**：
1. 在 `solosoul-vault` 中增加附件元数据表或扩展 `objects`/`attachments` 表，记录 `file_hash`、`size`、`content_type`。
2. 设计附件传输协议：
   - 选项 A：直接复用现有 Noise 加密 TCP 连接，在 `SyncMessage` 中新增 `AttachmentRequest` / `AttachmentChunk` / `AttachmentAck`。
   - 选项 B：附件走独立 TCP 连接或 QUIC，避免阻塞主同步通道。
3. 在 `delta.rs` 中识别需要附件的记录，生成附件请求列表。
4. 在 `SyncManager` 中实现附件流式传输（建议按 64KB 分块 + 校验和）。
5. 落地时保存到接收方 attachments 目录，并更新数据库记录。
6. 补充单元测试：单文件往返、分块重传、hash 校验失败回滚。

**验收标准**：
- 两个 Vault 同步后，接收方能在 UI 中打开发送方的附件文件。
- 100MB 附件传输不阻塞主同步线程。
- 单元测试覆盖成功传输与校验失败两种场景。

**风险/待确认**：
- 附件目录结构在不同平台（macOS/Windows）是否一致。
- 是否需要断点续传；P2 可先实现全量传输，P3 再优化。

---

#### P2.2 删除墓碑（Deletion Tombstones）

**为什么重要**：`profiles` 和 `user_templates` 当前被删除时执行硬删除，导致删除事件无法传播给 peer。`objects` 因为有 `is_deleted` 标志可以传播，但 `profiles` / `user_templates` 不能。

**具体步骤**：
1. 扩展 `sync_hlc` 表或新增 `sync_tombstones` 表：
   - 字段：`table_name`、`record_id`、`deleted_at_hlc`、`deleted_by_node_id`、`purged_at`。
2. 修改 `VaultStore::delete_profile` / `delete_user_template`：
   - 在删除记录前，先写入 tombstone，HLC 使用 `Hlc::now(local_node_id)`。
3. 修改 `list_sync_changes_since`：
   - 不仅返回存活记录，也要返回未被 peer 消费过的 tombstone。
4. 修改 `apply_sync_record`：
   - 如果收到 tombstone 且其 HLC 大于本地 HLC，则删除本地记录（或保持已删除状态）。
5. 清理策略：tombstone 保留 30 天后可物理清理（与 trash_items 过期策略一致）。

**验收标准**：
- 设备 A 删除 profile 后，设备 B 同步后该 profile 也消失。
- 单元测试验证 tombstone 胜过高水位线的旧记录，但低于旧水位的 tombstone 不会误删。

**风险/待确认**：
- 与现有 `trash_items` 的语义区分：trash_items 是用户主动删除的业务 tombstone；`sync_tombstones` 是同步层 tombstone。二者可合并考虑。

---

#### P2.3 本地双实例联调（Smoke Test）

**为什么重要**：单元测试覆盖了数据层和 Noise 握手，但尚未在真实两个应用进程间验证 mDNS 发现 + 双向同步全流程。

**具体步骤**：
1. 准备两个独立数据目录：`~/.solosoul/account_a_device_1/` 和 `~/.solosoul/account_a_device_2/`。
2. 在两份数据上创建相同 account（密码可不同，但建议相同以便验证）。
3. 启动第一个 App 实例，启用同步，记录 `SyncPage` 显示的 fingerprint。
4. 启动第二个 App 实例，启用同步，信任第一个设备的 fingerprint。
5. 在第一个实例中信任第二个设备，然后在任一实例中点击同步。
6. 验证对象、profile、template 双向同步一致。
7. 在两台机器（或同一局域网两个 VM）上重复步骤 3–6，验证 mDNS 跨设备发现。

**验收标准**：
- 同一局域网两台设备无需手动输入 host:port 即可发现对方。
- 双向修改后，两边数据一致。
- 如果测试失败，记录失败场景并提交 issue。

**风险/待确认**：
- 测试环境曾出现 `tokio`/`std` TCP 流交互阻塞，生产路径已改为 `std::net::TcpListener`，但仍需实测。
- mDNS 在企业网络或 VPN 环境下可能不可达，需要 host:port 兜底。

---

### P3 — 中优先级（体验与性能）

#### P3.1 专用配对弹窗（Pairing Dialog）

**为什么重要**：当前信任按钮内联在设备列表中，没有显式的 fingerprint 比对流程，用户可能误信任攻击者设备。

**具体步骤**：
1. 设计新组件 `src/components/sync/PairingDialog.tsx`。
2. 当发现未信任 peer 时，弹出对话框显示：
   - 对方 device name / node_id
   - 对方 fingerprint
   - 操作按钮：“信任并配对” / “忽略”
3. 在 `SyncPage` 中替换当前内联 trust 按钮为“配对”入口。
4. 后端命令 `sync_request_pairing` 可选：先发起一次不传输数据的握手，仅验证 fingerprint 与 account 匹配。
5. 补充 Vitest 组件测试。

**验收标准**：
- 用户必须显式比对 fingerprint 后才能信任设备。
- 弹窗支持中英双语。

---

#### P3.2 同步冲突与活动日志 UI

**为什么重要**：当前同步结果只在 `SyncPage` 显示 `examined/applied/skipped`，没有逐条冲突或历史记录。

**具体步骤**：
1. 扩展 `SyncService` / `SyncManager` 返回更详细的 `SyncResult`：
   - 每表 applied/skipped 数量
   - 冲突列表（记录 ID、本地 HLC、远程 HLC、结果）
2. 在 `SyncPage` 增加“同步活动”折叠面板，展示最近 N 次同步结果。
3. 冲突项提供“查看详情”入口，仅显示非敏感 ID 与 HLC（不显示明文数据）。
4. 后端审计日志已记录 `sync_with_device` 等事件，前端可读取 `audit_log` 表渲染历史。

**验收标准**：
- 用户能看到最近一次同步的 applied/skipped 明细。
- 敏感数据不直接暴露在冲突日志中。

---

#### P3.3 大库分块流式同步

**为什么重要**：当前 `list_sync_changes_since` 会加载整表到内存，profile/template 数量大时会导致单次 Batch 过大。

**具体步骤**：
1. 在 `SyncMessage::Batch` 中支持 `finished: false`，允许一个表拆成多个 Batch。
2. 在 `VaultStore` 中增加 `list_sync_changes_since_paginated(table, watermark, limit, offset)`。
3. 在 `delta.rs` 中按 `limit`（如 500 条）生成 Batch 序列，接收方逐批应用并回 Ack。
4. 发送方在收到每批 Ack 后更新该表 watermark 为已发送记录的最大 HLC，支持断点续传。
5. 进度可返回给前端，用于展示同步进度条。

**验收标准**：
- 10 万条 object 记录同步时内存占用稳定，不 OOM。
- 中途网络断开可在下次同步时从上次 watermark 继续。

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
