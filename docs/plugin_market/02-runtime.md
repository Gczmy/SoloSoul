# 2. Rust Host 运行时

> 实现位置：`tauri/crates/solosoul-plugin/`（crate 名 `solosoul_plugin`）。
> 本文以实际模块为准，取代原「04-rust-host / 05-field-mapping / 08-data-flow / 10-security / 13-lifecycle / 15-appendix」设计稿。

## 2.1 模块地图

| 模块 | 职责 |
|------|------|
| `manager.rs` | `PluginManager`：安装（SHA-256 校验 + bundled 兜底）、更新、卸载、运行编排、Consent/对话框响应、会话列表、审计查询、注册表刷新 |
| `registry.rs` | `PluginRegistry`：bundled 注册表加载、远程拉取（minisign 验签）、市场插件列表 |
| `store.rs` | `PluginStore`：`{data_dir}/plugins/` 目录读写（manifest / wasm 保存、加载、删除、目录扫描） |
| `sandbox.rs` | `WasmSandbox`：Wasmtime 编译（SHA-256 进程级缓存）、燃料上限、执行与 Trap 处理 |
| `host.rs` | `SoloHostFunctions`：全部 Host Functions 实现、异步 HTTP 代理、错误码 |
| `field.rs` | `FieldResolver`：字段路径解析（typed field / contracts / Vault 查询）、结构树、对象列表 |
| `consent.rs` | `ConsentManager`：字段授权请求生命周期管理 |
| `session.rs` | `PluginSessionManager`：运行会话创建、TTL 过期清理 |
| `rate_limiter.rs` | `RateLimiter`：按插件 × 操作键的访问频率限制 |
| `audit.rs` | `PluginAuditLogger`：审计日志写入与查询 |
| `version.rs` | 版本解析与兼容性检查（`min/max_app_version`） |
| `paths.rs` | 市场目录定位（打包资源 / 开发路径回退） |
| `event.rs` | `PluginEvent`：运行期间事件（log / result / consent_request / dialog_request / completed / error / custom_event） |
| `manifest.rs` | `PluginManifest` / `RegistryEntry` / `RegistryVersion` / `PluginNetworkPolicy` / `PluginParam` / `PluginTier`（p0–p4）/ 契约模型 |
| `error.rs` | `PluginError` 统一错误类型 |

## 2.2 生命周期

### 安装与更新（RF-214）

`PluginManager` 将安装分为准备与同步发布，公开接口如下：

| API | 契约 |
|-----|------|
| `prepare_install_from_registry_with_progress(id, version, callback)` | 准备注册表指定版本，返回 `PreparedPluginInstall`，不改变当前安装 |
| `prepare_update_with_progress(id, callback)` | 从共享注册表选择最新版本后准备；CLI 安装与更新都使用此入口 |
| `PreparedPluginInstall::manifest()` | 只读访问实际准备版本的完整清单，包含可能采用的 bundled 版本；准备对象字段私有且不可 Clone |
| `publish_install(prepared)` | 消费准备对象，返回实际 `PluginInstallResult`；不下载或重新哈希大文件 |

既有 `install_from_registry(_with_progress)` / `update(_with_progress)` 签名保留，以准备 → 发布包装；带进度接口只在发布成功后发送 Completed。CLI 则在原 `VaultSession` 的 `TaskContext::commit` 内调用 `publish_install`，终态来自真实任务 join。

准备阶段核对版本兼容性、manifest 的插件 ID/版本及 WASM 的注册表 SHA-256，并完成暂存。manifest 最多 1 MiB，WASM 必须非空且最多 10 MiB；远程读取既检查声明长度，也限制实际累积字节，每段最多处理 64 KiB 并让出执行权，bundled 文件同样有界。远程 manifest 身份不符、WASM 摘要不符或超限直接失败；bundled 回退版本须在当前注册表中存在并通过相同兼容性和摘要校验。不改变 registry 的 minisign 策略。

准备期间只报告真实阶段和字节进度，不写成功审计、不发送 Completed。原安装版本与注册表 SHA 均相同且完整时可免下载复用；准备时成对读取并校验，发布时复核版本选择和文件元数据，若已被移除或替换则失败，不能误报仍已安装。复用成功沿用原行为，不重复写 PluginInstalled 审计。

本地发布布局：

```text
{data_dir}/plugins/{plugin_id}/
  current.json
  versions/v-<本次随机标识>/manifest.json
  versions/v-<本次随机标识>/plugin.wasm
  manifest.json     # 存量旧布局，仅在没有 current.json 时读取
  plugin.wasm
```

