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
- **遗留未完成/待跟进 4 类**（§4 详细讨论）：
  1. **P223/P224**：长函数/巨型组件长期重构（唯一进行中工作项，§4.1 详述；**P224-①②④⑤ TrashDetailPanel/SyncPage/AboutPage/OcrPage 与 P223-② objects/trash/snapshots 域均已于 2026-08-03 完成拆分** `bc395973`/`8c74253c`/`084cfdd0`/`fd70cc77`/`005fbfdf`/`ae030551`/`ad244d7c`）；
  2. **R-3/R-4①**：已声明残余窗口（§4.2，低风险工程取舍）；
  3. **P209**：legacy XOR 迁移窗口保留（§4.3，决策保留）；
  4. **P206**：PDF embed 与 object-src CSP 遗留观察（§4.4，待核实）。
- **归档说明**：N-10/P207 与 P133/P134/P135 等已闭环项的详细修复记录已压缩至 §3（一行要点 + commit 指针），不再保留 §4 详节。

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
| P206 | ✅* | frame-src `data:` 移除（零 iframe）；style-src `'unsafe-inline'` 结构性保留（227+ 处内联 style，P048 级重构才可移除），残留风险被无 innerHTML + markdown 净化压制。**遗留观察**（PDF embed 与 object-src，见 §4.4） |
| P207 | ✅（N-10 已闭环，70e766ee） | minisign 校验逻辑 + **独立专用公钥已注入**（`EMBED_REGISTRY_PUBKEY_B64`，key_id `3C233881DD7399DE`，与 updater 密钥隔离）+ registry/minisig/zip 托管主仓库 + 3 防漂移测试；签名防护正式激活。专用私钥 2026-08-03 迁入本机 `~/SoloSoul/signing/embed-registry/`（9837a81f） |
| P208 | ✅ | WASI stdio 改默认空槽黑洞，插件输出收敛到 Consent 约束的 host 通道 |
| P209 | ✅（决策保留） | LEGACY_XOR_KEY 仅用于 <2.0 旧凭证一键迁移，零写入面；用户决策保留，关闭条件见 §4.3 |
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
| P133 | ✅ | 按用户决策**接入为 macOS 默认 OCR 引擎**：新增 `OcrModelTier::Vision` 档位（仅 macOS `ocr_list_available_tiers` 返回、置于首位），`OcrPreferences` 默认档 macOS 为 Vision；扫描走 `macos_vision::scan_image`（spawn_blocking + `#[cfg(target_os="macos")]` 属性剪裁，非 macOS 编译不引用门控模块）；MRZ 回退 small 档；前端仅 macOS 显示该档、默认选中、隐藏安装/下载/删除（builtin 标记）、按图片-only 过滤。commit `6e74f691` |
| P134 | ✅ | 按用户决策升级为 `feature = "future-keychain"` 门控（默认关闭，`#[cfg(all(target_os="macos", feature="future-keychain"))]`），移除 `#[allow(dead_code)]` 脱离默认编译面；启用时 10 个 dead_code 警告为「尚未接入 platform_storage()」的预期提醒。commit `f75605ae` |
| P135 | ✅ | 按用户决策**反向接入**（不删）：`VaultFileSystem::write_file_atomic`（trait 默认实现 + SAF 覆盖带 dirty）；7 处 config 写入 + `save_accounts` 全部切原子写，`write_config_atomic` 同步收紧 .bak 权限；`read_config_with_recovery` 接 unlock/verify 读取路径（孤儿 .tmp 提升 + .bak 回退）；R-4① 的「config 写一半」风险降为近乎不可达。commit `b721270c` |
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
| P223 | ⏸（② objects/trash/snapshots 已闭环） | 长函数长期重构——**storage.rs 表域拆分已完成三域**：objects 域抽至 `src/storage/objects.rs`（15 方法，7922→7293 行，`005fbfdf`）+ trash 域抽至 `src/storage/trash.rs`（7 方法，7296→7033 行，`ae030551`）+ snapshots 域抽至 `src/storage/snapshots.rs`（11 方法，7034→6589 行，`ad244d7c`）；host.rs 分簇与 lib.rs 收尾见 §4.1 |
| P224 | ⏸（①②④⑤已闭环） | 巨型组件长期重构——**① TrashDetailPanel**（1282→313 + TrashDetailSections 575 + TrashSnapshotView 526，`bc395973`）、**② SyncPage**（848→276 + ConflictPanel 76 + PairingPanel 135 + DeviceListPanel 440 + SyncHistoryPanel 143，`8c74253c`）、**④ AboutPage**（738→195 + UpdateInfoCard 331 + LinksCard 75 + LegalFooter 19 + MandatoryUpdateOverlay 249，`084cfdd0`）与 **⑤ OcrPage**（738→385 + ScanDropZone 127 + OcrResultList 170 + OcrScanSettingsPanel 203，`fd70cc77`）均已完成，等价重构零行为变更；③（TemplateManager）分解预案见 §4.1 |
| P225 | ✅ | 四大簇收敛（行解密闭包/unlock 共享前缀/PIN 凭证写入/附件源路径解析）；唯一错误文案前缀变化（Search→Object）确认无消费方 |
| P226 | ✅ | 三对前端组件收敛为 4 个共享组件（净 -236 行），微差均已声明核实 |

