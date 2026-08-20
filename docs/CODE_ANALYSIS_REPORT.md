# 代码分析修复报告

> 最后更新：2026-08-20 00:30:00
> 当前分支：`main`（领先 origin/main 2 个提交，工作区干净）
> 修复轮次：1（全新分析，旧报告不恢复、不继承，问题编号重新编排）

---

## 基线检查结果（阶段 0）

`npm run check-all` 全链路通过：

| 检查项 | 结果 |
|--------|------|
| TypeScript 类型检查（`tsc --noEmit`） | ✅ 通过 |
| Rust 格式化（`cargo fmt --check`） | ✅ 通过 |
| Clippy（`cargo clippy -- -D warnings`） | ✅ 通过 |
| Rust 单元测试（`cargo test`） | ✅ 通过 |
| ESLint（`npm run lint`） | ✅ 零警告 |
| 前端单元测试（Vitest） | ✅ 100 文件 / 838 用例全通过 |
| markdown chunk 边界检查 | ✅ 通过 |
| ACL 白名单一致性（196 个命令） | ✅ 通过 |
| 偏好 key 同步检查（20 个 key） | ✅ 通过 |

静态工具零告警，以下问题全部来自人工/启发式分析（4 个维度并行审计：Rust 后端质量、前端质量、安全漏洞、架构）。

**本次审计确认无问题的面**（防止后续轮次重复排查）：

- 无硬编码私钥/密码入库；无私钥泄露（仅有公钥常量与测试夹具）。
- `unsafe` 块全部为 macOS Keychain / Windows Hello FFI，均带 SAFETY 注释。
- 未使用 serde `untagged` enum；子进程调用全部 `.arg()` 参数数组，无 shell 拼接。
- 无 `dangerouslySetInnerHTML`；markdown 渲染走协议白名单；CSP 已收紧。
- 主密码 / session key / 派生密钥未落日志；安全场景随机数全部 OsRng。
- crates 依赖图单向分层，无循环依赖。
- 前端 511 个非测试模块无模块级死代码；大列表均已分页截断；无裸的循环内串行 IPC（除 P016 一处）。
- Rust 命令 handler 无 `Result` 被 `unwrap` 的高危路径；附件大文件读写均已分块流式。

---

## 问题清单（按优先级 P0 > P1 > P2）

**P0（严重）：无。**

