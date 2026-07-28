# SoloSoul 同步功能现状与下一步开发技术报告

> 文档类型：技术分析与开发规划报告
> 编写日期：2026-07-28
> 关联文档：
> - `docs/sync-roadmap.md`：同步功能路线图
> - `docs/android-saf-auto-sync-design.md`：Android SAF 自动同步设计
> - `docs/sync-smoke-test.md`：双实例联调文档
> 代码范围：
> - `tauri/src/pages/sync/SyncPage.tsx`
> - `tauri/src/stores/syncStore.ts`
> - `tauri/crates/solosoul-sync/`
> - `tauri/src-tauri/src/commands/sync.rs`
> - `tauri/src-tauri/src/commands/discovery.rs`
> - `tauri/src-tauri/gen/android/app/src/main/java/com/solosoul/app/NsdPlugin.kt`

---

## 1. 执行摘要

SoloSoul 的「设置-同步」（Device Sync）功能已完成端到端的协议与 UI 实现：基于 `Noise_XX_25519_ChaChaPoly_BLAKE2s` 的加密通道、HLC 时钟、LWW 冲突解决、分页 Delta 同步、附件分块同步、删除墓碑传播等均已落地。桌面端使用 `mdns-sd` 做 mDNS 发现，Android 端使用 `NsdPlugin` 做 NSD 发现。

然而，从**真实可用性**角度看，当前功能仍有明显缺口：

1. **账户级跨设备迁移缺失**：同步只能同步数据，不能传输/恢复账户本身。
2. **缺少自动触发机制**：Device Sync 仍依赖用户手动点击同步。
3. **跨平台发现层一致性与可观测性不足**（P1 已处理）：桌面 mDNS 服务类型与 Android NSD 服务类型因平台 API 差异写法不同，代码中缺乏明确注释；用户发现失败时缺少手动 fallback 引导。P1 已补充注释、一致性测试、监听端口展示与手动 fallback 提示。
4. **iOS 发现层未实现**：NSD 插件仅覆盖 Android。

本报告将围绕上述问题，给出下一阶段的开发方向、技术方案和验收标准。

---

## 2. 当前实现状态

### 2.1 前端

`SyncPage` 已具备：

- 同步开关
- 本机 fingerprint 展示
- 附近设备发现
- 手动 host:port 同步
- 配对/信任/撤销/忘记设备
- 同步活动与冲突日志

### 2.2 后端

| 模块 | 实现状态 | 关键文件 |
|------|----------|----------|
| 加密传输 | ✅ | `solosoul-sync/src/noise.rs` |
| 会话管理 | ✅ | `solosoul-sync/src/session.rs` |
| HLC / 冲突 | ✅ | `solosoul-sync/src/hlc.rs`、`delta.rs` |
| 分页 Delta | ✅ | `solosoul-sync/src/delta.rs` |
| 附件同步 | ✅ | `solosoul-sync/src/attachments.rs` |
| 删除墓碑 | ✅ | `solosoul-vault` + `delta.rs` |
| 桌面发现 | ✅ | `commands/discovery.rs`（mdns-sd） |
| Android 发现 | ✅ | `NsdPlugin.kt` + `commands/discovery.rs` |
| 移动端服务 | ✅ | `solosoul-sync/src/mobile.rs` |

### 2.3 已实现测试

- 单元测试覆盖 Noise 握手、附件传输、分页同步、墓碑传播等。
- 缺少真实的双实例 / 跨平台端到端自动化测试。

---

## 3. 关键 Gap 与风险

### 3.1 Gap 1：账户无法通过同步迁移

**问题描述**

当前同步协议在握手阶段会校验 `account_id` 是否一致。如果一台设备是新创建的账户，另一台设备是已有账户，两者的 `account_id` 不同，同步会被直接拒绝。

**影响**

- 用户无法像 iCloud/Google 账号那样「在新设备上登录同一账户并自动同步」。
- 跨设备使用门槛高：必须先手动导出/导入 `.solosoul` 备份。

**风险等级**：🔴 高

### 3.2 Gap 2：缺少自动同步触发

**问题描述**

Device Sync 目前仅支持手动触发（用户进入设置页点击同步）。没有后台定时同步、切后台同步或事件驱动同步。

**影响**

- 数据一致性完全依赖用户主动操作。
- 真实使用中容易出现「我在手机上改了，但桌面端没同步」的情况。

**风险等级**： 高

### 3.3 Gap 3：跨平台发现层一致性与可观测性不足

**问题描述**