### 3.2 复核发现 N/R 项闭环归档

| 项 | commit | 判定 | 要点 |
|----|--------|------|------|
| N-1 keyset 分页 | 62ee122a | ✅ | P110 引入的同步永久停滞闭环：游标 (有效 HLC, o.id) 全序 + 回退行 SQL 精确过滤 + 会话层节点编码对齐，3 回归测试；残余见 §4.2（R-3） |
| N-2 reencrypt 事务化 | 4d7d75c6 | ✅ | `reencrypt_all` 失败整体回滚（match result，Err 即 drop tx）；config 前置备份 + 写失败自动回滚，CLI 同受益 |
| N-3 streamBuffer 清理 | b9552f25 | ✅ | `llmStore.reset()` 接入 vault-locked 清理链，在途 chunk 竞态闭环 |
| N-4 provider 登记确认 | f493ef3c | ✅ | 原生确认对话框（XSS 不可绕过）+ embedding 通道登记校验；test/check 通道任意 URL 为已声明取舍（固定负载） |
| N-5 OCR 清单补全 | 0c6fbc08 | ✅ | sha256 清单扩至三档 12 文件，官方 HF 源实测一致 |
| N-6 MRZ spawn_blocking | a4bc74aa | ✅ | `ocr_scan_mrz` 与 P113 模式逐字对齐 |
| N-7 locale key | 9687ab71 | ✅ | P120/P122/P125 新增 4 key 双语补齐 |
| N-8 直写③收敛 | cda265d0 | ✅ | `syncPlaintextPref` 导出强制，两处迁移行为等价 |
| N-9 陈旧测试 | 07d84276 | ✅ | 已删命令 mock 测试移除 |
| N-10 P207 闭环 | 70e766ee | ✅ | 路径 1（真实公钥）实施：专用密钥对 + registry/minisig/zip 托管主仓库 + 公钥注入 + 3 防漂移测试（归档见 §3.1 P207） |
| N-11 失败态/重试 UI | 87a6507c | ✅ | 两处错误占位 Card + 重试，三态可区分 |
| R-1 trash keyset | 5fc032cb | ✅ | trash_items 表 SQL 级 keyset 分页，消除 P110 同构缺陷，回归测试 ×2 |
| R-2 秒/毫秒错配 | 40b5ecd9 | ✅ | `list_trash_changes_since` 按毫秒解释 deleted_at，回归测试锁定 wall == deleted_at ms |
| R-3 游标持久化 | fbe7d945 | ✅ | 迁移 v22 `sync_watermarks.cursor_id`，会话层恢复游标续传，回归测试 ×2；残余窗口见 §4.2 |
| R-4 回滚上抛 | 4ad8e9a8 | ✅（②③） | 回滚助手返回 Result + 调用方并入「automatic rollback FAILED」文案 + toggleable mock fs 失败注入测试；①见 §4.2 |
| R-5 locale | 397f6d84 | ✅ | `settings:link_open_failed` 补入双语 |

---

## §4 未完成项详细讨论

> 本节仅保留**未完成/有意保留**项。已闭环项（N-10/P207、P133/P134/P135 等全部 80 项）的详细修复与验证已压缩归档至 §3（一行要点 + commit 指针），完整细节见 git 各修复 commit。
> 排序：P223/P224（长期重构，唯一进行中工作项）→ R-3/R-4①（已声明残余窗口）→ P209（决策保留）→ P206（遗留观察）。

### 4.1 P223/P224：长函数与巨型组件长期重构（唯一未完成工作项）

