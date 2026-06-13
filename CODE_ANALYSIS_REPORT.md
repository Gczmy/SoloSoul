# 代码分析修复报告

> 最后更新：2026-06-13 02:50:00
> 当前分支：`master`
> 修复轮次：1（初始分析）

## 摘要

本次审查覆盖 Tauri v2 客户端的前端（TypeScript/React）与后端（Rust）。
基线检查：`tsc --noEmit` 通过，`cargo fmt --check` 通过，`cargo clippy -- -D warnings` 通过，
`npm run lint` 存在 10 个 warning，`cargo test --workspace` 271 个测试全部通过，
`npm run test` 124 个测试全部通过。

共识别出 **45 个问题**，其中 P0 13 个、P1 22 个、P2 10 个。
本报告优先处理可在最小侵入前提下修复的高影响问题；对于需要架构级重构的 P0（如 Vault 应用层加密、Noise 协议完整实现），先以临时缓解措施处理并标记为“需架构重构”。

## 问题清单（按优先级 P0 > P1 > P2）

| ID   | 优先级 | 类别       | 文件位置                                        | 描述                                           | 状态      |
|------|--------|------------|-------------------------------------------------|------------------------------------------------|-----------|
| R001 | P0     | 漏洞       | `tauri/src-tauri/src/commands/vault.rs:44-51`  | `delete_account` 接收 `_password` 但未校验密码即可删除账户 | `[x]` 已修复 |
| R002 | P0     | 漏洞/架构  | `tauri/crates/solosoul-sync/src/noise.rs:35-60` | Noise IX 握手为空实现，仅交换公钥无密钥派生/加密 | `[ ]` 暂缓 |
| R003 | P0     | 漏洞/架构  | `tauri/crates/solosoul-vault/src/storage.rs`    | 敏感数据以明文 JSON/BLOB 存入 SQLite，与零知识架构声明不符 | `[ ]` 暂缓 |
| R004 | P0     | 漏洞/性能  | `tauri/src-tauri/src/commands/crypto.rs:51-66`  | `derive_key` 允许前端无限制指定 Argon2 参数，可造成 DoS | `[x]` 已修复 |
| R005 | P0     | 漏洞       | `tauri/src-tauri/src/commands/biometric.rs:34-57` | 生物识别主密钥使用硬编码 XOR 混淆后明文落盘 | `[x]` 部分修复 |
| F001 | P0     | 漏洞/架构  | 多处（见详细描述）                              | 组件卸载时未清理 setTimeout/全局事件监听器，存在内存泄漏与竞态 | `[ ]` 待修复 |
| F002 | P0     | 漏洞/架构  | 多处（见详细描述）                              | 异步 IPC/HTTP 请求无取消/防竞态保护 | `[ ]` 待修复 |
| F003 | P0     | 漏洞       | `tauri/src/hooks/useRevealState.ts:16-38`      | `reveal` 创建的 setTimeout 未在卸载时清理 | `[x]` 已修复 |
| F004 | P0     | 架构/漏洞  | `tauri/src/stores/llmStore.ts:31-50`           | `startStream` 每次重复订阅 `listen`，`reset()` 未取消监听 | `[x]` 已修复 |
| F005 | P0     | 架构       | `tauri/src/stores/settingsStore.ts:369`        | `clearOnVaultLock` 将语言等 UI 偏好重置为默认值 | `[x]` 已修复 |
| F006 | P0     | 性能/架构  | `tauri/src/App.tsx:106-144`                    | 系统主题媒体查询监听未清理，且持有陈旧 config 闭包 | `[x]` 已修复 |
| F007 | P0     | 性能/漏洞  | `tauri/src/pages/workspace/ObjectWorkspacePage.tsx:211-228` | 计数 effect 依赖 `visibleObjects.length` 而非 ID 列表 | `[x]` 已修复 |
| F008 | P0     | 性能       | `tauri/src/pages/ai/LlmChatPage.tsx:248-265`   | effect 将整个 `llmStore` 对象放入依赖数组 | `[x]` 已修复 |
| R006 | P1     | 漏洞       | `tauri/src-tauri/src/commands/auth.rs:111`     | `verify_password_core` 使用 `==` 比较哈希，存在时序侧信道 | `[x]` 已修复 |
| R007 | P1     | 漏洞       | `tauri/src-tauri/src/commands/attachment.rs:209-224` | 附件保存直接使用前端 `file_name` 拼接路径 | `[ ]` 待修复 |
| R008 | P1     | 漏洞       | `tauri/src-tauri/src/commands/export_import.rs:1068-1076` | 导入备份附件时使用包内 `file_name` 直接落盘 | `[ ]` 待修复 |
| R009 | P1     | 漏洞       | `tauri/src-tauri/src/commands/backup.rs:76-86,144-168,216-234` | 备份文件名未校验；恢复/删除使用前缀匹配 | `[ ]` 待修复 |
| R010 | P1     | 漏洞       | `tauri/src-tauri/src/commands/system.rs:59-140` | `download_update` 允许前端指定文件名写入 Downloads | `[ ]` 待修复 |
| R011 | P1     | 漏洞       | `tauri/src-tauri/src/commands/log.rs:97-118`   | `log_export` 可写入任意路径 | `[ ]` 待修复 |
| R012 | P1     | 漏洞       | `tauri/src-tauri/src/commands/fs.rs:10-128`    | `encrypt_file` 等命令可直接读写任意路径 | `[ ]` 待修复 |
| R013 | P1     | 漏洞       | `tauri/src-tauri/src/commands/embed_model.rs:97-104` | `llm_delete_embed_model` 的 `model_id` 可导致路径穿越 | `[ ]` 待修复 |
| R014 | P1     | 漏洞/规范  | `tauri/src-tauri/src/commands/crypto.rs:70-75` | `generate_salt` / `derive_key` 返回未 `Zeroizing` 且长度无上限 | `[x]` 已修复 |
| R015 | P1     | 漏洞/架构  | `tauri/src-tauri/src/services/vault_service.rs:486-489` | 修改主密码后未重新加密已有数据 | `[ ]` 暂缓 |
| R016 | P1     | 漏洞/性能  | `tauri/crates/solosoul-sync/src/transport.rs:58-83` | 同步传输未限制帧长度，可触发 OOM | `[x]` 已修复 |
| R017 | P1     | 漏洞/死代码 | `tauri/src-tauri/src/state/session_state.rs`    | 会话密钥以普通 `Vec<u8>` 存储且模块无人使用 | `[x]` 已修复 |
| R018 | P1     | 架构/性能  | `tauri/src-tauri/src/state/app_state.rs`       | 异步上下文中持有同步锁并执行 IO | `[ ]` 待修复 |
| R019 | P1     | 性能       | `tauri/src-tauri/src/commands/fs.rs:10-57`     | 文件加解密命令整读整写大文件 | `[ ]` 待修复 |
| R020 | P1     | 性能       | `tauri/src-tauri/src/commands/crypto.rs:85-154` | `get_vault_stats` 对对象做 N+1 查询 | `[ ]` 待修复 |
| R021 | P1     | 漏洞       | `tauri/src-tauri/src/commands/llm.rs:228-274`  | LLM API Key 以明文形式存储在 Profile 中 | `[ ]` 待修复 |
| R022 | P1     | 漏洞       | `tauri/src-tauri/src/services/vault_service.rs:203-224` | Vault 文件/目录未按安全策略设置 0700/0600 权限 | `[x]` 已修复 |
| R023 | P1     | 规范       | `tauri/crates/solosoul-vault/src/storage.rs:2633` | 测试中使用 `serde_json::json!(3.14)` 触发 clippy approx_constant | `[x]` 已修复 |
| R024 | P1     | 并发/架构  | `tauri/src-tauri/src/services/vault_service.rs:159-181` | `create_account` 名称重复检查存在竞态 | `[ ]` 待修复 |
| R025 | P1     | 漏洞/架构  | `tauri/src-tauri/src/commands/object.rs:152-164` | `object_create` 允许客户端指定 ID 并可能覆盖已有对象 | `[ ]` 待修复 |
| R026 | P1     | 漏洞       | `tauri/src-tauri/src/commands/biometric.rs:201` | 生物识别 `verify_password` 使用 `!=` 比较哈希 | `[x]` 已修复 |
| F009 | P1     | 死代码     | 多处（见详细描述）                              | ESLint 报告的 10 处未使用 import/变量 | `[x]` 已修复 |
| F010 | P1     | 规范/质量  | 多个超大组件（见详细描述）                      | 组件超过 50 行有效代码，职责过多 | `[ ]` 暂缓 |
| F011 | P1     | 性能       | `tauri/src/pages/workspace/ObjectWorkspacePage.tsx:178-191,463-502` | 字段元数据查询未缓存，渲染复杂度 O(n²) | `[ ]` 待修复 |
| F012 | P1     | 性能       | `tauri/src/components/object/ObjectDetailModal.tsx:200-219,343-436` | 同样存在字段元数据 O(n) 重复查询 | `[ ]` 待修复 |
| F013 | P1     | 规范/质量  | `tauri/src/components/layout/SearchPopover.tsx:81-95` 等 | `Highlight` 使用不稳定 key | `[ ]` 待修复 |
| F014 | P1     | 漏洞       | `tauri/src/pages/help/HelpPage.tsx:54-71`      | 多次点击重试会产生重叠 interval | `[x]` 已修复 |
| F015 | P1     | 架构       | `tauri/src/pages/editor/ObjectEditorPage.tsx:96-101` | 初始模板选择只计算一次，异步加载后未更新 | `[ ]` 待修复 |
| F016 | P1     | 漏洞       | `tauri/src/pages/editor/ObjectEditorPage.tsx:133-177` | 回填 effect 缺少 `objectId` 依赖 | `[ ]` 待修复 |
| F017 | P1     | 漏洞       | `tauri/src/pages/auth/LoginPage.tsx:32-49,64-87` | 异步 effect 无 cleanup，切换账号存在竞态 | `[ ]` 待修复 |
| F018 | P1     | 规范/质量  | `tauri/src/components/layout/SideNavigation.tsx:116-119` | render 阶段修改 ref | `[ ]` 待修复 |
| F019 | P1     | 架构       | `tauri/src/components/layout/SideNavigation.tsx:747-753` | 重命名自定义页面时 `accountId` 传空字符串 | `[ ]` 待修复 |
| F020 | P1     | 架构       | `tauri/src/components/layout/SideNavigation.tsx:740-753` | 自定义页面更新未做失败回滚 | `[ ]` 待修复 |
| F021 | P1     | 性能       | `tauri/src/pages/settings/ExportImportPage.tsx:499-523` | 每 render 重复计算的 IIFE | `[ ]` 待修复 |
| F022 | P1     | 架构       | `tauri/src/stores/authStore.ts:102-105`        | `logout` 未清空账户列表与 hasAccount | `[ ]` 待修复 |
| R027 | P2     | 死代码     | 多处（见详细描述）                              | 死代码模块/函数 | `[ ]` 待修复 |
| R028 | P2     | 规范       | `tauri/src-tauri/src/commands/mod.rs:1` 等     | 全局 `allow(dead_code)` / `allow(unused_imports)` 掩盖问题 | `[ ]` 待修复 |
| R029 | P2     | 规范       | `tauri/src-tauri/src/commands/export_import.rs:1053-1065` | 导入附件加密方式通过试错回退 | `[ ]` 待修复 |
| R030 | P2     | 架构/死代码 | `tauri/crates/solosoul-vault/src/lib.rs:19-34` | `VaultConfig.sqlcipher_key` 声明但从未使用 | `[ ]` 待修复 |
| R031 | P2     | 漏洞       | `tauri/src-tauri/src/commands/system.rs:110-119` | 下载更新文件在 macOS 被设为全局可读 0o644 | `[x]` 已修复 |
| F023 | P2     | 规范/质量  | `tauri/src/pages/settings/TemplateManagerPage.tsx:707-720,891-904` | 字段图标映射重复定义 | `[ ]` 待修复 |
| F024 | P2     | 性能       | `tauri/src/pages/settings/TemplateManagerPage.tsx:571,704` | 编辑字段时使用 `findIndex` 导致 O(n²) | `[ ]` 待修复 |
| F025 | P2     | 规范/质量  | `tauri/src/pages/settings/TemplateManagerPage.tsx:680-780` | 深层 JSX 嵌套超过 4 层 | `[ ]` 待修复 |
| F026 | P2     | 漏洞       | `tauri/src/pages/ai/LlmChatPage.tsx:69-71` 等  | 使用 `Math.random()` 生成会话 ID | `[ ]` 待修复 |
| F027 | P2     | 规范/质量  | `tauri/src/pages/ai/LlmChatPage.tsx:582,588,698` | 使用翻译字符串作为错误消息标记 | `[ ]` 待修复 |
| F028 | P2     | 性能       | `tauri/src/pages/ai/LlmChatPage.tsx:185-187`   | 每条消息变化都触发平滑滚动 | `[ ]` 待修复 |
| F029 | P2     | 架构       | `tauri/src/lib/notification.ts:46-47`          | 强依赖全局 DOM 判断当前页面 | `[ ]` 待修复 |
| F030 | P2     | 死代码/架构 | `tauri/src/stores/vaultStore.ts:26-34`         | `vaultStore` 命名与实际行为不符且疑似未使用 | `[ ]` 待修复 |
| F031 | P2     | 架构       | `tauri/src/stores/profileStore.ts:32-40`       | `loadProfile` 未真正加载 sections | `[ ]` 待修复 |
| F032 | P2     | 漏洞       | `tauri/src/stores/settingsStore.ts:88-110` 等  | localStorage 解析未做 Schema 校验 | `[ ]` 待修复 |

