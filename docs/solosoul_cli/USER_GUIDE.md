# SoloSoul CLI 用户手册

> 本文档面向 SoloSoul 终端用户（CLI TUI 客户端）。
> 适用版本：v2.3.x（unlock/lock/list/open/sync/ocr/embed_model 等全部命令）。

## 1. 安装与启动

| 平台 | 安装来源 |
|------|----------|
| macOS / Linux | `cargo install solosoul-cli`（Cargo crates）或源码编译 |
| Windows | MSI 安装包中的 `solosoul.exe` |

CLI 默认数据目录：
- Unix：`~/.solosoul/`
- Windows：`%USERPROFILE%\.solosoul\`

通过环境变量 `SOLOSOUL_DATA_DIR` 可重定向到任意目录（用于测试或多账户隔离）。

CLI/GUI **共占用进程锁**，同一时刻只能有一个进程打开数据目录。

## 2. 启动

```bash
solosoul                  # 启动 TUI 主页（或 Welcome 页）
solosoul --data-dir <DIR> # 指定数据目录
SOLOSOUL_DATA_DIR=<DIR> solosoul
```

CLI 启动时：
- 无本地账户 → 进入 **Welcome** 页面，按 Enter 进入创建账户向导。
- 有账户但未解锁 → **Locked** 页面，可执行 `/unlock`、`/account_list`、`/doctor`、`/exit`。
- 已解锁 → **Home** 页，显示 6 张快捷卡片（对象、搜索、模板、附件、备份、OCR 等）。

## 3. 全局快捷键

| 键 | 行为 |
|----|------|
| `↑` / `↓` | 命令历史 / 菜单上下选择 |
| `Tab` | 命令补全 |
| `Enter` | 执行命令或确认输入 |
| `Esc` | 退出向导 / 清空输入 / 关闭错误弹窗 |
| `Ctrl+L` | 手动锁定 Vault |
| `Ctrl+C` | 强制退出 CLI |
| 鼠标左键 | 单击快捷卡片 / 命令按钮 |
| 鼠标滚轮 | 滚动列表 |
| 鼠标拖拽 | 卡片/Lock 页内滚动 |

## 4. 命令清单

### 4.1 账户与解锁

| 命令 | 说明 |
|------|------|
| `/unlock` 或 `/login` | 进入登录向导（单账户直接跳到密码页） |
| `/lock` 或 `/logout` | 立即锁定 Vault |
| `/account_list` | 显示本地账户列表 |
| `/doctor` | 运行环境诊断（Vault 状态、进程锁、模型完整性、依赖等） |
| `/exit` | 退出 CLI |
| `/back` | 返回上一屏 |

### 4.2 数据对象

| 命令 | 说明 |
|------|------|
| `/list` | 显示当前账户的所有对象（页面+独立对象） |
| `/open <id>` | 打开对象详情页 |
| `/newpage` | 创建新页面 |
| `/newobject` | 进入创建对象向导 |
| `/edit <id>` | 编辑对象字段（向导） |
| `/delete <id>` | 软删除对象（进入回收站） |
| `/search <query>` | 在解密字段中流式搜索（命中 200 条截断） |
| `/size` / `/status` / `/state` | 显示账户统计报告 |

### 4.3 回收站

| 命令 | 说明 |
|------|------|
| `/trash` 或 `/bin` | 回收站列表（键盘 `r` 恢复、`p` 永久删除、空格多选） |
| `/restore <id>` | 恢复单个对象 |
| `/purge <id>` | 永久删除单个对象 |

### 4.4 历史与审计

| 命令 | 说明 |
|------|------|
| `/history <id>` | 对象快照历史 |
| `/rollback <id> <ver>` | 回滚到指定快照版本 |
| `/operation_log [account_id]` | 显示审计日志 |
| `/export_log <path>` | 导出审计日志 |
| `/debug_log` | 导出诊断包（脱敏） |

### 4.5 附件

`/attach <subcommand>`：
- `list <obj_id>`         —— 列出对象附件
- `add <obj_id> <path>`   —— 添加附件
- `rename <obj_id> <aid> <new>` —— 重命名
- `delete <obj_id> <aid>` —— 软删除
- `restore <obj_id> <aid>` —— 恢复
- `purge <obj_id> <aid>`  —— 永久删除
- `cleanup <obj_id>`      —— 清理已永久删除项

### 4.6 备份

`/backup <subcommand>`：
- `list`                 —— 列出当前账户备份
- `create [name]`        —— 创建新备份
- `restore <name>`       —— 恢复（强制 Y/n 确认）
- `delete <name>`        —— 删除备份

### 4.7 加密导出/导入

| 命令 | 说明 |
|------|------|
| `/export` | 加密 ZIP 导出（`.solosoul`，包含 `manifest.json`、`payload.enc`、可选 `attachments/`） |
| `/import` | 导入 `.solosoul` 包 |

导出密码通过模态提示采集，**不允许与主密码**相同。

### 4.8 设置与安全

| 命令 | 说明 |
|------|------|
| `/language`            | 切换 CLI 语言 |
| `/theme`               | 切换主题 |
| `/setting <key> <val>` | 修改加密的账户偏好 |
| `/security`            | 修改主密码 / 提示词 / 回收站保留天数 / 删除账户 |
| `/debug_log`           | 导出诊断包 |

### 4.9 LLM

| 命令 | 说明 |
|------|------|
| `/model`                       | 切换默认 provider/model |
| `/llm_config`                  | 列出当前 LLM 配置 |
| `/llm_stats`                   | 用量统计 |
| `/llm_list_conversations`      | 列出对话历史 |
| `/llm_conversations`           | 同上 |
| `/llm_chat [model_id]`         | 进入 CLI 聊天 REPL（流式响应） |

### 4.10 插件

| 命令 | 说明 |
|------|------|
| `/plugin` 或 `/plugin_list` 或 `/plugin-market` 或 `/plugin_market` | 插件列表（可按名称过滤） |
| `/plugin_run <id> [args...]` | 运行插件 |
| `/plugin_install <id>` | 安装 |
| `/plugin_update <id>` | 更新 |
| `/plugin_uninstall <id>` | 卸载 |
| `/plugin_sessions` | 列出插件会话 |
| `/plugin_list_installed` | 已安装列表 |
| `/plugin_audit_log` | 插件审计日志 |
| `/plugin_registry_update` | 更新本地市场 registry |
| `/plugin_search <kw>` | 按关键词搜索 |

### 4.11 设备同步  ← *本期新增*

`/sync <subcommand> [args]`：

| 子命令 | 说明 |
|--------|------|
| `status` / `list` | 列出当前账户 vault 已持久化的 peers |
| `with <peer-or-host:port>` | 一次性向指定 peer 发起同步（start→sync→stop） |
| `trust <peer>` | 将 peer 标记为受信任 |
| `untrust <peer>` | 取消 trust |
| `forget <peer>` | 从 vault 中移除 peer |
| `help` | 帮助 |

> **运行时说明**：`/sync with` 创建一次性 tokio runtime，会话结束后释放。
> "始终在线"后台同步请使用 **GUI**（CLI 不维持 mDNS listener 守护进程）。
>
> Sync identity（node_id + NoiseKeys）在 vault 中以原始 `[u8;32]` 存储，
> 与 Tauri `SyncService::new(Arc<RwLock<VaultService>>)` 完全兼容。

### 4.12 本地 OCR  ← *本期新增*

`/ocr <subcommand> [args]`：

| 子命令 | 说明 |
|--------|------|
| `tiers` | 列出 tiny / small / medium 档位及其本地安装状态 |
| `scan <path>` | 对本地图片执行 PP-OCRv6 识别 |
| `scan --mrz <path>` | 护照 MRZ 结构化识别（证件号/国籍/有效期 + checksum） |
| `status` | 当前模型目录与已安装档位 |
| `help` | 帮助 |

环境变量 `SOLOSOUL_OCR_TIER=tiny|small|medium` 控制档位（默认 `small`）。
模型目录：`{SOLOSOUL_DATA_DIR}/models/pp-ocr-v6-{tier}/`。
若档位未安装，CLI 会提示通过 GUI 安装或手动放置。

### 4.13 Embedding 模型  ← *本期新增*

`/embed_model <subcommand> [args]`：

| 子命令 | 说明 |
|--------|------|
| `list` | 列出本地目录中的 embedding 模型 |
| `install <model_id>` | 从注册表下载并安装（reqwest + sha256 校验） |
| `remove <model_id>` | 删除本地模型目录 |
| `status` | 显示本地目录（CLI 不直接读写 GUI 端 LlmConfig） |
| `help` | 帮助 |

环境变量 `SOLOSOUL_EMBED_REGISTRY=https://...embed-registry.json` 覆盖默认 URL。
本地目录：`{SOLOSOUL_DATA_DIR}/embed_models/<model_id>/model.bin`。
激活的 embedding model 仍由 **GUI** 的 LlmConfig 设置，CLI 当前不修改。