**定位**：原报告明确「结构性拆分建议随功能迭代顺带、不单独安排修复轮次」——维持该定位。两轮复核（2026-08-02~03）未发现新增阻断缺陷，本版补齐**当前实测数据**与**逐文件分解预案**，供后续迭代直接取用。**进度**：P224-① TrashDetailPanel（`bc395973`，见 4.1.2 ①）、P224-② SyncPage（`8c74253c`，见 4.1.2 ②）、P224-④ AboutPage（`084cfdd0`，见 4.1.2 ④）、P224-⑤ OcrPage（`fd70cc77`，见 4.1.2 ⑤）与 P223-② objects/trash/snapshots 域（`005fbfdf`/`ae030551`/`ad244d7c`，见 4.1.1 ②）均已于 2026-08-03 完成拆分。

#### 4.1.1 P223 Rust 长函数（实测：host.rs 1711 / storage.rs 6589（已拆 objects/trash/snapshots 域）+ objects.rs 653 + trash.rs 281 + snapshots.rs 467 / lib.rs 649）

**① `crates/solosoul-plugin/src/host.rs`（1711 行）——主拆分对象**

- 当前结构：`SoloHostState`（98-193）→ `register_watermark_fn`（194-263）→ **`register_host_functions`（264-1186，约 923 行，全库最大函数）** → 独立助手函数 18 个（1187-1595）→ 测试（1596-1711）。
- `register_host_functions` 内连续注册 **22 个 `solosoul_*` host 函数**，按功能天然分 6 簇：

  | 簇 | host 函数 | 依赖助手 |
  |----|-----------|----------|
  | 字段/数据访问 | `request_field` / `list_objects` / `list_attachments` / `get_data_structure_tree` / `get_param` | `read_string`/`read_required_string`（共享，rate_limiter 检查模式一致） |
  | HTTP | `http_request` / `http_poll` / `http_read` / `http_close` | `perform_http_async`/`write_http_poll_result`（1522-1595 已独立） |
  | 输出/附件 | `prepare_attachment_copy` / `copy_output_file` / `write_output_file` | `is_under_workspace`/`sanitize_attachment_file_name`/`stamp_result_payload` |
  | 水印 | `image_watermark` / `pdf_watermark` | 独立簇 |
  | 交互 | `request_consent` / `show_dialog` / `log` | 独立簇 |
  | 工具 | `get_timestamp` / `get_locale` / `sleep` / `result` / `post_data` | 独立簇 |

- **分解方案**：抽 6 个 `fn register_<cluster>_fns(linker: &mut Linker<SoloHostState>) -> Result<(), PluginError>`，各自承载对应 `.func_wrap` 链（每个约 100-160 行）；`register_host_functions` 退化为 6 行调度器。rate_limiter 检查抽 `check_rate(host, name)` 助手消除重复。
- **风险**：`Linker<SoloHostState>` 泛型签名逐簇复制；注册顺序与闭包体**只移动不修改**。
- **测试**：既有 host 单测（sanitize/stamp/http，1596-1711）不依赖注册顺序；crate 级集成测试守护插件运行，提取后全量回归即可。
- **产出**：最大函数 923 行 → 6×~140 行 + 调度器 ≤10 行。

**② `crates/solosoul-vault/src/storage.rs`（7922 行 = 生产约 4500 + 测试约 3400）——按表域拆模块（收益最大，✅ 试点已完成）**

- **前提**：当前 ~100 个 `impl VaultStore` 方法已按业务聚簇、域边界清晰，拆分可机械化：

  | 域 | 生产行区间 | 代表方法 | 子模块建议 |
  |----|-----------|----------|-----------|
  | 基础/连接/迁移 | 178-505 | `open` / `init_schema` / `migrate_to_encrypted_format` / `reencrypt_all` / `lock` | 留 storage.rs 根（VaultStore 结构体 + SQL 常量 + 建表/迁移） |
  | Profile | 978-1064 | `save_profile(_tx)` / `load` / `delete` / `list` | `profile.rs` |
  | HLC + Peer 水印 | 1065-1482 | `record_hlc_or_fallback` / peer state / watermark / tombstone | `sync_meta.rs` |
  | 同步变更清单 | 1483-2060 | `list_sync_changes_since(_paginated)` ×4 表域 + keyset | `sync_changes.rs` |
  | 同步应用/冲突 | 2061-2501 | `apply_sync_records_batch` / conflicts / `hard_delete` | `sync_apply.rs` |
  | 对象 | ✅ 2526-3186（已拆） | `list_objects` / `list_object_metadata` / `save_object(_tx)` / `search` | **`objects.rs`（已实施 `005fbfdf`）** |
  | 回收站 | ✅ 2559-2819（已拆） | `trash_and_soft_delete_batch` / `list_trash_items` / `cleanup_expired_trash` | **`trash.rs`（已实施 `ae030551`）** |
  | 快照 | ✅ 2558-3001（已拆） | `save_snapshot(_at)` / `list_snapshots` / `backfill` / `copy` | **`snapshots.rs`（已实施 `ad244d7c`）** |
  | 审计/元数据/embeddings/sys_config | 3894-4287 | `log_structured` / `list_audit_log` / `guide_embeddings` / `read|write_metadata` | `metadata.rs` |
  | 用户模板 | 4288-4500 | `save_user_template` / `list` / `check_field_usage` | `templates.rs` |