## 修复进度

- 已完成：19 / 45
- 当前处理：无

## 本轮修复说明（轮次 1）

| ID   | 修复内容 |
|------|----------|
| F009 | 删除 10 处 ESLint 报告的未使用 import/变量（SearchPopover、HistoryViewer、ObjectEditorPage、HelpPage、AppearanceSettingsPage、ObjectWorkspacePage） |
| F003 | 在 `useRevealState` 中添加 `useEffect` cleanup，组件卸载时清理所有 pending timers |
| F005 | `settingsStore.clearOnVaultLock` 改为保留 UI 偏好（theme/language/accent/windowSize 等），仅重置账户相关状态；同步更新对应单元测试 |
| F006 | `App.tsx` 系统主题监听 effect 返回 cleanup 调用 `stopListeningForSystemTheme()`；回调内读取最新 settings，避免闭包陈旧 |
| F007 | `ObjectWorkspacePage` 计数 effect 依赖从 `visibleObjects.length` 改为 `visibleObjects.map(o => o.id).join(',')` |
| F008 | `LlmChatPage` stream error effect 移除 `llmStore` 整体依赖，改为 `useLlmStore.getState().reset()` |
| F004 | `llmStore` 增加 `unlistenPromise` 状态，`startStream`/`stopStream`/`reset` 同时取消已 resolved 和 pending 的监听器 |
| F014 | `HelpPage` 移除未使用的 `indexLoadingElapsed` 与 `setInterval`，避免重复启动 interval |
| R001 | `delete_account` 命令先读取账户 `config.json` 并调用 `verify_password_core` 校验密码，失败则拒绝删除 |
| R006 | `auth::verify_password_core` 使用 `solosoul_crypto::secure::secure_compare` 进行恒定时间比较 |
| R026 | `biometric::verify_password` 复用 `auth::verify_password_core`，消除非恒定时间比较 |
| R004 | `crypto::derive_key` 增加 Argon2 参数上限（memory_kb ≤ 64MiB、iterations ≤ 10、parallelism ≤ 16） |
| R014 | `crypto::generate_salt` 限制最大长度 64 字节，防止超大内存分配 |
| R022 | `vault_service.rs` 创建账户目录/文件与 `accounts.json` 时设置 0700/0600 权限；新增 Unix/非 Unix 兼容辅助函数 |
| R005 | `biometric.rs` 保存 `biometric_key` 后设置文件权限 0600；完全修复需迁移到 OS Keychain |
| R023 | `storage.rs` 测试中将 `3.14` 替换为 `std::f64::consts::PI`，消除 clippy approx_constant |
| R031 | `system.rs` 下载更新文件后设置权限 0o600（原 0o644） |
| R016 | `solosoul-sync/transport.rs` 接收帧时限制最大长度 64MB，防止恶意 peer 触发 OOM |
| R017 | 删除死代码 `state/session_state.rs` 与 `state/vault_state.rs`，并从 `state/mod.rs` 移除对应声明 |

