# SoloSoul 代码审计修复报告（合并版）

> 最后更新：2026-08-03
> 当前分支：`main`
> **合并说明**：本文件为 `CODE_ANALYSIS_REPORT.md`（2026-08-02 初始分析，修复人）与 `CODE_FIX_VERIFICATION.md`（2026-08-02~03 两轮独立复核，验证人）的**合并版**。验证报告已删除（git 历史可追溯），其验证结论、N/R 项跟进记录与提交指针均已并入本文件。
> **清理规则**：已通过独立复核的项仅保留**一行验证要点**归档（详见 §3），删除原始长篇修复细节（完整细节见 git 各修复 commit）；**未完成/有意保留项**在 §4 展开详细讨论。
> 分析范围：`tauri/src/`、`tauri/src-tauri/src/`、`tauri/crates/`（约 6.4 万行 Rust + 323 个前端文件）；复核另覆盖 `solosoul_cli/` 依赖面。

---

## §1 审计与验证基线

| 检查 | 初始基线（2026-08-02 阶段 0） | 复核后基线（2026-08-03 当前 HEAD） |
|------|-------------------------------|-------------------------------------|
| `npx tsc --noEmit` | ✅ 0 错误 | ✅ 0 错误 |
| `npm run lint` | ⚠️ 0 错误，1 warning（`useOnboarding.ts:131`） | ✅ 0 错误 0 警告（warning 随 P220 消除） |
| `cargo fmt --check` | ✅ 通过 | ✅ 通过 |
| `cargo clippy --workspace --all-targets` | ✅ 零警告 | ✅ 零警告 |
| 前端测试 | ✅ 46 文件 / 430 用例 | ✅ 55 文件 / 484 用例（较修复前 +54） |
| Rust 测试 | ✅ 全部通过 | ✅ 全部通过（solo_soul 364 / core 156（默认；`future-keychain` 开启时 +4）/ crypto 34 / plugin 56 / sync 47 / vault 123） |

## §2 总览：80 项问题处置结果

- **审计问题清单共 80 项**（P001-P007、P101-P142、P201-P231）。
- **80 项问题全部闭环**（78 项可执行修复 + P133 用户决策接入 + P134 用户决策门控 + P135 用户决策反向接入 + **N-10/P207 路径 1 公钥注入闭环**），其中 P104/P206 为部分修复/部分保留，P209 为用户决策保留。
- **遗留未完成 1 类**（§4 详细讨论）：
  1. **P223/P224**：长函数/巨型组件长期重构（有意留待迭代）。
- 另有两项「有意保留但需后续跟进」记录在案：P209（legacy XOR 迁移窗口）、P206 遗留观察（PDF embed 与 CSP）。

## §3 已通过复核项归档（清理版）

### 3.1 原始问题 P001-P231 判定归档

> 判定列：✅ 修复正确（复核确认）· 🔺 修复引入问题（已闭环）· ⏸ 决策暂缓/保留（见 §4）· ✅* 部分修复（残余已闭环）。

**P0 安全（A 组）**

| ID | 判定 | 验证要点 |
|----|------|----------|
| P001 | ✅ | 指纹取自 Noise 握手认证值，双角色握手后强制比对，`record_peer` 落库握手认证指纹；mDNS 明文广播 node_id 为有意保留 |
| P002 | ✅ | 真 DPAPI（CryptProtectData）+ 魔数检测原子迁移旧凭证；残余为 DPAPI 固有限制（同用户上下文进程可自行解密，非 TPM 绑定） |
| P003 | ✅* | debug/release 判定正确、旧账户按落库参数验证再升级不锁死；N-2 修 `reencrypt_all` 无条件 commit 与两阶段非原子，R-4 补回滚失败上抛文案 + 失败注入测试 |
| P004 | ✅ | `trashStore.clearOnVaultLock()` 接入 vault-locked 清理链，真实清空无竞态 |
| P005 | ✅ | `searchCache.clear()` 挂接；N-3 补 `llmStore.reset()`（清明文 + 退订 llm-stream-chunk） |
| P006 | ✅ | 删除失败 toast + logger + 弹窗保持打开，i18n 双语齐全 |
| P007 | ✅ | 三处保存失败统一 toast + 日志，i18n 双语 + defaultValue 兜底 |

**安全中危（C 组）**