| ID   | 优先级 | 类别 | 文件位置 | 描述 | 状态 |
|------|--------|------|----------|------|------|
| P001 | P1 | 漏洞 | `tauri/crates/solosoul-sync/src/attachments.rs:48-60` | 同步附件落盘路径仅净化 `file_name`，`object_id`/`attachment_id` 未校验，恶意已信任对端可路径遍历写出 vault 目录 | `[ ]` 待修复 |
| P002 | P1 | 漏洞 | `tauri/src-tauri/src/commands/attachment/mod.rs:450-475` 等 4 处 | 解密附件明文写入共享临时目录，未设 0600/0700，最长驻留 30 分钟，进程崩溃时永久残留 | `[x]` 已修复（4536a393） |
| P003 | P1 | 漏洞 | `tauri/crates/solosoul-core/src/vault_service/unlock.rs:1034-1066` | 改密重加密把附件明文写 `.rekey.tmp` 临时文件，崩溃残留后无任何清理路径 | `[x]` 已修复（527f0e44） |
| P004 | P1 | 漏洞（设计） | `tauri/crates/solosoul-core/src/pin.rs:341,602-613` | PIN（6-8 位纯数字）派生 KEK 加密 session key 落盘，可离线爆破（约 20 bit 熵），拉平主密码强度 | `[ ]` 待修复 |
| P005 | P1 | 架构/健壮性 | `tauri/crates/solosoul-vault/src/storage/objects.rs:326` | `object_row_to_record` 对 JSON 反序列化失败静默吞为 `Value::Null`，用户随后编辑保存将用空 properties 覆盖原数据 | `[x]` 已修复（18f0af93） |
| P006 | P1 | 架构/并发 | `tauri/src-tauri/src/services/profile_prefs.rs:20-44` | `update_profile_prefs` 读-改-写跨两次独立锁获取，全量 UPSERT 无版本校验，并发写者互相覆盖（lost update） | `[x]` 已修复（b913a5d5） |
| P007 | P1 | 架构/健壮性 | `tauri/src-tauri/src/commands/attachment/crud.rs:110-117` | `attachment_delete` 先删文件后改元数据，物理文件缺失时 `NotFound` 直接中止，元数据永远无法删除（与 batch 版容错行为不一致） | `[x]` 已修复（909539e3） |
| P008 | P1 | 性能 | `tauri/src-tauri/src/commands/llm/conversation.rs:156,171,199` | 软删/恢复/重命名会话均整表解密只为更新一条记录，单行读取 `load_conversation` 已存在未用 | `[x]` 已修复（8d3d9f52） |
| P009 | P1 | 死代码 | `tauri/crates/solosoul-crypto/src/aes.rs:95-245` | SOLO v3 分块加解密约 150 行仅被自身测试引用，生产路径全走 `cipher.rs` SOLC 格式，属平行重复实现且 v3 blob 已不可读 | `[ ]` 待修复 |
| P010 | P1 | 重复代码 | `tauri/crates/solosoul-core/src/template_service.rs:199` vs `objects.rs:1248` | `template_fingerprint` 字节级重复实现两处，模板 hash 漂移将产生静默不一致 | `[x]` 已修复（a60a7ada） |
| P011 | P1 | 重复代码/架构 | `tauri/crates/solosoul-core/src/llm/service.rs:362` vs `tauri/src-tauri/src/commands/llm/mod.rs:24` | LLM 内置 provider 默认值两处定义且已发散（id 体系、模型名均不同），GUI 与 CLI 默认配置不一致 | `[x]` 已修复（3355002b） |
| P012 | P1 | 重复代码/架构 | `tauri/crates/solosoul-core/src/export_import.rs:961` vs `tauri/src-tauri/src/commands/export_import/import.rs:1050`（导出侧同理） | 加密导入导出存在 core（CLI）与 GUI 两套平行实现，加密格式安全敏感面双维护易漏同步 | `[ ]` 待修复 |
| P013 | P2 | 漏洞 | `tauri/crates/solosoul-core/src/ocr/macos_vision.rs:429-433,424` | Vision CLI 失败路径把 OCR 识别文本带进错误消息与 info 级日志，敏感内容外溢到非加密存储面 | `[x]` 已修复（54124ff5） |
| P014 | P2 | 漏洞 | `tauri/crates/solosoul-core/src/path_util.rs:79-86` | `sanitize_file_name` 错误消息回显完整原始文件名，与 P019「错误不携带完整路径」既定约定不一致 | `[x]` 已修复（35d31d41） |
| P015 | P2 | 漏洞 | `tauri/src-tauri/src/setup/mod.rs:92-93` | `RUST_LOG` 环境变量可静默提升日志级别，debug 级下更多标识信息落盘；发布构建建议固定上限 | `[x]` 已修复（44168a28） |
| P016 | P2 | 性能 | `tauri/src/hooks/useAttachmentManagerBatchOps.ts:129,177,231` | 三个批量操作跨对象时循环内串行 `await invoke`，N 次顺序 IPC 往返（项目已有 `Promise.allSettled` 并行化先例） | `[x]` 已修复（7afabfdd） |
| P017 | P2 | 可优化（重复） | `tauri/src/hooks/useAttachmentManagerBatchOps.ts:115-260` | 批量软删/永久删/恢复三函数各约 48 行逐行相同，仅 IPC 命令名与 i18n key 不同 | `[x]` 已修复（c5be1ecd） |
| P018 | P2 | 可优化（重复） | `tauri/src/components/layout/SearchPopover.tsx:104-140` vs `tauri/src/pages/search/SearchPage.tsx:57-93` | `doSearch` 两处近乎复制，共享逻辑应下沉到 `lib/searchShared.tsx` | `[x]` 已修复（e68fffd0） |
| P019 | P2 | 性能 | `tauri/src/components/object/useObjectDetailModal.tsx:234-235` | `fieldOrder` 与 `flattenProperties` 每次渲染重算未 memo，与项目已确立的 memo 化范式不一致 | `[x]` 已修复（3a5aa904） |
| P020 | P2 | 漏洞（资源泄漏） | `tauri/src/lib/vaultDirectory.ts:71-92` | `pickVaultDirectory` 的 `visibilitychange` 监听器仅特定分支移除，桌面端正常返回后永久残留并逐次累积 | `[x]` 已修复（a37a7279） |
| P021 | P2 | 架构（超长函数） | `tauri/src/hooks/useRecoveryReceive.ts:28`（478 行）、`useLlmChatCore.ts:63`（450 行）、`pages/ai/useLlmConfigPage.ts:49`（437 行）、`components/layout/AddPageButton.tsx:25`（422 行）、`components/settings/PinSection.tsx:20`（415 行） | 前端 5 个 400+ 行超长函数/组件，多职责混杂，建议拆分 | `[ ]` 待修复 |
| P022 | P2 | 死代码 | `tauri/src/lib/updater.ts:171,259`、`tauri/src/lib/i18n.ts:24`、`tauri/src/lib/themeSchemes.ts:512` | 4 个导出无任何外部引用，仅文件内使用，`export` 多余（API 面污染，非真死代码） | `[ ]` 待修复 |
| P023 | P2 | 架构/健壮性 | `tauri/src-tauri/src/commands/llm/conversation.rs:23-27` | `load_conversations` 静默丢弃解析失败的会话行，无日志无上报，用户表现为「会话凭空消失」 | `[ ]` 待修复 |
| P024 | P2 | 性能 | `tauri/crates/solosoul-vault/src/storage/objects.rs:219-232` | `save_object_tx` 每保存一对象额外执行一次未缓存的模板名 SELECT，批量导入放大为 N 次查询 | `[ ]` 待修复 |
| P025 | P2 | 架构/性能 | `tauri/crates/solosoul-vault/src/storage.rs:484` | 全库单一 `Mutex<Connection>` 串行化，且逐行 AES 解密在持锁闭包内执行，长查询阻塞全部 DB 操作 | `[ ]` 待修复 |
| P026 | P2 | 性能 | `tauri/crates/solosoul-core/src/export_import.rs:203-205,277-284,334-338` | 导入导出 JSON 层未流式，峰值内存可达明文+密文+JSON 树三份，接近 100MB 上限的库在移动端有 OOM 风险 | `[ ]` 待修复 |
| P027 | P2 | 架构/并发 | `tauri/src-tauri/src/state/app_state.rs:551-563` | `init_saf_sync` 持 `RwLock` 读锁执行网络 I/O，`replace_vault_service` 写锁被阻塞，std RwLock 有写者饥饿风险 | `[ ]` 待修复 |
| P028 | P2 | 可优化（长函数） | `tauri/crates/solosoul-core/src/vault_service/unlock.rs:795` | `change_password` 126 行、嵌套 5 层，密码学关键路径的失败混态防护难以审查 | `[ ]` 待修复 |
| P029 | P2 | 可优化（长函数） | `tauri/crates/solosoul-core/src/vault_service/unlock.rs:990` | `reencrypt_attachments` 122 行、嵌套 6 层，建议按「单文件重加密」提取子函数（与 P003 修复可合并） | `[ ]` 待修复 |
| P030 | P2 | 可优化（长函数） | `tauri/crates/solosoul-sync/src/session.rs:263` | `handle_inbound` 125 行、嵌套 5 层、7 参数（带 `too_many_arguments` allow） | `[ ]` 待修复 |
| P031 | P2 | 可优化（嵌套/重复） | `tauri/crates/solosoul-sync/src/manager.rs:321` | `spawn_mdns_discovery` 79 行、嵌套 7 层；TXT 属性解析与 `commands/discovery.rs:136-190` 高度重复 | `[ ]` 待修复 |
| P032 | P2 | 可优化（重复） | `tauri/crates/solosoul-vault/src/storage/metadata.rs:200-228` | `list_audit_log` 内两段同构解密 match 块，可提取闭包消除 | `[ ]` 待修复 |
| P033 | P2 | 可优化（冗余） | `tauri/crates/solosoul-crypto/src/cipher.rs:47-56,78-87` | `Payload` 构造的 match 重复且多余，`aad.unwrap_or(&[])` 一行可等价表达 | `[ ]` 待修复 |
| P034 | P2 | 可优化 | `tauri/src-tauri/src/commands/object/mod.rs:1021-1028` | `collect_updated_fields` 对 JSON 值双重 clone，大文本字段每字段复制两次 | `[ ]` 待修复 |
| P035 | P2 | 架构 | `tauri/crates/solosoul-core/src/biometric/legacy.rs:19-20,70-104` | 桌面端生物识别降级为「文件 + account_id 派生密钥」，等价主密钥混淆落盘，建议 Keychain 失败时报错而非静默降级 | `[ ]` 待修复 |
| P036 | P2 | 文档 | `AGENTS.md`（项目根） | KDF 参数说明已过时：代码 `kdf.rs:47-55` 的 `from_env()` 在 release 构建已默认 production 参数，与文档「默认 8MiB/2」不符 | `[ ]` 待修复 |