## 详细问题描述与修复指引

### R001 · `delete_account` 未校验密码即可删除账户
**位置**：`tauri/src-tauri/src/commands/vault.rs:44-51`
**影响**：任何能在前端调用该命令的代码都可以删除账户及本地数据。
**修复**：在 `delete_account` 中先调用 `verify_password` 校验密码；删除前可擦除 session key。

### R002 · 同步模块 Noise 加密握手为空实现
**位置**：`tauri/crates/solosoul-sync/src/noise.rs:35-60`
**影响**：同步数据在传输层未加密，与文档声明的 Noise IX 模式不符。
**修复**：使用 `snow` Builder 构建真正的 Noise IX 会话；或移除 Noise 相关依赖与错误声明。
**状态说明**：属于架构级重构，需重新设计同步握手流程，本次暂缓。

### R003 · Vault 中敏感数据以明文形式落盘
**位置**：`tauri/crates/solosoul-vault/src/storage.rs`
**影响**：任何能读取本地 `vault.db` 的进程可直接获取用户敏感信息。
**修复**：在写入前使用会话密钥通过 `solosoul_crypto::cipher` 加密敏感字段，读取时解密；或启用并正确传递 SQLCipher 密钥。
**状态说明**：涉及所有 CRUD 路径与数据迁移，属于架构级重构，本次暂缓。