- **分解方案**（Rust 同 crate 多 impl 块天然支持）：每域抽 `mod objects;` 等，`impl VaultStore { … }` 连同域内私有助手整体平移；跨域共享助手（`with_tx`、`data_key`、`record_hlc_or_fallback`、`object_row_to_record`）改 `pub(crate)` 或下沉到所属域模块。
- **✅ 试点实施（2026-08-03，`005fbfdf`）——objects 域抽至 `src/storage/objects.rs`**：
  - 15 个对象 CRUD 方法（`save_object`/`save_objects_batch`/`save_object_tx`/`object_row_to_record`/`load_object`/`load_object_tx`/`load_objects_batch`/`list_object_attachment_ids`/`list_objects`/`list_object_metadata`/`delete_object`/`restore_object`/`count_objects`/`list_object_records`/`search_objects`）逐行搬运至子模块 `impl VaultStore { … }`，storage.rs 7922→7293 行。
  - **隐私规则实证**：子模块经 `super::` 直接访问父模块私有项（`data_key()`/`with_tx`/`json_contains_ignore_case`/对象 SQL 常量）——**无需** `pub(crate)` 放宽（Rust 隐私向下可见）；唯二例外是 `save_object_tx`/`load_object_tx` 提为 `pub(crate)`（根模块 `apply_object_sync_record_tx` 2459/2464 跨域调用，隐私只向下可见故父模块必须 `pub(crate)` 才能访问子模块私有）。
  - **风险点①的实证结论**：VaultStore 字段**无需** `pub(crate)`/accessor——子模块可访问父结构体私有字段（`self.conn`/`self.data_key`）。②③ 维持预案：测试留根、`test_key`/`setup` 待子模块测试出现时再放宽。
  - **验证**：逐行保留性 diff 0 行丢失 / fmt 干净 / clippy `--all-targets` 0 警告 / solosoul-vault 123 测试全绿 / workspace + CLI check 0 错误 / code-reviewer GO。
- **✅ trash 域实施（2026-08-03，`ae030551`）——回收站域抽至 `src/storage/trash.rs`**：
  - 7 个回收站方法（`save_trash_item`/`trash_and_soft_delete_batch`/`save_trash_item_tx`/`list_trash_items`/`get_trash_item`/`delete_trash_item`/`cleanup_expired_trash`）逐行搬运，storage.rs 7296→7033 行。
  - `save_trash_item_tx` 提升 `pub(crate)`（根模块 `apply_trash_sync_record_tx` 跨域复用，同 objects 试点）；`delete_object`（兄弟域 objects 的 pub fn）**无需**放宽可见性——private 子模块内的 pub 项在 storage 子树内（含 trash）可见；`log_structured`/`with_tx`/`data_key`/SQL 常量经 super::/self. 直接访问。
  - 根 use 移除 `TrashItem`/`TrashItemSummary`（测试模块改显式 `use crate::{ObjectRecord, Profile, TrashItem}`）。
  - **验证**：逐行保留性 diff 0 行丢失 / fmt 干净 / clippy 0 警告 / solosoul-vault 123 测试全绿 / workspace + CLI check 0 错误 / code-reviewer GO。