| ID | 判定 | 验证要点 |
|----|------|----------|
| P101 | ✅ | 188 条显式 allowlist 替代 `allow-all-custom-commands`，移除 crypto oracle，与前端全部 invoke 比对零缺失 |
| P102 | ✅* | URL scheme/host 校验 + 数据路径强制已登记；N-4 对未登记新 URL 强制**原生确认对话框**（XSS 不可程序化点击，堵死两步绕过）+ embedding 通道发送前登记校验 |
| P103 | ✅ | 信任检查前仅回最小错误帧、peer 落库延迟到配对确认后、指纹绑定全链路 |
| P104 | ✅* | URL 收窄 + 重定向白名单 + 流式双限 + 原子 rename；N-5 内置 sha256 清单扩至三档 12 文件（官方 HF 源交叉验证） |
| P105 | ✅ | SOLC v2 头部纳入 AAD，v1 自动探测回退，旧包可解密；已知权衡：v2 包仅新版可读 |
| P106 | ✅ | 头部一致性校验 + 容量封顶 + 流式增量读取；SOLO v3 blob 无生产调用方（理论面加固） |
| P107 | ✅ | 基目录收窄 Desktop/Documents/Downloads + vault 附件目录；两处轻微降级（目录外选附件 sizeBytes=0、遗留外部 vaultPath 预览报错）已声明 |
| P108 | ✅ | copy/stat 仅 `$APPCACHE/$TEMP`，全库唯一调用点 `mobileFileTransfer.ts` 无破坏 |
| P201 | ✅ | zip manifest 三重防线（读取前 size 检查 + take 限制 + 读后复核） |
| P202 | ✅ | 导出密钥改 `KdfConfig::from_env()`（release 生产档），manifest 携带 kdf 字段，旧包 balanced 回退；已知前向不兼容已声明 |
| P203 | ✅ | `attachment_open` 调试日志删除/脱敏，剩余 error! 仅含 io 错误/静态文本 |
| P204 | ✅ | session key/派生主密钥 Zeroizing 包裹，语义不变 |
| P205 | ✅* | `commands/crypto.rs` 整文件删除，命令面彻底清除；N-9 清理 ipc.test.ts 陈旧 mock 测试 |
| P206 | ✅* | frame-src `data:` 移除（零 iframe）；style-src `'unsafe-inline'` 结构性保留（227+ 处内联 style，P048 级重构才可移除），残留风险被无 innerHTML + markdown 净化压制。**遗留观察**（PDF embed 与 object-src，见 §4.6） |
| P207 | ✅（N-10 已闭环） | minisign 校验逻辑 + **独立专用公钥已注入**（`EMBED_REGISTRY_PUBKEY_B64`，与 updater 密钥隔离）+ 真实签名端到端防漂移测试；签名防护正式激活，详见 §4.1 |
| P208 | ✅ | WASI stdio 改默认空槽黑洞，插件输出收敛到 Consent 约束的 host 通道 |
| P209 | ✅（决策保留） | LEGACY_XOR_KEY 仅用于 <2.0 旧凭证一键迁移，零写入面；用户决策保留，关闭条件见 §4.5 |
| P229 | ✅ | `isSafeExternalUrl` 显式白名单（http/https/mailto/相对路径），javascript:/data: 等变体全覆盖，4 组 7 断言 |
| P230 | ✅ | `ocrScanStore.clearOnVaultLock` 清空结果数据保留 UI 偏好，清理链风格与 P004/P005 一致，无复活窗口 |

**Rust 性能（D 组）**

| ID | 判定 | 验证要点 |
|----|------|----------|
| P109 | ✅ | LEFT JOIN sync_hlc 批量取 HLC + 水印过滤下推 SQL，语义逐字节等价；隐含假设：updated_at 恒为 UTC `+00:00` |
| P110 | 🔺→✅ | 初版 LIMIT/OFFSET 引入同步永久停滞（N-1，复核发现）→ keyset 分页 + 回退行 SQL 精确过滤 + 会话层节点编码对齐闭环；R-2/R-1/R-3 跟进（trash 表同构缺陷、秒/毫秒错配、游标持久化）全部闭环 |
| P111 | ✅ | `list_object_metadata` 不 SELECT/解密负载列，7 处调用方切换无误用 |
| P112 | ✅ | 解密收敛单轮（parent_id 预分组 + 复用 summary.properties），输出与旧实现等价 |
| P113 | ✅* | `ocr_scan_image` 移入 spawn_blocking；N-6 补 `ocr_scan_mrz` 同模式 |
| P114 | ✅ | object/trash/search/attachment 重路径命令统一 spawn_blocking，VaultStore 全 Mutex 线程安全 |
| P115 | ✅ | `apply_sync_records_batch` 单事务 + `BorrowedSyncRecord` 零克隆 + HLC 只查写一次，单条失败不中断整批 |
| P210 | ✅ | 递归值树匹配替代 Value→String 往返，为旧预筛超集不漏放 |
| P211 | ✅ | page_delete 批量加载 + 单事务批量写入，删除语义等价、失败语义从「部分成功」变「整体回滚」属改进 |
| P212 | ✅ | 借用迭代 + metadata 预查 + 单事务；SkipExisting/Overwrite 语义等价，两处已声明边界 |
| P213 | ✅ | SQL 常量化 + 热点语句 prepare_cached + with_tx 手动事务，SQL 常量逐字等价 |
| P214 | ✅ | public 级先 SQL 预筛再解密，输出与旧全量筛选等价，有防泄漏测试 |

**前端性能（E 组）**