### R004 · `derive_key` 允许前端无限制指定 Argon2 参数
**位置**：`tauri/src-tauri/src/commands/crypto.rs:51-66`
**影响**：攻击者可传入极大参数触发内存耗尽或长时间 CPU 占用。
**修复**：增加硬上限（memory_kb ≤ 64*1024、iterations ≤ 10、parallelism ≤ 16）。

### R005 · 生物识别主密钥使用固定字符串 XOR 混淆后明文落盘
**位置**：`tauri/src-tauri/src/commands/biometric.rs:34-57`
**影响**：获取文件者可立即恢复主密钥。
**修复**：迁移到 OS Keychain/Secure Enclave 安全存储；短期缓解为设置 `biometric_key` 文件权限 0600。
**状态说明**：本次先实施文件权限 0600；完整修复需迁移到 Keychain。

### F001 · 卸载时事件监听器与定时器未清理
**位置**：`src/components/layout/SideNavigation.tsx`、`src/components/object/HistoryViewer.tsx`、`src/components/object/AttachmentViewer.tsx`、`src/components/object/ObjectDetailModal.tsx`、`src/components/guide/GuideCodeBlock.tsx`、`src/pages/system/AboutPage.tsx`
**影响**：内存泄漏、竞态、已卸载组件上 setState。
**修复**：统一使用 `useEffect` + ref 保存 timeout id，在 cleanup 中清理。