- 桌面端注册的服务类型：`_solosoul._tcp.local.`（`mdns-sd` 要求完整后缀）。
- Android NSD 注册的服务类型：`_solosoul._tcp`（`NsdManager` 要求省略后缀）。

两者在**底层网络层等价**（Android 会自动补齐 `.local.`），但代码层面缺乏明确注释和一致性校验，容易被后续开发者误改。此外，当前缺少发现失败时的手动 fallback 引导。

**影响**

- 新开发者可能误以为两端字符串应完全一致，导致误改某端后自动发现失效。
- 用户发现不到设备时缺少下一步指引。

**风险等级**：🟡 中

### 3.4 Gap 4：iOS 发现层缺失

**问题描述**

`NsdPlugin.kt` 仅在 Android 目录下，`solosoul-sync/src/mobile.rs` 虽同时被 `android` 和 `ios` 条件编译命中，但 iOS 没有 NSD 插件，发现功能无法工作。手动 host:port 理论上可用，但用户需要自行知道地址。

**影响**

- iOS 目前无法使用自动发现功能。
- 跨平台体验不完整。

**风险等级**： 中

### 3.5 Gap 5：冲突解决单一

**问题描述**

当前仅支持 LWW（Last-Write-Wins）自动冲突解决。UI 可以查看冲突记录，但无法手动选择版本。

**影响**

- 误删/误改数据时无法挽回。
- 高级用户希望有冲突解决界面。

**风险等级**：🟢 低

---

## 4. 下一阶段开发方向

基于优先级和实现成本，建议按以下顺序推进：

### 4.1 第一阶段：发现层一致性加固与手动 fallback 优化 ✅ 已完成

**目标**

- 明确桌面端与 Android 端服务类型字符串的平台差异，并通过文档/测试固化。
- 在真实局域网环境下验证跨平台发现能力。
- 提供清晰的手动 host:port 配对引导。

**已完成工作**

1. **服务类型注释与一致性校验**：
   - 在 `manager.rs`、`commands/discovery.rs` 与 `NsdPlugin.kt` 的常量旁补充注释，说明平台 API 差异。
   - 新增测试 `test_mdns_service_type_matches_android_base`，验证桌面端常量去掉 `.local.` 后与 Android 端常量完全一致。
2. **暴露当前监听端口**：
   - 桌面端 `sync_listen_port` 命令从返回 0 改为返回 `SyncManager` 实际监听端口。
   - 新增 `SyncService::listen_port()` 和 `syncStore.loadListenPort()`。
   - 前端 `SyncPage` 在启用同步后展示本机监听端口，方便用户手动输入 `host:port`。
3. **手动 fallback 引导**：
   - 在 `SyncPage`「未发现设备」区域增加提示文案，告知用户可尝试同一 Wi-Fi 下使用 `host:port` 连接。
   - 新增 i18n 文案 `sync_your_port` 与 `sync_manual_fallback_hint`（中/英）。

**验证结果**

- `cargo clippy --lib -- -D warnings` 通过。
- `cargo clippy -p solosoul-sync -- -D warnings` 通过。
- `cargo test -p solosoul-sync --lib` 全部 17 项测试通过。
- 前端 `npx tsc --noEmit` 通过。

**遗留优化**

- 监听端口目前仅在页面加载/启用同步时刷新，未来可随状态变化自动刷新。
- 移动端 UI 尚未展示 NSD 注册端口，后续可补充。

**验收标准**

- ✅ 服务类型常量均有明确注释，测试能捕获不一致变更。
- 同一 Wi-Fi 下，Android 能发现桌面端，桌面端能发现 Android（需真机验证）。
- ✅ 发现失败时，用户可在 UI 上看到本机端口和手动连接指引。

### 4.2 第二阶段：账户级跨设备恢复（同步前准备） ✅ 已实现

**目标**

- 让新设备能够安全地恢复已有账户，然后再参与同步。
- 不破坏「零知识 / 本地优先」架构。

**已实现方案**

1. **协议设计**：
   - 新增 `solosoul-sync/src/recovery.rs`，基于现有 `Noise_XX_25519_ChaChaPoly_BLAKE2s` 建立加密通道。
   - 主机端生成 6 位 PIN + 32 字节随机恢复密码，将当前账户完整导出为内存/临时 `.solosoul` 包。
   - 新设备输入主机地址与 PIN 后连接，校验 PIN 并通过加密通道下载恢复包 + 恢复密码。
   - 使用 `VaultService::create_account_with_id` 在本地创建与主机**相同 account_id** 的账户，保证后续 Device Sync 可直接识别为同一账户。