- **✅ snapshots 域实施（2026-08-03，`ad244d7c`）——快照域抽至 `src/storage/snapshots.rs`**：
  - 11 个快照方法（`list_snapshots`/`get_snapshot`/`count_snapshots_batch`/`delete_snapshots`/`snapshots_size_batch`/`backfill_missing_snapshots`/`repair_restored_objects`/`backfill_missing_property_labels`/`save_snapshot`/`save_snapshot_at`/`copy_snapshots`）逐行搬运，storage.rs 7034→6589 行。
  - **新实证点：全部方法为 `pub fn`，跨模块调用（src-tauri 命令 snapshot/mod/export_import + 根模块 open 迁移路径 208/214/220）无需放宽可见性**——区别于 objects/trash 的 `_tx` 私有助手提升。
  - **边界决策**：`normalize_details_text` 有意留在根模块（仅被审计日志域 `log_structured` 3100 调用，非快照域方法）；跨域助手（`data_key()`/`get_sys_config`/`set_sys_config`/objects 域 `load_object`/`save_object`/模板域 `load_user_template`）均为固有方法或 storage 子树内可见，经 self. 直接调用。
  - 测试零改动：根测试模块直接调用全部 pub 快照方法（返回类型在签名中全限定，无需类型导入）。
  - **验证**：逐行保留性 diff 0 行丢失 / fmt 干净 / clippy 0 警告 / solosoul-vault 123 测试全绿 / workspace + CLI check 0 错误 / code-reviewer GO。
- **产出（实测校准）**：7922 行 → 6589 根 + 653 objects + 281 trash + 467 snapshots 模块；后续 6 域各 300-900 行。
- **收益**：后续 P109/P110/P213 类同步/对象性能优化与表结构变更的 diff 面缩小约 10×；`reencrypt_all`（740-972）等重函数随迁移收编。

**③ `src-tauri/src/lib.rs`（649 行）——已基本达标，仅收尾**

- 已拆出 9 个 `setup_*` 助手（panic/logging/data_dir/resources/init_state/registry_refresh/locale/theme_polling），`run()`（338-649，约 311 行）为 Tauri Builder 链编排器。
- **剩余工作**：Builder 链可按插件组抽 `build_app(app)`（核心 / 同步 / OCR / LLM / 插件市场分组注册），或维持现状（命名清晰的 setup_* + 311 行编排器已可维护）。**建议优先级最低**，仅在触碰该文件时顺手。

#### 4.1.2 P224 前端巨型组件（实测 5 文件）

**① `src/components/trash/TrashDetailPanel.tsx`（1282 行）——✅ 已于 2026-08-03 完成拆分（`bc395973`）**

- 原结构：5 个组件，`ObjectDetailContent`（40-594，约 554 行，最大）承载 6 个渲染区块；`SnapshotContent`/`SnapshotDataView`/`DynamicGroupSnapshotRow` 为快照展示域。
- **拆分结果（等价重构，零行为变更）**：
  - **`TrashDetailSections.tsx`（新，575 行）**：6 个纯展示子组件——`TrashDetailHeader`（返回/标题/关闭）、`TrashMetaInfo`（删除时间/原位置/保留期/删除者）、`TrashFieldList`（字段预览含 dynamic_group，空则早退 null）、`TrashAttachmentsSection`（active/trash 切换 + 附件行）、`SnapshotSummaryRow`（可折叠快照行）、`TrashDetailActions`（恢复/永久删除）；`TrashSnapshotView.tsx`（新，526 行）：快照展示域逐字平移（SnapshotContent 改 export）。
  - **`TrashDetailPanel.tsx`（1282→313 行）**：保留模态外壳 + `ObjectDetailContent` 编排器（状态与数据加载原样）。
- **验证**：tsc 0 错误 / eslint 0 警告 / 全量 vitest 55 文件 484 用例全绿（基线不变）/ prettier 规范化；逐行保留性 diff 确认无内容丢失；修复机械改名隐患（`() => onToggle` 返回函数不执行 → `onClick={onToggle}` 直调）。
- **遗留**：`SnapshotContent._detailId` 未用 prop（原文件既有，下划线前缀为有意保留）。

**② `src/pages/sync/SyncPage.tsx`（848 行）——✅ 已于 2026-08-03 完成拆分（`8c74253c`）**

- 原结构：`SyncPage`（46-713，约 667 行）已消费 `useSyncPage` hook（数据层已收敛）；`SyncStatusCard`（714-848）独立。
- **拆分结果（等价重构，零行为变更，数据经 useSyncPage 透传）**：
  - **`ConflictPanel.tsx`（新，76 行）**：未解决冲突摘要卡片 + SyncConflictDialog 对话框。
  - **`PairingPanel.tsx`（新，135 行）**：QR 配对卡片 + PairingDialog/SyncShowQrDialog/SyncScanQrDialog 三对话框（`activePairPeer`/`isWaitingFlow`/`confirmLabel` 在编排层计算后透传）。
  - **`DeviceListPanel.tsx`（新，440 行）**：手动同步 + 已发现设备 + 已知设备 + 忘记二次确认 ConfirmDialog（`onRefresh=loadStatus`、`onTrustPeer=trustPeer(id,false)`）。
  - **`SyncHistoryPanel.tsx`（新，143 行）**：折叠式同步活动卡片，`formatNodeId`/`formatHlc` 随迁，空 recentResults 早退 null。
  - **`SyncPage.tsx`（848→276 行）**：缩为编排层——`useSyncPage` 解构 + 四面板 props 透传 + SyncStatusCard 子组件原样保留。