## 修复进度

- 已完成：17 / 36（P001、P002、P003、P005、P006、P007、P008、P010、P011、P013、P014、P015、P016、P017、P018、P019、P020）

---

## 详细问题描述与修复指引

### P001（P1 漏洞）同步附件落盘路径遍历

`crates/solosoul-sync/src/attachments.rs:48-60` 的 `attachment_file_path` 直接 `base.join("attachments").join(object_id).join(attachment_id).join(safe_name)`，仅 `file_name` 过了 `sanitize_file_name`，而 `object_id`/`attachment_id` 来自对端 manifest（接收侧 :422-427）。被攻陷的已信任对端可发送 `../../` 形式的 ID，把附件写到 vault 目录之外。指纹/SAS 信任机制只防陌生攻击者，对端失陷场景无纵深防御。
**建议**：对 `object_id`/`attachment_id` 同样做白名单校验（如仅允许 `[a-zA-Z0-9_-]`），并在 join 后 canonicalize 校验仍位于 vault 附件目录内（参照 `commands/fs.rs:119-158` 的 reject_traversal 先例）。

### P002（P1 漏洞）解密附件明文落共享临时目录

`commands/attachment/mod.rs:450-475`（`decrypt_to_temp_dir` + `OPEN_TEMP_GRACE = 30min`）、`export_import/export.rs:492-511`、`export_import/import.rs:1225-1240`、`attachment_import_plugin.rs:559-564` 把密文附件解密为明文写入 `std::env::temp_dir()` 子目录；`create_dir_all` + `File::create`（`crates/solosoul-core/src/attachment_crypto.rs:61-77`）未设 0600/0700，Unix 上通常产出 0755 目录 / 0644 文件，多用户系统同机可读，违反项目「文件权限 0600」约定。清理靠 `thread::sleep` 延迟删除，进程被杀时明文永久残留。
**建议**：临时目录与文件显式 `set_permissions(0700/0600)`（Windows 参照 `icacls` 先例）；打开文件场景考虑改用内存映射或系统安全临时区；进程退出钩子里兜底清理。