2. **Tauri 命令**：
   - `recovery_host_start`：生成恢复包并启动监听主机，返回显示地址、PIN、QR payload。
   - `recovery_host_cancel`：取消正在运行的恢复主机并清理临时导出文件。
   - `recovery_restore_from_host`：客户端连接主机、下载恢复包、创建账户并导入。
3. **UI 入口**：
   - `SyncPage` 新增「关联新设备」按钮，打开 `RecoveryHostDialog` 展示二维码、PIN 与地址。
   - `OnboardingDialog` 新增「从其他设备恢复」入口，打开 `RecoveryReceiveDialog`。
   - `RecoveryReceiveDialog` 支持手动输入主机地址与 PIN。
4. **i18n**：新增中英双语恢复流程文案（`common:recovery_*`）。

**关键代码路径**

- 协议层：`tauri/crates/solosoul-sync/src/recovery.rs`
- 命令层：`tauri/src-tauri/src/commands/recovery.rs`
- 状态层：`tauri/src-tauri/src/state/app_state.rs`（新增 `RecoveryState`）
- UI 层：`tauri/src/components/recovery/RecoveryHostDialog.tsx`、`RecoveryReceiveDialog.tsx`
- 集成入口：`tauri/src/pages/sync/SyncPage.tsx`、`tauri/src/components/onboarding/OnboardingDialog.tsx`

**验证结果**

- `cargo clippy -p solosoul-sync -- -D warnings` 通过。
- `cargo clippy --lib -- -D warnings` 通过。
- `cargo test -p solosoul-sync --lib recovery` 4 项恢复测试通过。
- `npx tsc --noEmit` 通过。

**遗留风险与后续优化**

- **PIN 重放与 MITM**：当前 QR payload 仅包含地址和 6 位 PIN，未绑定主机临时公钥或时间戳。在受信任的局域网内使用是合理的，但在公共网络中存在重放和中间人风险。后续可考虑在 QR 中加入主机公钥指纹或一次性 nonce。
- **账户创建后导入失败**：`recovery_restore_from_host` 先创建账户再导入；若导入失败，会留下一个空账户。后续应把「创建账户 + 导入」包装为原子操作或失败回滚。
- **PIN 暴力破解**：6 位 PIN 空间较小，目前仅按连接限制 3 次尝试。后续应增加全局速率限制或更复杂的短码机制。
- **临时文件清理**：主机端在会话结束或取消时会删除临时导出文件；客户端下载后也会删除。需要更多边界测试（如应用崩溃、进程被 Kill）。

**验收标准**

- ✅ 已登录设备可在 2 步内开启「关联新设备」会话。
- ✅ 新设备可在 3 步内完成账户恢复（输入地址/PIN、设置新主密码、导入完成）。
- ✅ 恢复后两设备的 `account_id` 一致，可继续参与 Device Sync。

### 4.3 第三阶段：自动/半自动同步触发

**目标**

- 减少用户手动操作，提高数据一致性。

**建议方案**

1. **进入前台时自动同步一次**：App 切到前台时，如果同步已启用且距上次同步超过阈值（如 5 分钟），自动触发同步。
2. **关键数据变更后触发同步**：
   - 创建/更新对象后
   - 保存 profile 后
   - 删除对象后
   - 附件上传/下载后
3. **增加「自动同步开关」**：允许用户关闭自动同步，仅在点击时同步。
4. **借鉴 Android SAF AutoSync 的调度器**：复用 `AutoSyncManager` 的事件队列模式，为 Device Sync 增加 `SyncEvent::DataChanged`。

**验收标准**

- 用户在一台设备上修改数据后，另一台设备在合理时间内（如 30 秒内）可自动同步。
- 自动同步不会显著影响电池和性能。

### 4.4 第四阶段：冲突解决 UI

**目标**

- 在冲突发生时，允许用户查看并选择保留本地/远程版本。

**建议方案**

1. 后端在 `ConflictRecord` 中补充足够元数据（对象名、摘要、修改时间）。
2. 新增 `SyncConflictPage` 或弹窗，展示冲突列表。
3. 提供「全部保留本地」「全部保留远程」「逐项选择」三种操作。

**验收标准**

- 冲突发生时用户可感知。
- 用户可以手动选择版本并应用。

### 4.5 第五阶段：iOS 发现层补齐

**目标**

- 让 iOS 也能自动发现局域网内设备。

**建议方案**

1. 新增 iOS Swift 插件，封装 `NetService` / `NWListener` / `NWBrowser`。
2. 复用现有 `nsd_plugin.rs` 的接口定义。
3. 在 `solosoul-sync/src/mobile.rs` 中统一调用。