`current.json` 记录布局版本、版本目录标识、manifest/WASM 摘要及 WASM 字节数。Unix 新建版本暂存目录从创建时显式使用 `0700`，正文文件使用 `0600`（RF-1066），不依赖 tempfile 默认目录权限或当前 umask；准备前后的取消清理和发布行为一致。暂存对象唯一拥有本次目录和临时指针；准备中完成写入、flush/sync 及句柄关闭，取消导致 Future/准备对象析构时清理本次文件。同步发布原子替换 `current.json` 后即保留完整新目录，不先删除旧指针或逐个覆盖原文件。同一进程内的配对读取、发布与卸载共用一把锁，网络等待和暂存正文写入不持锁。Windows 目标占用等发布失败保留旧选择。未引用目录不登记为已安装；本项不自动清扫旧版本或进程异常退出留下的目录。

没有指针时兼容旧双文件布局；有指针但损坏、路径无效或目标不完整时拒绝读取，不回退旧版。列表仅有界读取清单并检查指针、普通文件、WASM 非空及尺寸，不能据列表出现就断言 WASM 内容摘要有效。运行入口用 `load_plugin` 在同一次版本选择下取得 manifest/WASM：新布局强制验证指针中的摘要，旧布局保留有清单摘要才验证的兼容规则。运行时不混用后台更新前后两版的代码与权限。

发布后安装审计保持 best effort，其写入失败不改变发布结果。新版本目录和旧布局文件均可继续留存，因此旧客户端既可能无法识别新安装，也可能只读到遗留旧版；降级不构成可靠回退。本项不保证断电后的目录项耐久，也不提供跨进程或同用户外部改写的并发协调。

GUI 的取消资源、安装串行锁及后处理顺序保持不变：Host 把 Manager 的 Completed 映射为 98% Finalizing，触发两路同步通知；安装命令还尝试种子模板绑定迁移，然后才发送最终 Completed。迁移失败仅告警，更新命令不新增该迁移。CLI 的取消与提交争取同一原子许可：取消先发生不发布，提交先发生则等待真实结果；事件按 TaskId、账户与会话代次接纳，旧结果不能回填新会话或清掉新任务。插件运行本身的 worker 生命周期尚未由 RF-214 迁移。

### 卸载

`PluginManager::uninstall`：删除插件目录（`store.rs::delete_plugin`）。Vault 数据与审计日志不受影响。

### 运行（`PluginManager::run`）

```
用户触发 run（携带 params + 事件 Channel）
  → 创建 Session（TTL 取自 manifest.data_ttl_seconds，session.rs）
  → 审计 PluginRunStarted
  → 构造 FieldResolver（vault + contracts）/ SoloHostFunctions / ConsentManager
  → WasmSandbox 编译并执行 run()（燃料上限 10_000_000_000）
  → 运行期间经 Channel 推送事件（log / consent_request / dialog_request / result / …）
  → 完成或 Trap：发送 completed / error 事件，审计 PluginRunCompleted
```

## 2.3 权限与数据流

```
插件调用 Host Function（如 get_field / post_json）
  → RateLimiter 检查（同字段超限返回 -8）
  → manifest 声明范围检查（required/optional_fields，越界返回 -1）
  → Session TTL 检查（过期返回 -3）
  → 敏感字段触发 consent_request 事件 → 前端弹窗 → 用户响应（plugin_consent_response）
  → 经 FieldResolver 从 Vault 解密取值（public/internal 级字段自动放行，sensitive/critical 级需用户确认）
  → 返回字段值并记录审计（字段访问 / 拒绝 / 网络拦截 / 速率触发）
```

- **网络**：`post_json` 经宿主代理（异步 HTTP 句柄），受 `network_policy`（`block_all_outbound` + `allowed_domains`）域名白名单约束，未授权返回 -10。
- **数据 TTL**：授权数据仅在 Session 存活期间可访问；Wasm 内存随 Store drop 销毁（Store 级隔离）。
- **stdio 黑洞**：插件 stdout/stderr 静默丢弃，不继承宿主终端，杜绝日志注入。
- **单次运行限制**：`MAX_PLUGIN_SLEEP_MS = 1000`、`MAX_PLUGIN_READ_LEN = 64 KiB`。

## 2.4 字段解析

`FieldResolver`（`field.rs`）取代了旧设计中的硬编码字段映射表：

- **typed field**：`parse_typed_field` 将 `对象类型.字段名`（camelCase，如 `idCard.number`、`passport.expiryDate`）解析为对象 + 字段。
- **契约**：`PluginContractBinding`（`typeId` / `typeIdAliases` / `roles`）把插件声明的语义角色（如 `address.street`）绑定到 Vault 实际对象。
- **解析优先级**：typed field → contracts 绑定 → Vault 查询（`resolve` / `resolve_typed` / `field_metadata`）。
- 辅助能力：`build_structure_tree`（数据契约结构树）、`list_objects`（按契约类型列出对象）、`list_attachments` / `prepare_attachment_copy`（附件）。