- **验证**：tsc 0 错误 / eslint 0 警告 / 全量 vitest 55 文件 484 用例全绿（基线不变）/ prettier 规范化；逐行保留性 diff 0 行丢失（74 处差异全为 import 再分配/doc 改写/const 行过滤伪报/prop 改名/对话框重写）；code-reviewer GO。
- **有意视觉取舍**：同步活动卡片移至已知设备之后（设备管理聚合语义归组），无功能影响；SyncHistoryPanel 早退 null 后移除块内冗余条件（reviewer 建议）。

**③ `src/pages/settings/TemplateManagerPage.tsx`（810 行）**

- 单组件 747 行，状态密集（20+ useState/useMemo/useCallback），已复用 `DynamicGroupConfig`、`useConfirm`、模板 store。
- **拆分方案**：抽 `TemplateListSection`（列表/搜索/过滤）、`TemplateEditorModal`（编辑器模态，含动态组/字段/废弃字段面板，编辑器内部状态随之内聚）、`SampleGalleryModal`（示例库）、`ImportExportPanel`（导入导出/复制）。主组件仅保留列表态与「打开哪个模态」。
- 产出：747 → 主组件 ~200 行 + 4 子组件各 100-180 行。**状态内聚是难点**，编辑器相关 state 全部随模态迁移。

**④ `src/pages/system/AboutPage.tsx`（738 行）——✅ 已于 2026-08-03 完成拆分（`084cfdd0`）**

- 原结构：单组件 712 行（数据来自 `useUpdateChecker` 与系统命令）。各卡片区块（版本/更新、链接、版权、强制更新覆盖层）彼此零耦合。
- **拆分结果（等价重构，零行为变更，数据与回调经 AboutPage 透传）**：
  - **`UpdateInfoCard.tsx`（新，331 行）**：版本/更新信息卡片（版本行 + 更新状态徽标 + 失败重试 + 下载进度），`friendlyPlatform` 随迁（唯一消费方）。
  - **`LinksCard.tsx`（新，75 行）**：外部链接卡片（shell open + P231 失败 toast 原样）。
  - **`LegalFooter.tsx`（新，19 行）**：静态版权页脚，零依赖。
  - **`MandatoryUpdateOverlay.tsx`（新，249 行）**：强制更新全屏覆盖层（createPortal 到 document.body）；原 `{isMandatory && createPortal(...)}` 改写为 `if (!isMandatory) return null; return createPortal(...)`（语义等价）。
  - **`AboutPage.tsx`（738→195 行）**：缩为编排层——保留 `<style>` 块（release-notes-md 样式被两面板 SafeMarkdown 消费）+ 品牌头部 + links 数组构建 + AppShell 壳。
- **验证**：tsc 0 错误 / eslint 0 警告 / AboutPage.test.tsx 6 用例全绿（未改动）/ 全量 vitest 55 文件 484 用例全绿 / prettier 规范化；逐行保留性 diff 0 行丢失（14 处差异全为 prettier 行合并/门控改写，grep 逐条核实）；code-reviewer GO（`AboutLink` 接口 export 冗余 nitpick 已修正）。

**⑤ `src/pages/scan/OcrPage.tsx`（738 行）——✅ 已于 2026-08-03 完成拆分（`fd70cc77`）**