## 5. 进程锁与并发

CLI 启动时获取 `solosoul_core::ProcessLock`（基于 `fs2` 文件锁），持有期间：
- 状态栏显示 `🔒 进程锁已持有 · GUI 不可用`。
- GUI 启动会立刻拒绝并提示 CLI 正在使用。

退出 CLI（`/exit`、`/logout` 或 force-quit）时锁自动释放。

## 6. 自动锁定

- 已登录状态下 5 分钟无键盘输入自动锁定 Vault，会话密钥 zeroize；
- 模态提示（密码、确认对话框）打开期间暂停自动锁定计时；
- 状态栏显示倒计时（`锁定倒计时: 240s`）。

## 7. 日志

CLI 日志写入 `{DATA_DIR}/logs/cli.log`，**不输出主密码或 session key**。
`/doctor` 中列出日志路径，便于排错。

## 8. 常见问题

| 问题 | 解决方案 |
|------|----------|
| "无本地账户" | 第一次启动请在 Welcome 页按 Enter 走创建账户向导 |
| "Vault 未解锁" | 先执行 `/unlock` 或在登录向导中输入密码 |
| `/ocr scan` 报"模型未安装" | 通过 GUI 安装或放置模型到 `models/pp-ocr-v6-{tier}/` |
| `/embed_model install` 报"注册表 schema 不匹配" | 假定 schema `{"models":[{id,name,size_mb,sha256,download_url}]}`；现网注册表协议可能不同，见 §4.13 风险说明 |
| `/sync with` 卡顿 | 后台 mDNS/TCP listener 立即被 `stop()`，但 mDNS 浏览线程最多丢 200ms 后干净退出 |
| GUI 启动说"CLI 持有锁" | 退出 CLI 或等待 5 分钟自动锁定并停止持有锁 |

