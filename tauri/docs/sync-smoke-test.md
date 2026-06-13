# SoloSoul 设备同步冒烟测试指南

> 本文档描述如何在本地启动两个 SoloSoul 应用实例，验证端到端设备同步（mDNS 发现、Noise 握手、双向数据同步、附件同步、删除墓碑传播）。

---

## 前置条件

- macOS 或 Windows 开发环境已配置。
- 已执行 `npm install` 安装前端依赖。
- 已执行 `cargo build` 至少一次，确保 Rust 依赖已编译。
- 同一局域网（本地回环即可）可互通。

---

## 快速启动

项目已提供自动化脚本：

```bash
cd tauri
bash scripts/dev-two-instances.sh
```

脚本会：

1. 检测 `SOLOSOUL_SMOKE_DIR`（默认 `/tmp/solosoul-smoke`）下是否存在 device-a 账号数据。
2. 若不存在，先单独启动 device-a，提示你创建账号；创建后关闭窗口并按回车。
3. 将 device-a 数据复制到 device-b，保证两边 `account_id` 与主密码一致。
4. 同时启动两个 `tauri dev` 实例，分别使用：
   - device-a：数据目录 `/tmp/solosoul-smoke/device-a`，Vite 端口 `1420`
   - device-b：数据目录 `/tmp/solosoul-smoke/device-b`，Vite 端口 `1430`

---

## 手动验证步骤

### 1. 解锁两个实例

在两个窗口中使用同一主密码解锁同一账号。

### 2. 启用同步

在两个实例的 `SyncPage` 中打开“Enable Sync”，记录各自显示的 fingerprint。

### 3. 信任对端

- 若 mDNS 正常，未信任设备会出现在 Known Devices 列表中。
- 点击 `Trust`（P3.1 后替换为 PairingDialog 显式确认）。
- 对比对端 fingerprint 是否与对端窗口显示的一致。

### 4. 触发同步

在任一实例中：

- 输入对端 `host:port`（如 `127.0.0.1:<port>`，port 在 SyncPage 中不直接显示，可通过对端日志或 mDNS 获取），点击 Sync；或
- 直接点击已知设备行的 Sync 按钮（如果 UI 已提供）。

### 5. 验证数据一致

在 device-a 中：

- 创建一个 Profile、一个 Object、一个 User Template，并上传一个附件。
- 点击同步。

在 device-b 中：

- 等待同步完成，刷新对应页面，确认数据与附件均已到达。
- 在 device-a 删除该 Profile，再次同步，确认 device-b 中该 Profile 也被删除（验证 P2.2 删除墓碑）。

---

## 端口与数据目录自定义

脚本使用以下环境变量，可根据需要覆盖：

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `SOLOSOUL_SMOKE_DIR` | `/tmp/solosoul-smoke` | 测试数据根目录 |
| `SOLOSOUL_VITE_PORT` | `1420` / `1430` | Vite 开发服务器端口 |
| `SOLOSOUL_VITE_HMR_PORT` | `1421` / `1431` | Vite HMR WebSocket 端口 |
| `SOLOSOUL_DATA_DIR` | `~/.solosoul` | 应用数据目录 |

> 注意：两个实例的 `SOLOSOUL_DATA_DIR` 必须不同，否则会发生文件冲突。

---

## 故障排查

### 两个实例无法发现对方

- 确认两边 `account_id` 完全一致（通过复制同一账号数据实现）。
- 某些网络环境会屏蔽 mDNS，可直接使用 `host:port` 手动同步。
- 检查防火墙是否放行 `5353/udp` 以及随机 TCP 端口。

### 同步提示 "Peer is not trusted"

- 先在接收方信任发送方的 fingerprint，再在发送方信任接收方。
- 信任状态持久化在 `sync_peers` 表中，重新打开应用后仍然有效。

### 附件同步失败

- 检查 Object 的 `__attachments` 字段是否包含正确 `id`、`objectId`、`fileName`、`sizeBytes`。
- 确认附件文件存在于 `SOLOSOUL_DATA_DIR/<account_id>/attachments/<object_id>/<attachment_id>/` 下。
- 查看 Rust 日志中的 `Attachment exchange failed` 警告。

### Vite 端口冲突

- 如果 `1420/1421/1430/1431` 已被占用，修改脚本中的端口或使用环境变量覆盖。

---

## 关联文件

| 文件 | 说明 |
|------|------|
| `tauri/scripts/dev-two-instances.sh` | 双实例启动脚本 |
| `tauri/src-tauri/src/services/vault_service.rs` | `SOLOSOUL_DATA_DIR` 环境变量支持 |
| `tauri/vite.config.ts` | `SOLOSOUL_VITE_PORT` / `SOLOSOUL_VITE_HMR_PORT` 端口支持 |
| `tauri/crates/solosoul-sync/src/manager.rs` | mDNS 发现、Noise 握手、同步会话 |
| `tauri/src/pages/sync/SyncPage.tsx` | 同步设置页面 |