| ID | 判定 | 验证要点 |
|----|------|----------|
| P116 | ✅ | `ChatMessageItem` memo，仅末条流式消息重渲染；小瑕疵：rehypePlugins 内联数组致复制点击全量重解析（严格优于修复前） |
| P117 | ✅ | 字段级选择器 + 稳定 action 引用，生产代码整店订阅清零 |
| P118 | ✅ | 7 个 useCallback deps 正确，卡片行为逐项等价 |
| P119 | ✅ | 过滤→分页顺序正确，批量操作仍作用于全部 filtered；加载更多后游标回缩为声明设计 |
| P215 | ✅ | 整店订阅改字段级选择器，插件日志环形截断（上限 200/50 为产品可见变更已注释） |
| P216 | ✅ | 折叠瞬间持久化滚动位置，弃用每帧 onScroll 写入，折叠/卸载双路径持久化 |
| P217 | ✅ | 重命名输入本地化 + 三级组件 memo（自定义比较器只比数据 props），8 个防回归测试 |
| P218 | ✅ | `filteredLogs` useMemo + `OperationLogCard` memo + 加载更多分页 |

**错误处理/架构（F 组）**

| ID | 判定 | 验证要点 |
|----|------|----------|
| P120 | ✅* | toast + N-7 locale key 补入双语 + N-11 失败态/重试 UI（三态可区分，不再与空导出范围同态） |
| P121 | ✅ | 批量操作失败计数 toast + logger，i18n 双语齐全 |
| P122 | ✅* | toast + N-7 locale + N-11 失败占位 Card + 重试（保留原 trashId），与「无数据」严格区分 |
| P123 | ✅ | verify 失败（返回 false）与 invoke 异常（toast）正确区分，key 双语存在 |
| P124 | ✅ | 异常细节保留并上抛，仅密码错误返回 false；靠错误消息正则匹配，轻微脆弱已备注 |
| P125 | ✅* | 成功/失败 toast 齐全 + N-7 locale key 补入 |
| P126 | ✅ | 仅 not-found 返回 null，真实异常抛出，有防回归测试 |
| P127 | ✅ | `loadCustomPages` await + catch 补齐 |
| P128 | ✅ | 第 5 写入点消除，主题缓存统一交回 settingsStore helper |
| P129 | ✅* | ②③ 副本收敛唯一 helper；N-8 `syncPlaintextPref` 导出，App/index.tsx 与 notification.ts 两处直写③收敛，「唯一写入点」代码级强制 |
| P130 | ✅ | `confirmWithPause` try/finally，取消/异常路径均正确 resume，无残留裸调 |
| P131 | ✅ | `ipcClient.ts` 统一层，61 文件迁移零漏迁；统一日志仅 dev 生效、requireUnlocked 守卫预留无调用方 |
| P227 | ✅ | 10 处静默 catch 补 logger.warn + 2 处完全无 catch 的未捕获 rejection 补 toast，降级行为不变 |
| P228 | ✅ | 两处循环依赖断链（notification 依赖注入 + 共享类型抽离到 `types/templateSync.ts`），3 个调用点无漏传 accountId |
| P231 | ✅* | window.open 兜底改应用内 toast；R-5 补 `settings:link_open_failed` 双语 locale key |

**死代码/去重（G 组）**

| ID | 判定 | 验证要点 |
|----|------|----------|
| P132 | ✅ | 8 个死命令删除（含 crypto oracle），注册与 allowlist 同步清理，前端/CLI 零残留 |
| P133 | ✅ | 按用户决策**接入为 macOS 默认 OCR 引擎**：新增 `OcrModelTier::Vision` 档位（仅 macOS `ocr_list_available_tiers` 返回、置于首位），`OcrPreferences` 默认档 macOS 为 Vision；扫描走 `macos_vision::scan_image`（spawn_blocking + `#[cfg(target_os="macos")]` 属性剪裁，非 macOS 编译不引用门控模块）；MRZ 回退 small 档；前端仅 macOS 显示该档、默认选中、隐藏安装/下载/删除（builtin 标记）、按图片-only 过滤；详见 §4.2.1 |
| P134 | ✅ | 按用户决策升级为 `feature = "future-keychain"` 门控（默认关闭，`#[cfg(all(target_os="macos", feature="future-keychain"))]`），移除 `#[allow(dead_code)]` 脱离默认编译面；启用时 10 个 dead_code 警告为「尚未接入 platform_storage()」的预期提醒；详见 §4.2.2 |
| P135 | ✅ | 按用户决策**反向接入**（不删）：`VaultFileSystem::write_file_atomic`（trait 默认实现 + SAF 覆盖带 dirty）；7 处 config 写入 + `save_accounts` 全部切原子写，`write_config_atomic` 同步收紧 .bak 权限；`read_config_with_recovery` 接 unlock/verify 读取路径（孤儿 .tmp 提升 + .bak 回退）；R-4① 的「config 写一半」风险降为近乎不可达；详见 §4.2.3 |
| P136 | ✅ | 自我纠错正确：CLI 依赖方法全部恢复，最终仅删 15 个零调用方法（provider 管理 8 + 会话管理 5 + reset_stats + 非流式 send_message） |
| P137 | ✅ | LLM 8 结构体统一复用 `solosoul_core::llm::config` 唯一真理来源，serde 字段逐字段一致 |
| P138 | ✅ | 11 对 cfg 命令合并，真有差异的 2 对保留；附带发现 `sync_discover` 系历史遗留未注册死命令（非本次引入） |
| P139 | ✅ | `shared.rs` 收敛三处重复，与三处原函数逐字节一致 |
| P140 | ✅ | `useOcrModelManager` 收敛，两页行为逐分支等价，10 个防回归测试 |
| P141 | ✅ | `searchShared.tsx` 收敛，防抖/缓存/过滤/渲染逐分支一致 |
| P142 | ✅ | `useNavButtonCards` 收敛，删除的两处与 hook 实现逐字一致 |
| P219 | ✅ | 6 处前端死导出删除（SAMPLE_TEMPLATES/LOCK_ITEM/SETTINGS_ITEM/VaultStateStr/AttachmentCompositeKey/LlmChatPage 冗余再导出），测试同步清理 |
| P220 | ✅ | 2 处未用 React 导入 + 1 处失效 eslint-disable 删除，基线 lint warning 清零 |
| P221 | ✅ | 13 项死函数/类型删除（delta/transport/noise/pdfium/template_service/vault_file_system/profile/storage），按调用图逐项核验 |
| P222 | ✅ | 25 处 pub 可见性收敛 + 1 处死项删除，消费关系一致 |
| P223 | ⏸ | 长函数长期重构（host.rs 主 handler / storage.rs 表域拆分 / AppState::new）——有意留待迭代，详见 §4.3 |
| P224 | ⏸ | 巨型组件长期重构（TrashDetailPanel 1282 行 / SyncPage 848 等）——有意留待迭代，详见 §4.3 |
| P225 | ✅ | 四大簇收敛（行解密闭包/unlock 共享前缀/PIN 凭证写入/附件源路径解析）；唯一错误文案前缀变化（Search→Object）确认无消费方 |
| P226 | ✅ | 三对前端组件收敛为 4 个共享组件（净 -236 行），微差均已声明核实 |