### P003（P1 漏洞）改密重加密的明文临时文件残留

`crates/solosoul-core/src/vault_service/unlock.rs:1034-1066` 把附件解密明文写 `.rekey.tmp` 再加密为 `.rekey.new`。崩溃残留的 `.rekey.tmp` 在下次运行时只被过滤（:1011-1019 `files.retain`），全仓无任何清理路径；且 `remove_file` 非安全擦除。
**建议**：改为内存流式重加密（附件大小可控时）或在临时文件上落密文中转；启动/重加密前清理历史残留 `.rekey.tmp`（该修复与 P029 函数拆分可合并进行）。

### P004（P1 漏洞·设计）PIN 可离线爆破

`crates/solosoul-core/src/pin.rs:341` 以 Argon2id 从 PIN 派生 KEK 加密 session key 落盘，`:602-613` 仅允许 6-8 位纯数字（约 20 bit 熵）。攻击者拿到 vault 目录文件后可离线枚举（即使 64MiB/3 iter 也是小时级），直接解锁全部数据。`pin_failed_attempts` 锁只防在线尝试。
**建议**：至少在 UI 显著提示该风险；更彻底的做法是提高 PIN 最小熵（允许字母数字或更长长度），或对 PIN 凭据文件引入设备绑定（Keychain 包裹）。

### P005（P1 架构/健壮性）对象读取静默吞 JSON 损坏

`crates/solosoul-vault/src/storage/objects.rs:326`：`serde_json::from_str(&decrypted_props).unwrap_or(Value::Null)`（:325/:335 的 `children_ids`/`tags` 同理 `unwrap_or_default()`）。解密成功但 JSON 损坏时记录被静默显示为空对象；用户此时编辑保存即用空 properties 覆盖原数据，造成数据丢失。同文件列表路径 `map_object_list_row`（:101-121）对同样错误是传播报错的，两路径行为不一致。
**建议**：详情读取路径改为传播错误（与列表路径对齐），UI 显示「数据损坏」而非空对象。