## 2.5 安全机制清单

| 维度 | 机制 | 实现 |
|------|------|------|
| 注册表完整性 | minisign 签名验证（`SOLOSOUL_REGISTRY_PUBKEY`），失败回落 bundled | `registry.rs` |
| 插件完整性 | 安装时 SHA-256 强制校验 | `manager.rs` |
| 执行隔离 | Wasmtime 沙盒 + WASI Preview1；stdio 黑洞 | `sandbox.rs` |
| 防死循环 | 单次运行燃料上限 10,000,000,000 | `sandbox.rs` |
| 编译性能 | 编译产物按 wasm SHA-256 进程级缓存；移动端 Pulley 解释器 | `sandbox.rs` |
| 字段级授权 | manifest 声明范围 + 逐字段 Consent；TTL 自动失效 | `host.rs`、`consent.rs`、`session.rs` |
| 会话控制 | Session TTL + 过期清理 + 撤销 | `session.rs` |
| 内存安全 | Store 级隔离（TTL 到期整个 Store drop）；Host 侧 `Zeroizing` | `sandbox.rs`、`host.rs` |
| 网络隔离 | 域名白名单代理（`block_all_outbound` 默认阻断） | `host.rs` |
| 速率限制 | 按插件 × 操作键限流（如字段访问） | `rate_limiter.rs` |
| 审计追溯 | 结构化审计日志（运行/字段访问/网络拦截/失败） | `audit.rs` |
| 存储隔离 | 插件独立目录 `{data_dir}/plugins/`，与 Vault 隔离 | `store.rs` |
| 版本兼容 | `min/max_app_version` 范围检查 | `version.rs` |

> 说明：当前实现**无** Debug/Release 安全参数差异（燃料、TTL、速率限制在两种构建下相同）；仅 `paths.rs` 在开发模式回退源码路径。

## 2.6 Host Functions ABI 与错误码

完整 ABI 以 [`SDK/rust/src/lib.rs`](../../SoloSoul_plugin_market/SDK/rust/src/lib.rs)（插件侧声明）与 `host.rs`（宿主侧实现）为准，两者一一对应。核心分组：

| 分组 | 函数 |
|------|------|
| 数据访问 | `list_objects`、`get_field`、`get_data_structure_tree`、`list_attachments`、`prepare_attachment_copy` |
| 网络 | `post_json`（宿主白名单代理） |
| 交互 | `show_dialog` |
| 参数/本地化 | `get_param`、`get_locale` |
| 结果 | `send_result_json`、`result_text`、`result_key_value`、`result_table`、`result_markdown`、`write_output_file`、`copy_output_file` |
| 工具 | `log`/`log_info`/`log_warn`/`log_error`/`log_debug`、`get_timestamp`、`sleep`、`escape_json`、`truncate`、`parse_date_yyyymmdd_or_iso`、`days_until_ymd` |
| 媒体 | `image_watermark`、`pdf_watermark` |

**错误码**（`host.rs::code`，与 SDK `PluginError` 一致）：

| 码 | 含义 | 码 | 含义 |
|----|------|----|------|
| 0 | 成功 | -8 | 频率超限 |
| 1 | HTTP 请求进行中（非错误，轮询用） | -9 | 未实现 |
| -1 | 权限不足（越出 manifest 声明范围） | -10 | 域名未授权 |
| -2 | 用户拒绝 | -11 | 非法参数 |
| -3 | TTL 过期 / 会话撤销 | -12 | Wasm Trap |
| -4 | 缓冲区不足 | -13 | 文件不存在 |
| -5 | 字段路径非法 | -14 | 文件过大 |
| -6 | 网络超时 | -15 | 处理失败 |
| -7 | Vault 已锁定 | | |

## 2.7 版本兼容性检查

`version.rs`：`min_app_version ≤ 客户端版本 ≤ max_app_version` 通过（SemVer 比较），否则拒绝安装并提示版本不兼容。`plugin_api_version` 用于 ABI 兼容提示（当前官方插件 1.0–2.0）。

## 2.8 未实施的设计提案

以下内容仅存在于早期设计稿，**未进入实现**（如未来需要，以本文档更新为准）：

- **JIT 即时解密**（排队期间不持明文、提交前才解密）——未实现。
- **熔断机制**（按插件区分永久/临时失败并自动熔断）——未实现。
- **Debug/Release 差异化安全参数**——未实现（见 §2.5 说明）。