### F002 · 异步数据请求无取消/防竞态保护
**位置**：`src/pages/workspace/ObjectWorkspacePage.tsx`、`src/components/object/ObjectDetailModal.tsx`、`src/components/object/HistoryViewer.tsx`、`src/pages/help/HelpPage.tsx`、`src/pages/editor/HistoryPage.tsx`、`src/pages/settings/ExportImportPage.tsx`
**影响**：旧请求覆盖新数据、对已卸载组件 setState。
**修复**：引入 `AbortController` 或 `let cancelled = false` 标志。

### F003 · `useRevealState` 组件卸载后仍在已卸载组件上 setState
**位置**：`src/hooks/useRevealState.ts:16-38`
**影响**：React 警告、timers 持续累积。
**修复**：在 hook 内添加 `useEffect`，组件卸载时遍历 `timersRef.current` 并 `clearTimeout`。

### F004 · `llmStore` 将 `listen` 监听器常驻且不可清理
**位置**：`src/stores/llmStore.ts:31-50`
**影响**：监听器只增不减，可能导致重复处理 stream chunk。
**修复**：await 上一个 `listen` 的 Promise 再设置新的 unlisten，或在 store 初始化时只订阅一次。

### F005 · `settingsStore.clearOnVaultLock` 将语言等 UI 偏好重置为默认值
**位置**：`src/stores/settingsStore.ts:369`
**影响**：Vault 锁定后语言闪变。
**修复**：锁定后只清除敏感/账户级状态，保留 UI 偏好（theme、language、accent 等）。