### P006（P1 架构/并发）profile prefs 读-改-写丢失更新

`src-tauri/src/services/profile_prefs.rs:20-44` + `crates/solosoul-vault/src/storage/profile.rs:26-44,58-66`：`update_profile_prefs` 的读-改-写跨两次独立 `conn` 锁获取，且 `PROFILE_SAVE_SQL` 全量覆盖 `data` 整列、无版本校验。两个并发写者（如 LLM 统计落盘与设置页更新，写同一 profile 的不同 key）互相覆盖。Tauri command 在线程池并发执行，窗口真实可达。
**建议**：在 `save_profile` 层提供「单次锁内读-改-写」的原子 API，或引入 version 列做乐观锁冲突检测。

### P007（P1 架构/健壮性）附件删除在文件缺失时卡死

`src-tauri/src/commands/attachment/crud.rs:110-117`：`attachment_delete` 先删物理文件、后更新元数据，且 `remove_dir_all` 的 `NotFound` 直接 `return Err` 中止整个命令。物理文件已不存在时（用户手删、或同步端元数据先到附件未到），元数据永远无法通过该命令删除；batch 版（:367-369 `let _ =` 容错）行为不一致。先删文件后 `save_object` 的顺序在保存失败时还留下悬空元数据。
**建议**：`NotFound` 容错（与 batch 版对齐）；调换顺序或保存失败时记录可恢复的清理任务。

### P008（P1 性能）LLM 会话整表解密更新单行

`src-tauri/src/commands/llm/conversation.rs:156,171,199`：软删/恢复/重命名均通过 `load_conversations` 解密整张会话表只为更新一条；单行读取 `vault.load_conversation` 已存在（:124 已在用，注释自述 P004「避免整表加载只为取一条」）。会话多/消息大时每次重命名都是全表 AES 解密。
**建议**：三处改为 `load_conversation` 定位 + 单行 `save_conversation`。

### P009（P1 死代码）aes.rs 的 SOLO v3 分块实现无人使用

`crates/solosoul-crypto/src/aes.rs:95-245`：`encrypt_chunked_stream`/`decrypt_chunked_stream`/`validate_chunked_header` 及 `BLOB_VERSION_V3` 等常量仅被自身测试引用；生产路径 20+ 处调用全走 `cipher.rs` 的 SOLC 格式。且 `decrypt_blob`（:85）只接受 v2，v3 blob 实际不可读。约 150 行与 cipher.rs 平行的重复实现。
**建议**：删除（属删除文件级操作，按流程约束标记暂缓、最后由用户确认），或注明保留理由并加 `#[allow(dead_code)]`。

### P010（P1 重复代码）template_fingerprint 两处逐字节相同

`crates/solosoul-core/src/template_service.rs:199`（私有）与 `crates/solosoul-core/src/objects.rs:1248`（pub，经 `commands/object/mod.rs:10` 再导出）。模板 hash 是导入去重与模板同步判断依据，两处漂移将静默不一致。
**建议**：保留 `objects.rs` 的 pub 版本，`template_service.rs` 改为复用。

### P011（P1 重复代码/架构）LLM 内置 provider 默认值已发散

`crates/solosoul-core/src/llm/service.rs:362`（CLI 用）与 `src-tauri/src/commands/llm/mod.rs:24`（GUI 用）：id 体系不同（`openai` vs `builtin_openai`），值已漂移（Ollama `llama3.2` vs `llama3.1`、Alibaba `qwen-plus` vs `qwen-max`、DeepSeek embedding `None` vs `"text-embedding"`）。
**建议**：收敛到 core 单一来源，GUI 命令层直接复用。

### P012（P1 重复代码/架构）加密导入导出双实现

附件导入：core `import_attachments`（`export_import.rs:961`，137 行，CLI 用）vs GUI 版（`commands/export_import/import.rs:1050`，94 行）——相同 ZIP 布局、相同 HKDF 标签、相同遍历结构，仅进度回调与选择性导入不同。导出侧 core `export_vault`（:190）vs GUI `export_execute`（`export.rs:583`）同理。加密格式是安全敏感面，双实现易出现一边修了一边没修。
**建议**：以 core 为唯一实现，GUI 仅做进度回调与选择性导入的薄包装；合并时顺带按阶段拆分 core 版 137 行函数。

### P013（P2 漏洞）Vision CLI 错误外泄 OCR 文本