### 3.2 复核发现 N/R 项闭环归档

| 项 | commit | 判定 | 要点 |
|----|--------|------|------|
| N-1 keyset 分页 | 62ee122a | ✅ | P110 引入的同步永久停滞闭环：游标 (有效 HLC, o.id) 全序 + 回退行 SQL 精确过滤 + 会话层节点编码对齐，3 回归测试；残余见 §4.4（R-3） |
| N-2 reencrypt 事务化 | 4d7d75c6 | ✅ | `reencrypt_all` 失败整体回滚（match result，Err 即 drop tx）；config 前置备份 + 写失败自动回滚，CLI 同受益 |
| N-3 streamBuffer 清理 | b9552f25 | ✅ | `llmStore.reset()` 接入 vault-locked 清理链，在途 chunk 竞态闭环 |
| N-4 provider 登记确认 | f493ef3c | ✅ | 原生确认对话框（XSS 不可绕过）+ embedding 通道登记校验；test/check 通道任意 URL 为已声明取舍（固定负载） |
| N-5 OCR 清单补全 | 0c6fbc08 | ✅ | sha256 清单扩至三档 12 文件，官方 HF 源实测一致 |
| N-6 MRZ spawn_blocking | a4bc74aa | ✅ | `ocr_scan_mrz` 与 P113 模式逐字对齐 |
| N-7 locale key | 9687ab71 | ✅ | P120/P122/P125 新增 4 key 双语补齐 |
| N-8 直写③收敛 | cda265d0 | ✅ | `syncPlaintextPref` 导出强制，两处迁移行为等价 |
| N-9 陈旧测试 | 07d84276 | ✅ | 已删命令 mock 测试移除 |
| N-10 P207 闭环 | 70e766ee | ✅ | 路径 1（真实公钥）实施：专用密钥对 + registry/minisig/zip 托管主仓库 + 公钥注入 + 3 防漂移测试，见 §4.1 |
| N-11 失败态/重试 UI | 87a6507c | ✅ | 两处错误占位 Card + 重试，三态可区分 |
| R-1 trash keyset | 5fc032cb | ✅ | trash_items 表 SQL 级 keyset 分页，消除 P110 同构缺陷，回归测试 ×2 |
| R-2 秒/毫秒错配 | 40b5ecd9 | ✅ | `list_trash_changes_since` 按毫秒解释 deleted_at，回归测试锁定 wall == deleted_at ms |
| R-3 游标持久化 | fbe7d945 | ✅ | 迁移 v22 `sync_watermarks.cursor_id`，会话层恢复游标续传，回归测试 ×2；残余窗口见 §4.4 |
| R-4 回滚上抛 | 4ad8e9a8 | ✅（②③） | 回滚助手返回 Result + 调用方并入「automatic rollback FAILED」文案 + toggleable mock fs 失败注入测试；①见 §4.4 |
| R-5 locale | 397f6d84 | ✅ | `settings:link_open_failed` 补入双语 |

---

## §4 未完成项详细讨论

> 本节为合并版报告的核心内容。按影响面排序：N-10（安全面，已闭环）→ P135（死模块处置）→ P223/P224（长期重构）→ 已声明残余窗口（R-3/R-4①）→ 有意保留记录（P209/P206 遗留观察）。

### 4.1 N-10 / P207：Embedding 注册表 minisign 公钥注入 —— ✅ 已闭环（2026-08-03，路径 1）