## 9. 命令兼容性

以下命令与 Tauri GUI 端 1:1 对齐（路径、参数、错误字符串）：

- 所有 §4.1、§4.2、§4.3 命令
- `/search` / `/history` / `/rollback` / `/operation_log`
- `/attach` / `/backup` / `/export` / `/import`
- `/security` / `/setting` / `/language` / `/theme`
- `/template` / `/model` / `/llm_*` / `/plugin*`

新增命令（/sync /ocr /embed_model）目前以 CLI 端接口为主，UI GUI 可在后续迭代补齐对应面板。

## 10. 发布与下载

SoloSoul CLI 二进制随 `vX.Y.Z` Tag 自动发布到 GitHub Releases：
- `artifacts/cli/macos-aarch64/solosoul` — macOS arm64
- `artifacts/cli/macos-x86_64/solosoul` — macOS x86_64
- `artifacts/cli/windows/solosoul.exe` — Windows x86_64

⚠️ **macOS CLI 当前未公证**（未签约 / 未 notarize）。首次启动可能被 Gatekeeper 隔离。应急跳过命令：

```bash
which solosoul                                           # 先确认实际安装路径 (Homebrew/源码可能不在 /usr/local/bin)
xattr -dr com.apple.quarantine "$(which solosoul)"        # 只对单个文件，避免误伤同目录其它二进制
```

或右键 → 打开 → 确认。正式分发需 codesign + notarize,留待后续 PR。

⚠️ **Windows CLI 同样未签名**。未签名的 PE 在 Win10/11 上会被 SmartScreen 拦截（“Windows protected your PC”），点 “More info → Run anyway” 可跳过。正式分发需 EV 代码签名证书或加入微软 ISV 认证。