`crates/solosoul-core/src/ocr/macos_vision.rs:429-433` 异常退出时错误消息内嵌 `stdout`（识别出的用户图片文字）与 `stderr`，可进入 UI 状态/审计链；`:424` 把 CLI stderr 以 info 级写入 `app.log`。
**建议**：错误消息只带退出码与 stderr 摘要，不带 stdout 内容；日志脱敏。

### P014（P2 漏洞）sanitize_file_name 回显原始文件名

`crates/solosoul-core/src/path_util.rs:79-86`：`Err(format!("附件文件名无效: {}", file_name))` 把含路径的原始输入放进错误链，与项目「错误消息不携带完整文件路径」约定不一致（`attachment_import_plugin.rs:554-555` 正是按该约定写的）。
**建议**：错误消息只带净化后的描述或文件名长度，不回显原文。

### P015（P2 漏洞）RUST_LOG 可静默提升日志级别

`src-tauri/src/setup/mod.rs:92-93` 日志 filter 直接读环境变量，debug 级下 `biometric/mod.rs:392-397`（account_id）等更多标识信息落盘。本地威胁模型下风险低。
**建议**：发布构建固定 `info,ort=warn` 上限，仅 debug 构建响应 `RUST_LOG`。

### P016（P2 性能）附件批量操作串行 IPC

`src/hooks/useAttachmentManagerBatchOps.ts:129,177,231`：跨对象时 `for ... of byObject` 循环内串行 `await invoke`，N 次顺序 IPC 往返。`settingsStore.ts:420` 的 P030 迁移对同类模式已用 `Promise.allSettled` 并行化。
**建议**：改为 `Promise.allSettled` 并行（注意失败聚合与 toast 文案）。

### P017（P2 重复代码）批量操作三函数逐行相同

`src/hooks/useAttachmentManagerBatchOps.ts:115-260`：三个函数各约 48 行，仅 IPC 命令名与 i18n key 不同。
**建议**：参数化合并为一个函数。

### P018（P2 重复代码）doSearch 两处复制

`src/components/layout/SearchPopover.tsx:104-140` 与 `src/pages/search/SearchPage.tsx:57-93`：空查询守卫、缓存命中、`invoke('search_unified')`、写缓存、错误处理近乎复制，仅 `filter` 参数不同。
**建议**：共享逻辑下沉到已有的 `lib/searchShared.tsx`。

### P019（P2 性能）useObjectDetailModal 缺 memo

`src/components/object/useObjectDetailModal.tsx:234-235`：`fieldOrder`（`.map` 新数组）与 `flattenProperties(...)` 每次渲染重算，项目自身已在 `ObjectWorkspacePage.tsx:45`（V004 注释）、`HistoryViewer.tsx:126`、`WorkspaceObjectCard.tsx:111` 确立 memo 化范式。
**建议**：两值包 `useMemo`。

### P020（P2 资源泄漏）pickVaultDirectory 监听器残留

`src/lib/vaultDirectory.ts:71-92`：`visibilitychange` 监听器只在「页面曾 hidden 再 visible」分支移除（:77）；桌面端走系统对话框不触发 visibility 变化，正常返回后监听器永久残留，每次调用累积一个。
**建议**：在 promise settle（finally）中统一 `removeEventListener`。

### P021（P2 架构）前端超长函数

`useRecoveryReceive.ts:28`（478 行）、`useLlmChatCore.ts:63`（450 行）、`useLlmConfigPage.ts:49`（437 行）、`AddPageButton.tsx:25`（422 行）、`PinSection.tsx:20`（415 行），多职责混杂。
**建议**：按职责拆分子 hook / 子组件（无紧急性，可逐个轮次处理）。

### P022（P2 死代码）多余 export

`src/lib/updater.ts:171,259`、`src/lib/i18n.ts:24`、`src/lib/themeSchemes.ts:512` 的导出无任何外部引用（已全库交叉验证，含 `import type`）。
**建议**：去掉 `export` 关键字即可，零风险。

### P023（P2 健壮性）会话解析失败静默丢弃

`src-tauri/src/commands/llm/conversation.rs:23-27`：`if let Ok(c) = serde_json::from_slice(...)` 静默跳过损坏行，无日志。
**建议**：至少 `log::warn!` 记录会话 id 与错误，便于诊断。

### P024（P2 性能）save_object_tx 逐对象模板名查询