- 原结构：单组件 712 行；P140 已把模型安装/下载/tier 逻辑收敛进 `useOcrModelManager`，剩余为扫描流程 UI 与设置面板。
- **拆分结果（等价重构，零行为变更，数据与回调经 OcrPage 透传）**：
  - **`ScanDropZone.tsx`（新，127 行）**：扫描入口面板（通用/MRZ 模式切换 + 选择文件/拍照），导出 `type ScanMode`（规范出处）。
  - **`OcrResultList.tsx`（新，170 行）**：扫描中指示 + 通用 OCR/MRZ 结果卡片 + 导入命名 PromptDialog。
  - **`OcrScanSettingsPanel.tsx`（新，203 行）**：模型档位选择/安装/下载 + 移动端系统 OCR 说明（消费 useOcrModelManager 数据）。
  - **`OcrPage.tsx`（738→385 行）**：缩为编排层——全部 state/logic 保留（performScan/自动扫描 useEffect/选择文件/拍照/导入流程/指南页），useOcrModelManager 解构 + 三面板 props 透传；`handleScanModeChange` 保持原内联清理语义（setScanMode + 清空结果）。
- **验证**：tsc 0 错误 / eslint 0 警告 / 全量 vitest 55 文件 484 用例全绿（OcrPage.test.tsx 未改动）/ prettier 规范化；逐行保留性 diff 0 行丢失（21 处差异全为 import 再分配/doc 改写/prop 改名/prettier 行合并）；code-reviewer GO（redundant fragment 与 ScanMode 出处注释两 nitpick 已采纳）。

#### 4.1.3 重构流程与验收标准（沿用 P048/P217 先例）

1. **等价重构铁律**：每个拆分**不改变任何行为**——纯移动代码 + props 透传，禁止顺手改逻辑；行为差异一律单独 commit。
2. **防回归测试**：前端拆分后跑 `npx tsc --noEmit` + `npx eslint` + `npx vitest run`（现 55 文件 484 用例全绿为基线）；Rust 拆分后跑 `cargo fmt --check` + `cargo clippy --workspace --all-targets` + `cargo test --workspace`（现 675+ 全绿为基线）。目标：**拆分前后测试零变化**。
3. **执行顺序建议（风险隔离从高到低）**：
   - 试点：✅ **P224-① TrashDetailPanel**（`bc395973`，2026-08-03 完成）→ 前端拆分节奏已确立；
   - 其次：✅ **P223-② storage.rs 表域拆分**（`005fbfdf` objects 域试点 + `ae030551` trash 域 + `ad244d7c` snapshots 域，2026-08-03 完成 → 模式已确立，下一域 sync_meta/metadata 按此推进）；
   - 然后：✅ **P224-② SyncPage 四面板**（`8c74253c`）、✅ **P224-④ AboutPage 四面板**（`084cfdd0`）与 ✅ **P224-⑤ OcrPage 三面板**（`fd70cc77`，均 2026-08-03 完成）→ 编排层 + 数据经 hook/props 透传模式已确立；剩余 P223-① host.rs 分簇；
   - 最后：P224-③（TemplateManager 状态密集，剩余唯一前端巨型组件）与 P223-③ lib.rs（已达标，收尾项）。
4. **产出约束**：每个拆分**单独 commit**（一项一提交），commit message 注明「纯移动/等价重构」；本报告 §3 归档表随拆分补充新行。

**当前建议**：不单独安排修复轮次；**P224-①②④⑤ 与 P223-②（objects/trash/snapshots）已完成**（`bc395973`/`8c74253c`/`084cfdd0`/`fd70cc77`/`005fbfdf`/`ae030551`/`ad244d7c`），下次触碰任一文件时按上述预案顺带执行，优先 P223-② 下一域（sync_meta/metadata）与 P224-③（TemplateManager）。

### 4.2 已声明残余窗口：R-3 / R-4①

两处均为修复人已声明的窄窗口，属可接受的工程取舍；彻底解法已登记长期改进。

#### 4.2.1 R-3：同步中断后等值组尾部 at-least-once 缺口（低）

- **窗口**：N-1/R-3 修复后，会话**中断**（断网/崩溃/退出）时已持久化水印停在等值组最大值而页游标随同落库（R-3 已修）；残余为——中断后、续传前出现**同毫秒新行且 id < 旧游标**时该行被永久跳过。
- **概率**：需「同毫秒」+「随机 UUID 序逆序」双条件，天文概率，仅影响回退等值组；中断时已存在的行均按 id 序投递完毕。
- **彻底解法**：等值组尾部回扫（续传时对 == 水印组做 id < 游标 的补查）——收益极低，不优先。

#### 4.2.2 R-4①：reencrypt commit 后、config 写前进程崩溃（低）