**验收标准**

- iOS 能发现桌面端和 Android 端。
- iOS 与桌面/Android 可正常完成同步。

---

## 5. 技术实现建议

### 5.1 服务类型跨平台处理

```rust
// manager.rs / discovery.rs
// mdns-sd 要求完整 mDNS 域名后缀；Android NsdManager 会自动补齐 .local.
const MDNS_SERVICE_TYPE: &str = "_solosoul._tcp.local.";
const SERVICE_TYPE_BASE: &str = "_solosoul._tcp";
```

```kotlin
// NsdPlugin.kt
// Android NsdManager 必须省略 ".local."，否则注册/发现会失败
private const val SERVICE_TYPE = "_solosoul._tcp"
```

建议新增一致性测试，确保 `MDNS_SERVICE_TYPE` 去除 `.local.` 后缀后与 Android 常量相等。

### 5.2 自动同步事件接入

在 `tauri/src-tauri/src/sync/auto_sync.rs` 旁新增 `device_sync/auto_sync.rs`，或扩展现有事件模型：

```rust
pub enum DeviceSyncEvent {
    Manual,      // 用户手动触发
    AppForeground, // 应用进入前台
    DataChanged,   // 数据变更
}
```

### 5.3 账户恢复凭证

- 使用 Noise 身份密钥派生一个短期的恢复密钥。
- 通过二维码展示恢复密钥指纹 + 账户加密包。
- 新设备扫描二维码后解密并恢复账户，然后自动参与同步。

### 5.4 冲突解决数据流

```
Backend: apply_sync_records() -> 冲突 -> ConflictRecord -> 持久化到 sync_conflicts 表
Frontend: 进入 SyncPage -> 拉取冲突列表 -> 用户选择 -> 调用新命令 apply_conflict_resolution() -> 更新本地记录
```

---

## 6. 测试与验证计划

### 6.1 单元测试补充

| 测试项 | 目标 |
|--------|------|
| 服务类型一致性 | 桌面/Android 使用同一基础服务类型字符串，测试能捕获差异 |
| 账户不匹配拒绝 | 不同 account_id 时握手被拒绝 |
| 自动同步触发 | 模拟数据变更事件，验证同步命令被调用 |

### 6.2 集成测试

| 场景 | 环境 | 预期 |
|------|------|------|
| 桌面 ↔ Android 自动发现 | 同一 Wi-Fi | 双方可发现对方并完成同步 |
| 手动 host:port 同步 | 关闭 mDNS/NSD | 手动输入地址可完成同步 |
| 跨设备账户恢复 | 新安装 Android | 通过二维码恢复账户后同步 |
| 自动同步触发 | 桌面修改对象 | Android 30 秒内自动同步 |

### 6.3 真机回归

- 至少覆盖 1 款 Android 真机和 1 台桌面设备。
- 验证 mDNS/NSD 在常见路由器下的可用性。
- 验证权限弹窗、发现、配对、同步全流程。

---

## 7. 风险与缓解

| 风险 | 影响 | 缓解 |
|------|------|------|
| mDNS/NSD 在某些网络下不可用 | 自动发现失败 | 保留手动 host:port 兜底；提供清晰提示 |
| 自动同步增加电量消耗 | 电池压力 | 采用防抖 + 增量同步；提供开关 |
| 账户恢复凭证泄露 | 安全威胁 | 恢复凭证设置短有效期 + 使用限制次数 |
| iOS 实现周期长 | 跨平台延迟 | 先完成 Android/桌面，再补齐 iOS |

---

## 8. 建议的里程碑排期

| 阶段 | 目标 | 预计工作量 |
|------|------|----------|
| P1 | 加固发现层一致性（注释/测试）、暴露监听端口、完善手动配对提示 | 1-2 周 |
| P2 | 账户恢复流程（二维码/短码） | 2-3 周 |
| P3 | Device Sync 自动同步触发 | 1-2 周 |
| P4 | 冲突解决 UI | 1-2 周 |
| P5 | iOS 发现层补齐 | 2-3 周 |

---

## 9. 结论

SoloSoul 的 Device Sync 在协议层和 UI 层已经具备了较完整的基础能力，但在**真实可用性**上仍处于「高级用户可用」的阶段。下一步开发应优先解决：

1. **跨平台发现稳定性**（明确并测试服务类型差异，优化手动 fallback）
2. **账户级跨设备恢复流程**（降低使用门槛）
3. **自动同步触发**（减少手动操作）

这三项完成后，Device Sync 才能从「功能可用」过渡到「普通用户可用」。

---

*最后更新：2026-07-28*