### F006 · `App.tsx` 系统主题监听未清理，且持有陈旧 config 闭包
**位置**：`src/App.tsx:106-144`、`src/lib/theme.ts:101-123`
**影响**：媒体查询监听器永久泄漏；系统切换主题时使用旧的 defaultLightTheme/defaultDarkTheme。
**修复**：在 `useEffect` cleanup 中调用 `stopListeningForSystemTheme()`；在回调中读取最新 settings。

### F007 · `ObjectWorkspacePage` 列表项计数依赖 `visibleObjects.length` 而非内容
**位置**：`src/pages/workspace/ObjectWorkspacePage.tsx:211-228`
**影响**：列表长度不变但 ID 改变时不重新拉取；长度变化时频繁触发 IPC。
**修复**：依赖改为 `visibleObjects.map(o => o.id).join(',')`。

### F008 · `LlmChatPage` 将整个 `llmStore` 对象放入 effect 依赖数组
**位置**：`src/pages/ai/LlmChatPage.tsx:248-265`
**影响**：effect 被反复触发，存在消息重复保存或无限循环风险。
**修复**：只解构需要的原子状态作为依赖。

### R006 · `verify_password_core` 使用非恒定时间比较
**位置**：`tauri/src-tauri/src/commands/auth.rs:111`
**影响**：存在时序侧信道风险。
**修复**：使用 `solosoul_crypto::secure::secure_compare` 对字节切片进行恒定时间比较。

### R007-R013 · 多处路径穿越/任意文件读写
**位置**：`attachment.rs`、`export_import.rs`、`backup.rs`、`system.rs`、`log.rs`、`fs.rs`、`embed_model.rs`
**影响**：目录穿越、任意文件覆盖、删除。
**修复**：对文件名/ID 做白名单校验；使用精确匹配；限制可写目录。

### R014 · `generate_salt` / `derive_key` 返回的密钥未做安全擦除且长度无限制
**位置**：`tauri/src-tauri/src/commands/crypto.rs:70-75`
**影响**：密钥可能在内存中残留；超大 salt 可触发 OOM。
**修复**：返回 `Zeroizing<Vec<u8>>`；限制 `length <= 64`。

### R015 · 修改主密码后未重新加密已有数据
**位置**：`tauri/src-tauri/src/services/vault_service.rs:486-489`
**影响**：已有加密 blob 仍使用旧密钥，导致后续解密失败。
**修复**：修改密码后遍历资源，用旧密钥解密、新密钥加密后写回。
**状态说明**：依赖 R003 的加密实现，暂缓。