- **窗口**：`reencrypt_all` 事务提交成功、config.json 写入完成前进程崩溃 → 账户「数据已换新钥、config 仍记旧参数」不可用（毫秒级窗口，彻底解需 journal/双 config）。
- **协同解法（与 P135 联动，已实施）**：见 §3.1 P135 归档——config 写入已全部接入 `safe_storage::write_atomic`（.tmp+rename），「写一半」→「要么旧、要么新」，崩溃后孤儿 tmp / 损坏主文件由 `read_config_with_recovery` → `recover_or_load` 恢复；残余为「reencrypt 提交后、新 config 落盘前崩溃」仍回退旧钥（reencrypt 侧需 journal 才完全关闭），但账户不再出现**截断/损坏**的 config——风险从「账户不可用」降为「下次解锁重新升级」。

### 4.3 P209：LEGACY_XOR_KEY（已决策保留）

- **现状**：`LEGACY_XOR_KEY` 仅用于 `legacy_xor_decrypt` 一键解密 <2.0 旧版 XOR 凭证文件并原子迁移为 AES-256-GCM；`legacy.rs` 不可删（`FileBiometricStorage` 为 macOS/iOS 回退的活动存储后端）；当前 `save`/`update` 已只写新格式，**XOR 路径零写入面**。
- **威胁面**：攻击者需同时持有编译产物与 0600 权限的旧文件，且内容为会话密钥非主密钥。
- **决策（2026-08-02 用户确认）**：迁移窗口未关闭，保留 XOR 路径，接受已充分记录的低危风险。
- **关闭条件**：迁移窗口关闭（<2.0 凭证全部完成迁移）后删除整个 `legacy.rs`——建议在下个大版本发布后评估（届时可加「启动时扫描旧凭证数量」日志辅助决策）。

### 4.4 P206 遗留观察：PDF embed 与 object-src CSP（待确认）

- **问题**：`AttachmentPreviewOverlay` 的 PDF `<embed src=data:>` 受 `object-src`（缺省继承 `default-src 'self'`）管辖，现行 CSP 下**本就被拦截**——PDF 附件预览在现行 CSP 中预期不生效。
- **待决策**：① 该路径是否本就弃用（附件预览走其他路径）→ 若是，清理 embed 代码；② 若预期支持 PDF 预览 → 需在 CSP 增加 `object-src data:`（有 XSS 面放大，需评估）。**建议在下一轮 UI 清理时核实附件预览实际路径后决策**，不单独安排修复。

---

## §5 结论与后续建议

1. **审计闭环状态**：80 项问题**全部闭环**并经两轮独立复核（70 项首轮 + 16 项二轮补验 + P133/P134/P135 三项用户决策处置）验证，测试用例较修复前净增 60+；**所有 N/R 项复核发现均已闭环**。
2. **遗留未完成/待跟进 4 类**（本报告 §4）：
   - **P223/P224**：长函数/巨型组件长期重构（唯一进行中工作项，§4.1 含逐文件分解预案与执行顺序）；不单独安排修复轮次，随功能迭代顺带执行。**P224-①②④⑤ TrashDetailPanel/SyncPage/AboutPage/OcrPage（`bc395973`/`8c74253c`/`084cfdd0`/`fd70cc77`）与 P223-② objects/trash/snapshots 域（`005fbfdf`/`ae030551`/`ad244d7c`）已完成**，当前优先 P223-② 下一域（sync_meta/metadata）与 P224-③（TemplateManager）。
   - **R-3/R-4①**：已声明残余窗口，可接受工程取舍，登记长期改进（等值组尾部回扫 / config journal，见 §4.2）。
   - **P209**：legacy XOR 迁移窗口保留，建议下个大版本发布后评估关闭（见 §4.3）。
   - **P206**：PDF embed 与 object-src CSP 遗留观察，待附件预览路径核实后决策（见 §4.4）。
3. **验证指针**：本报告 §3 归档表含全部修复 commit（`f1970c67` 起，N/R 项见 §3.2；P133=`6e74f691`、P134=`f75605ae`、P135=`b721270c`、N-10=`70e766ee`）；完整修复细节与两轮验证记录在 git 历史中可追溯。

## §6 测试基线参考（当前 HEAD）

- 前端：tsc 0 错误 / eslint 0 警告 / vitest 55 文件 484 用例全绿。
- Rust：`cargo fmt --check` 通过 / `cargo clippy --workspace --all-targets` 零警告 / `cargo test --workspace` 全绿（solo_soul 364、core 156 默认（`future-keychain` 开启时 +4）、crypto 34、plugin 56、sync 47、vault 123）；`solosoul_cli` cargo check 0 错误。
- 工作区干净，当前分支 `main` 与 `origin/main` 同步。