**用户决策**：实施路径 1（真实公钥）——生成**独立专用密钥对**（与 Tauri updater 密钥 `~/SoloSoul/signing/tauri-updater/secret.key`（原 `~/.tauri/secret.key`，2026-08-03 迁入 `~/SoloSoul/signing/`）隔离，避免单点信任域扩张）；**不新建仓库/组织**，registry + 签名 + 模型 zip 托管于主仓库现有 `resources/models/` 目录（raw.githubusercontent 直接可下载）。

**修复内容**：

| 组件 | 改动 |
|------|------|
| 密钥体系 | `cargo tauri signer generate` 生成专用密钥对（key_id `3C233881DD7399DE`）；私钥已迁至本机 `~/SoloSoul/signing/embed-registry/embed-registry.key`（2026-08-03 统一迁入 `~/SoloSoul/signing/`，目录 700 / 私钥 600，不入库；再签名流程：`cargo tauri signer sign -f <key> -p '' registry.json`） |
| `embed_model.rs` | `REGISTRY_URL` → `https://raw.githubusercontent.com/Gczmy/SoloSoul/main/tauri/src-tauri/resources/models/registry.json`；`EMBED_REGISTRY_PUBKEY_B64`：`None` → `Some("RWTemXPdgTgjPGuPgRxV+e3ng0NH2lgS8HzRbmi0XSlyjYXKI6zGkvXD")`——配置公钥后校验失败即硬失败，**签名防护正式激活** |
| `.gitignore` | `models/*` → `**/models/*`（任意层级前缀，`models/*` 只匹配 .gitignore 所在目录的 models/，无法匹配 `resources/models/`）+ 白名单 `!**/models/registry.json` / `.minisig` / `*.zip`（onnx 仍忽略；git add -n 确认恰好 3 文件入库） |
| 资源入库 | `resources/models/registry.json`（all-MiniLM-L6-v2，download_url 指向同仓库 zip，checksum `sha256:2d07de44...baca4e`）、`registry.json.minisig`（tauri signer 输出，**外层 base64 解包为明文 minisign 格式**——`Signature::decode` 需要明文 untrusted comment + base64 行结构）、`all-MiniLM-L6-v2.zip`（16MB，顶层 model.onnx+tokenizer.json+config.json，与 `download_model` 解压结构一致） |
| 测试 | 新增 3 条：`test_real_registry_signature_end_to_end`（读实际 registry+minisig+编译期公钥真实验证；篡改注册表/换公钥/坏签名均拒绝）、`test_compiled_public_key_is_valid_minisign`（公钥+签名可解析）、`test_committed_zip_checksum_matches_registry`（zip sha256 与 registry checksum 一致性，评审建议防 zip/registry 漂移） |

**关键兼容性结论**（决定发布链路可行性的核心）：tauri signer 产出 **ED 前缀（Blake2b 预哈希）签名**；`minisign_verify::verify(data, sig, false)` 的第三参数是 **`allow_legacy`** 而非 prehashed 标志——签名 `is_prehashed=true` 时自动 Blake2b 数据再验证，与既有 `sign_data` 测试助手（ED+Blake2b）模式逐字一致，故真实签名路径与已测试路径完全吻合。

**验证**：fmt 干净 / workspace check 0 / clippy workspace 0 / embed_model 21 测试全绿 / solo_soul 364 全绿；**远程全链路实测**：raw.githubusercontent 三文件（registry/minisig/zip）可下载，zip sha256 与 registry checksum 一致，minisig 格式（ED 前缀、key_id `de9973dd8138233c`）与编译期公钥匹配。提交 `70e766ee` 已推送。

**已声明取舍**（评审确认）：① 主仓库体积 +16MB（zip 随版本演进，用户决策接受）；② 私钥在 /tmp 待迁移离线保管（操作事项非代码问题）；③ e2e 测试的维护负担（每次更新 registry.json 需重新签名）已在测试注释中明示。

### 4.2 死模块处置（P133/P134/P135 全部闭环）

三模块原合计约 926 行，均确认零生产引用（grep 仅命中模块声明自身）。原报告建议删除属破坏性操作，2026-08-02 用户确认暂缓。**P133 已于 2026-08-03 按用户决策接入为 macOS 默认 OCR 引擎并闭环**（见 4.2.1）；**P134 已于 2026-08-03 按用户决策升级为 feature 门控并闭环**（见 4.2.2）；**P135 已于 2026-08-03 按用户决策反向接入原子写并闭环**（见 4.2.3）。

#### 4.2.1 P133 `crates/solosoul-core/src/ocr/macos_vision.rs`（389 行）—— ✅ 已闭环（2026-08-03）

**用户决策**：macOS 版本加入 macOS Vision Framework 原生 OCR 桥接，**设为 macOS 端默认档位**（仅 macOS 显示该选项，其他端不显示）。

**修复内容**：