### R016 · 同步传输未限制帧长度
**位置**：`tauri/crates/solosoul-sync/src/transport.rs:58-83`
**影响**：恶意 peer 发送大长度值可触发 OOM。
**修复**：设置最大帧长度（如 64 MB）。

### R017 · 会话密钥以普通 `Vec<u8>` 存储且相关状态模块无人使用
**位置**：`tauri/src-tauri/src/state/session_state.rs`、`tauri/src-tauri/src/state/vault_state.rs`
**影响**：内存中密钥不被安全擦除；死代码增加维护负担。
**修复**：删除死代码文件。

### R018 · 在异步上下文中持有同步锁且长时间阻塞运行时
**位置**：`tauri/src-tauri/src/state/app_state.rs`、几乎所有命令
**影响**：阻塞 async worker 线程，降低并发能力。
**修复**：将 `VaultService` 内部改为 `tokio::sync::RwLock`，或在 `spawn_blocking` 中执行同步 IO。

### R019 · 文件加解密命令整读整写大文件
**位置**：`tauri/src-tauri/src/commands/fs.rs:10-57`
**影响**：GB 级文件会耗尽内存。
**修复**：使用 `std::fs::File` 分块读写。

### R020 · `get_vault_stats` 对对象做 N+1 查询
**位置**：`tauri/src-tauri/src/commands/crypto.rs:85-154`
**影响**：对象多时触发大量数据库查询。
**修复**：在 `VaultStore` 增加聚合统计接口，使用 SQL JOIN / 单次扫描。

### R021 · LLM API Key 以明文形式存储在 Profile 中
**位置**：`tauri/src-tauri/src/commands/llm.rs:228-274`
**影响**：能读取 `vault.db` 的进程可直接拿到 key。
**修复**：使用会话密钥对 API key 做 AES-256-GCM 加密后再存。

### R022 · Vault 文件/目录未按安全策略设置权限
**位置**：`tauri/src-tauri/src/services/vault_service.rs:203-224` 等
**影响**：默认 umask 下目录 755、文件 644，与文档要求的 0700/0600 不符。
**修复**：创建目录后 `set_mode(0o700)`，写敏感文件后 `set_mode(0o600)`。

### R023 · Clippy approx_constant 在测试目标失败
**位置**：`tauri/crates/solosoul-vault/src/storage.rs:2633`
**修复**：使用 `std::f64::consts::PI` 或允许该 lint。

### R024 · `create_account` 名称重复检查存在竞态条件
**位置**：`tauri/src-tauri/src/services/vault_service.rs:159-181`
**影响**：并发请求可能创建同名账户。
**修复**：使用单一写锁包裹“检查+插入”。

### R025 · `object_create` 允许客户端指定 ID 并可能覆盖已有对象
**位置**：`tauri/src-tauri/src/commands/object.rs:152-164`
**影响**：已知 ID 可覆盖现有对象。
**修复**：若提供 id，先校验数据库中不存在；未提供时由后端生成 UUID。

### R026 · 生物识别模块中密码哈希比较同样非恒定时间
**位置**：`tauri/src-tauri/src/commands/biometric.rs:201`
**修复**：复用 `auth::verify_password_core` 或统一的恒定时间比较工具。

### F009 · ESLint 已报告的死代码
**位置**：见问题清单。
**修复**：删除未使用 import；未使用参数改为 `_t`。

### F010-F032 · 前端规范/性能/架构问题
详见问题清单；其中 F010 组件拆分、F011/F012 元数据索引、F013 Highlight key、F014 HelpPage interval、F015/F016 ObjectEditorPage effect、F017 LoginPage 竞态、F018 SideNavigation render 阶段 ref、F019/F020 SideNavigation 状态同步、F021 ExportImportPage IIFE、F022 authStore.logout 等将在后续轮次逐步修复。