`crates/solosoul-vault/src/storage/objects.rs:219-232`：每保存一对象执行一次 `SELECT name FROM user_templates`（未走 `prepare_cached`），批量导入放大为 N 次。
**建议**：批入口一次性加载模板名 map 传入。

### P025（P2 架构/性能）全局 DB Mutex 持锁解密

`crates/solosoul-vault/src/storage.rs:484`：单一 `Mutex<Option<Connection>>` 串行化所有 DB 访问，逐行 AES 解密在持锁闭包内执行（如 `objects.rs:937-950`）。一次高级搜索/同步批量应用期间，GUI 其他 DB 操作全部阻塞。
**建议**：评估「SQL 取数 → 释放锁 → 锁外解密」两阶段改造（属设计权衡，改动面大，可作为专项评估而非直接修复）。

### P026（P2 性能）导入导出 JSON 层未流式

`crates/solosoul-core/src/export_import.rs:203-205,277-284`（导出）与 :334-338（导入）：加解密已流式，但 JSON 层持有全量明文 records + JSON 树 + 字节多份拷贝，导入端 `payload.enc` 整块读入（上限 100MB）后整体解密。峰值内存约三份明文，移动端 OOM 风险。
**建议**：导入端改为流式解密 + 流式 JSON 解析（如 `serde_json::Deserializer::from_reader` 分段），或降低单包上限并文档化。

### P027（P2 并发）init_saf_sync 持读锁做网络 I/O

`src-tauri/src/state/app_state.rs:551-563`：`spawn_blocking` 闭包内全程持 `vault_service` 的 std `RwLock` 读锁执行 `sync_from_remote`（含网络 I/O）；期间 `replace_vault_service`（:380-388）写锁被阻塞，std RwLock 有写者饥饿风险。
**建议**：缩小持锁范围（取必要句柄后释放再同步），或换 `tokio::RwLock`。

### P028–P034（P2 可优化）Rust 长函数与局部冗余

- P028 `vault_service/unlock.rs:795` `change_password`：126 行、嵌套 5 层，建议拆出 config 迁移与审计子函数。
- P029 `vault_service/unlock.rs:990` `reencrypt_attachments`：122 行、嵌套 6 层，建议提取「单文件重加密」子函数（与 P003 合并修复）。
- P030 `solosoul-sync/src/session.rs:263` `handle_inbound`：125 行、嵌套 5 层、7 参数，参照已提取的 `validate_handshake_peer` 继续拆。
- P031 `solosoul-sync/src/manager.rs:321` `spawn_mdns_discovery`：嵌套 7 层；TXT 属性解析与 `commands/discovery.rs:136-190` 高度重复，可提取共用解析结构体。
- P032 `solosoul-vault/src/storage/metadata.rs:200-228`：两段同构解密 match，提取 `decrypt_field` 闭包。
- P033 `solosoul-crypto/src/cipher.rs:47-56,78-87`：`Payload` 构造 match 用 `aad.unwrap_or(&[])` 收敛。
- P034 `commands/object/mod.rs:1021-1028`：`collect_updated_fields` 双重 clone，按分支移动所有权。

### P035（P2 架构）生物识别静默降级

`crates/solosoul-core/src/biometric/legacy.rs:19-20,70-104`：Keychain 不可用时 fallback 以 `account_id`（公开值）派生密钥保护主密钥，实际只依赖 OS 文件权限（注释已如实承认）。拿到用户目录的进程可还原主密钥。
**建议**：Keychain 失败时向用户显式报错/确认，而非静默降级。

### P036（P2 文档）AGENTS.md 的 KDF 参数说明过时

文档称「开发模式默认 8MiB/2 iter，生产需 `SOLOSOUL_SECURE=1`」，但 `crates/solosoul-crypto/src/kdf.rs:47-55` 的 `from_env()` 在 release 构建已默认 production 参数。
**建议**：更新 AGENTS.md 相应小节，与代码实际行为对齐。

---

## 备注

- 本报告为全新生成（2026-08-20），未恢复/继承旧报告内容；问题编号与旧报告无对应关系。
- 按用户要求，本轮仅执行阶段 0–1（基线检查 + 全库分析 + 报告生成），**未执行任何修复**；后续修复从 P001 开始按阶段 3 流程逐项进行。
- 涉及删除文件级操作的项（P009）按流程约束应先标记暂缓，最后汇总给用户确认。