| 层 | 改动 |
|----|------|
| core `types.rs` | 新增 `OcrModelTier::Vision` 变体（`dir_name=macos-vision`、`remote_name=vision`、Display/FromStr/serde lowercase `"vision"`），单测覆盖往返 |
| `commands/ocr.rs` | `OcrPreferences::default()` macOS 默认 Vision 否则 Small；`ocr_list_available_tiers` macOS 将 vision 插入首位（共 4 档）其余平台 3 档；`ocr_get_model_status` Vision 恒 builtin（installed+bundled=true）；`ocr_set_active_tier`/`ocr_install_bundled_model`/`ocr_download_model`/`ocr_delete_model` 对 Vision 加平台与「系统内置」守卫；`model_file_size_limit` Vision=>0 |
| `ocr_scan_image` | Vision 分支在 `#[cfg(target_os="macos")]` 属性下调用 `macos_vision::scan_image`（返回 `(String, f64)`，spawn_blocking 包裹，拒绝 PDF）；**注意点：必须用 cfg 属性而非运行时 `cfg!` 剪裁**——`macos_vision` 模块本身 cfg 门控，运行时判断仍会在非 macOS 编译期引用不存在模块（E0432）；`#[cfg(not(target_os="macos"))]` 分支防御性拒绝 |
| `ocr_scan_mrz` | Vision 不产出 MRZ 框线 → 激活为 Vision 时回退 small 档引擎，保证 macOS 默认档位下 MRZ 功能可用 |
| CLI | `tier_size_mb` 穷尽匹配补 `Vision=>0.0`（CLI 模型清单不含该档） |
| 前端 | `platform.ts` 新增 `isMacOSSync()`（缓存判断，未命中返回 false，权威值仍以后端 `ocr_get_active_tier` 为准）；`ipc.ts` `OcrModelStatus.builtin?`；`ocrScanStore`/`useOcrModelManager` 默认 `activeTier` macOS 为 `'vision'`；`OcrSettingsPage` 显示 `status_builtin`、隐藏删除按钮、存储占用空；`OcrPage`/`OcrScanControls` 按图片-only 过滤（Vision 无 PDF 管线）；zh-CN/en-US `ocr.json` 新增 `tier_vision_name`/`tier_vision_description`/`status_builtin` |
| 测试 | `test_ocr_preferences_default_tier` 平台感知；`test_ocr_model_status_camelcase_serde` 补 builtin；新增 `test_ocr_list_available_tiers_platform_aware`（真实调用命令：macOS 首项 vision 共 4 档 / 非 macOS 3 档无 vision）；core `test_vision_tier_parse_display_and_serde` |

**验证**：fmt 干净 / clippy workspace 0 / workspace check 0 / solosoul-core ocr 测试 28 全绿 / solo_soul ocr 测试 23 全绿 / CLI check 0 / tsc 0 / eslint 0 / vitest 484 全绿。

**已声明取舍**（评审确认）：① `_language` 参数对 Vision 分支未透传（Swift 桥接恒为自动语言，如需指定语言需在 `macos_vision.rs` 扩展 `recognitionLanguages`——既定为后续改进）；② 老 macOS 用户若 `ocr_preferences.json` 已存旧档位，升级后保留原选择（尊重既有偏好），全新用户默认 Vision；③ Android target 交叉编译检查受 `ring` NDK 工具链环境限制（非代码问题），但 cfg 属性剪裁已保证非 macOS 不引用门控模块，Windows/Android 编译面无此引用。

#### 4.2.2 P134 `crates/solosoul-core/src/biometric/macos_keychain.rs`（439 行）—— ✅ 已闭环（2026-08-03）

**用户决策**：保留，将 `#[allow(dead_code)]` 升级为**显式规划注释 + `#[cfg(feature = "future-keychain")]` 门控**（脱离默认编译面，消除 dead_code 豁免）。

**修复内容**：

| 文件 | 改动 |
|------|------|
| `Cargo.toml` | 新增 `[features] future-keychain = []`（空 feature，依赖已按 target 门控；注释明示启用前提：Apple Developer Program + Developer ID 签名 + entitlements 声明 keychain-access-groups） |
| `biometric/mod.rs` | 模块声明 `#[cfg(target_os="macos")] #[allow(dead_code)]` → `#[cfg(all(target_os="macos", feature="future-keychain"))]`（移除 dead_code 豁免）；文档注释说明门控语义 |
| `macos_keychain.rs` | 顶部文档补充 P134 门控说明 |

**验证**：
- **feature 关闭（默认）**：workspace check 0 / clippy workspace 0 / CLI 0；core 默认测试 154→150（4 个 macos_keychain 测试随 feature 脱离默认编译面，符合预期）。
- **feature 开启**（`--features future-keychain`）：模块编译通过，biometric 22 测试全绿；**10 个 dead_code 警告全部为「模块尚未接入 platform_storage()」的预期提醒**（struct never constructed / 自由函数与常量 never used / associated items never used），文档已明示该语义；CI 只跑默认 feature 不受污染。

**已声明取舍**（评审确认）：feature 关闭后 macOS target 段 `security-framework`/`core-foundation` 等 4 依赖无引用仍被编译（Cargo 对 target 级声明依赖总是编译）——轻微编译开销、无警告；若追求严格按需可后续改 `optional = true` 挂 feature，但不建议现在做（churn > 收益）。

#### 4.2.3 P135 `crates/solosoul-vault/src/safe_storage.rs`（98 行）—— ✅ 已闭环（2026-08-03，反向接入）

**用户决策**：不删，反向接入——将 config 写入路径（至少 `change_password`/`unlock_with_kdf_upgrade`/`create_account` 三处关键路径）切换到 `write_atomic` 语义，P135 从死代码变为被消费代码，R-4① 的「config 写一半」风险降为近乎不可达。

**修复内容**：

| 层 | 改动 |
|----|------|
| `vault_file_system.rs` | `VaultFileSystem` trait 新增 `write_file_atomic`（默认实现：`local_path()` + 父目录 + `solosoul_vault::safe_storage::write_atomic`）；`SafVaultFileSystem` 覆盖（resolve + 原子写 + **置 dirty 标记**保证后续同步远端） |
| `vault_service.rs` | 新增 `write_config_atomic` 助手（原子写 + `ensure_private_file` + **评审补强：同步收紧 `.bak` 权限到 0600**——`fs::copy` 生成 .bak 为 umask 默认 0644，含同敏感级数据）；新增 `read_config_with_recovery`（主文件合法 JSON 直接用；**缺失或损坏（非 JSON）均回退 `recover_or_load`**：提升孤儿 .tmp、回退 .bak，SAF 场景恢复后置 dirty） |
| 写入切换 | **7 处 config 写入全部切原子**：create_account / create_account_with_id / rollback_reencrypt_and_config（恢复旧 config）/ change_password / unlock_with_kdf_upgrade / reset_security_flags / update_password_hint；另按评审建议把 `save_accounts`（accounts.json 账户清单）一并切原子 |
| 读取接线 | `load_config_and_derive_master_key`（unlock/verify_password 共用前缀）改用 `read_config_with_recovery` |
| 测试 | 新增 6 条：fs 层原子写不残留 .tmp / 孤儿 .tmp 提升 / SAF 原子写置 dirty；service 层端到端「孤儿 .tmp + 主文件缺失 → unlock 恢复成功」/「主文件损坏 .bak 完好 → unlock 回退 .bak」/「.bak 权限收紧 0600」；R-4 mock（FailConfigWriteFs）同步覆盖 `write_file_atomic` 注入 |

**验证**：fmt 干净 / clippy workspace 0 / workspace check 0 / core 156（+3）全绿 / solo_soul 361 / vault 123 / CLI 0。

**已声明取舍**（评审确认）：① 原子写每次产生 `.bak`（一账户一个，随 `remove_dir_all` 清理，非泄漏）；② `read_config_with_recovery` 恢复路径对 SAF 通过 `local_path`（本地临时目录）工作——恢复后已置 dirty 待同步；③ 残余窗口见 §4.4.2——reencrypt 提交后、新 config 落盘前崩溃仍回退旧钥（需 reencrypt 侧 journal 才完全关闭），但账户不再出现**截断/损坏**的 config。

### 4.3 P223/P224：长函数与巨型组件长期重构

原报告定位为长期重构项，建议随功能迭代顺带拆分、不单独安排修复轮次——维持该定位。合并复核补充**当前实测量**与分解优先级建议：

#### 4.3.1 P223 Rust 长函数（当前实测）

| 文件 | 当前行数 | 候选长函数 | 分解建议 |
|------|----------|------------|----------|
| `crates/solosoul-plugin/src/host.rs` | 1711 | 主执行 handler（起始 :264，含 `process_sse` 深度 9） | 按协议帧类型拆分 handler；SSE 解析抽独立模块（此前已抽出部分测试辅助） |
| `crates/solosoul-vault/src/storage.rs` | 7922 | `list_objects`（:2841）、`apply_sync_records_batch`（:2136）、`reencrypt_all`（:740） | 按表域拆模块（objects/profiles/templates/trash/sync_hlc），或抽「行解码」「记录应用」助手簇 |
| `src-tauri/src/lib.rs` | 649 | `AppState::new`（原 :338，重构后位置漂移） | 按依赖组抽 builder（vault/ocr/embedding/sync/plugin 各组初始化独立函数） |

#### 4.3.2 P224 前端巨型组件（当前实测）

| 文件 | 当前行数 | 分解建议 |
|------|----------|----------|
| `src/components/trash/TrashDetailPanel.tsx` | 1282 | 按区块抽子组件（详情头/字段列表/附件区/审计日志区），超大 JSX 拆纯展示组件 |
| `src/pages/sync/SyncPage.tsx` | 848 | 配对流程/设备列表/冲突面板/历史记录各抽独立组件（可复用 P142 hook 模式） |
| `src/pages/settings/TemplateManagerPage.tsx` | 810 | 模板列表/编辑器模态/导入导出面板分离 |
| `src/pages/system/AboutPage.tsx` | 738 | 各卡片区块抽数据驱动子组件 |
| `src/pages/scan/OcrPage.tsx` | 735 | 扫描视图/结果列表/设置面板分离（P140 hook 已收敛模型逻辑，剩 UI 拆分） |

**方法论参考**：P048（手写 hover 全量迁移工具类）与 P217（render-props 改数据透传 + 自定义 memo 比较器）已确立「逐组件等价重构 + 防回归测试」先例，P223/P224 沿用同流程即可。建议**从 P224 的 `TrashDetailPanel` 与 P223 的 storage.rs 按表域拆分开始**（风险隔离最清晰），每个拆分单独 commit + 测试。

### 4.4 已声明残余窗口：R-3 / R-4①

两处均为修复人已声明的窄窗口，属可接受的工程取舍；彻底解法已登记长期改进。

#### 4.4.1 R-3：同步中断后等值组尾部 at-least-once 缺口（低）

- **窗口**：N-1/R-3 修复后，会话**中断**（断网/崩溃/退出）时已持久化水印停在等值组最大值而页游标随同落库（R-3 已修）；残余为——中断后、续传前出现**同毫秒新行且 id < 旧游标**时该行被永久跳过。
- **概率**：需「同毫秒」+「随机 UUID 序逆序」双条件，天文概率，仅影响回退等值组；中断时已存在的行均按 id 序投递完毕。
- **彻底解法**：等值组尾部回扫（续传时对 == 水印组做 id < 游标 的补查）——收益极低，不优先。

#### 4.4.2 R-4①：reencrypt commit 后、config 写前进程崩溃（低）

- **窗口**：`reencrypt_all` 事务提交成功、config.json 写入完成前进程崩溃 → 账户「数据已换新钥、config 仍记旧参数」不可用（毫秒级窗口，彻底解需 journal/双 config）。
- **协同解法（与 P135 联动，已实施）**：见 §4.2.3——config 写入已全部接入 `safe_storage::write_atomic`（.tmp+rename），「写一半」→「要么旧、要么新」，崩溃后孤儿 tmp / 损坏主文件由 `read_config_with_recovery` → `recover_or_load` 恢复；残余为「reencrypt 提交后、新 config 落盘前崩溃」仍回退旧钥（reencrypt 侧需 journal 才完全关闭），但账户不再出现**截断/损坏**的 config——风险从「账户不可用」降为「下次解锁重新升级」。

### 4.5 P209：LEGACY_XOR_KEY（已决策保留）

- **现状**：`LEGACY_XOR_KEY` 仅用于 `legacy_xor_decrypt` 一键解密 <2.0 旧版 XOR 凭证文件并原子迁移为 AES-256-GCM；`legacy.rs` 不可删（`FileBiometricStorage` 为 macOS/iOS 回退的活动存储后端）；当前 `save`/`update` 已只写新格式，**XOR 路径零写入面**。
- **威胁面**：攻击者需同时持有编译产物与 0600 权限的旧文件，且内容为会话密钥非主密钥。
- **决策（2026-08-02 用户确认）**：迁移窗口未关闭，保留 XOR 路径，接受已充分记录的低危风险。
- **关闭条件**：迁移窗口关闭（<2.0 凭证全部完成迁移）后删除整个 `legacy.rs`——建议在下个大版本发布后评估（届时可加「启动时扫描旧凭证数量」日志辅助决策）。

### 4.6 P206 遗留观察：PDF embed 与 object-src CSP（待确认）

- **问题**：`AttachmentPreviewOverlay` 的 PDF `<embed src=data:>` 受 `object-src`（缺省继承 `default-src 'self'`）管辖，现行 CSP 下**本就被拦截**——PDF 附件预览在现行 CSP 中预期不生效。
- **待决策**：① 该路径是否本就弃用（附件预览走其他路径）→ 若是，清理 embed 代码；② 若预期支持 PDF 预览 → 需在 CSP 增加 `object-src data:`（有 XSS 面放大，需评估）。**建议在下一轮 UI 清理时核实附件预览实际路径后决策**，不单独安排修复。

---

## §5 结论与后续建议

1. **审计闭环状态**：80 项问题**全部闭环**并经两轮独立复核（70 项首轮 + 16 项二轮补验 + P133/P134/P135 三项用户决策处置）验证，测试用例较修复前净增 60+；**所有 N/R 项复核发现均已闭环**。
2. **遗留未完成 1 类**（本报告 §4）：
   - P223/P224：长期重构，建议从 `TrashDetailPanel`（P224）与 storage.rs 按表域拆分（P223）开始，沿用 P048/P217 的等价重构 + 防回归测试先例。
   - R-3/R-4① 残余：可接受工程取舍，登记长期改进（等值组尾部回扫 / config journal，见 §4.4）。
   - N-10/P207 已于 2026-08-03 按路径 1 闭环（独立专用公钥注入 + 托管 + 防漂移测试，见 §4.1）——此前建议的路径 3（bundled 兜底）不再需要（公钥已就绪）。
3. **验证指针**：本报告 §3 归档表含全部修复 commit（`f1970c67` 起，N/R 项见 §3.2）；完整修复细节与两轮验证记录在 git 历史中可追溯。

## §6 测试基线参考（当前 HEAD）

- 前端：tsc 0 错误 / eslint 0 警告 / vitest 55 文件 484 用例全绿。
- Rust：`cargo fmt --check` 通过 / `cargo clippy --workspace --all-targets` 零警告 / `cargo test --workspace` 全绿（solo_soul 364、core 156 默认（`future-keychain` 开启时 +4）、crypto 34、plugin 56、sync 47、vault 123）；`solosoul_cli` cargo check 0 错误。
- 工作区干净，当前分支 `main` 与 `origin/main` 同步。
