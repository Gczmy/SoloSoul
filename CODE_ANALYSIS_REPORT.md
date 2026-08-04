# SoloSoul 代码审计修复报告（合并版）

> 最后更新：2026-08-04
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
| Rust 测试 | ✅ 全部通过 | ✅ 全部通过（solo_soul 365 / core 156（默认；`future-keychain` 开启时 +4）/ crypto 34 / plugin 56 / sync 47 / vault 140（方案 B +4、#1 墓碑传播 +6、N-14 墓碑清理 +7）） |

## §2 总览：80 项问题处置结果

- **审计问题清单共 80 项**（P001-P007、P101-P142、P201-P231）。
- **80 项问题全部闭环**（78 项可执行修复 + P133 用户决策接入 + P134 用户决策门控 + P135 用户决策反向接入 + **N-10/P207 路径 1 公钥注入闭环**），其中 P104/P206 为部分修复/部分保留，P209 为用户决策保留。
- **遗留未完成/待跟进 4 类**（§4 详细讨论）：
  1. **P223/P224**：长函数/巨型组件长期重构——**已全部完成**（§4.1 归档；**P224-①②③④⑤ TrashDetailPanel/SyncPage/TemplateManagerPage/AboutPage/OcrPage 与 P223-① host.rs 六簇分簇、P223-② objects/trash/snapshots/sync_meta/sync_changes/sync_apply/metadata/profile 八域、P223-③ lib.rs 收尾均已完成拆分** `bc395973`/`8c74253c`/`2bdc5fdd`/`084cfdd0`/`fd70cc77`/`0f0a37ff`/`005fbfdf`/`ae030551`/`ad244d7c`/`22e1a20f`/`14eff424`/`89446aeb`/`508f9445`/`cef5776c`/`a7d5925d`）；
  2. **R-3/R-4①**：均已闭环——R-3 随方案 B 阶段 3 关闭（§4.2.1，2026-08-04）；R-4① 经方案 2（config.json.pending 两阶段交换 + probe 判定）彻底关闭（§4.2.2，2026-08-04）；
  3. **P209**：legacy XOR 迁移窗口保留（§4.3，决策保留）；
  4. ~~**P206**：PDF embed 与 object-src CSP 遗留观察~~（§4.4，✅ 已闭环 `d446dc0e`，不再属于遗留）。
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
| P206 | ✅ | frame-src `data:` 移除（零 iframe）；style-src `'unsafe-inline'` 结构性保留（227+ 处内联 style，P048 级重构才可移除）；**PDF 预览修复**：CSP 增加 `object-src data:`（`d446dc0e`，方案 A），桌面端 PDF 附件预览恢复，XSS 面评估见 §4.4 |
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
| P223 | ⏸（①②③已全部闭环） | 长函数长期重构——**① `register_host_functions` 六簇分簇**（923 行→7 行调度器 + 6 簇注册函数，1711→1746 行，`0f0a37ff`+`3494a8d3`）；**② storage.rs 表域拆分八域全部完成**：objects 域抽至 `src/storage/objects.rs`（15 方法，7922→7293 行，`005fbfdf`）+ trash 域抽至 `src/storage/trash.rs`（7 方法，7296→7033 行，`ae030551`）+ snapshots 域抽至 `src/storage/snapshots.rs`（11 方法，7034→6589 行，`ad244d7c`）+ sync_meta 域抽至 `src/storage/sync_meta.rs`（22 方法，6589→6170 行，`22e1a20f`）+ sync_changes 域抽至 `src/storage/sync_changes.rs`（8 方法，6171→5595 行，`14eff424`）+ sync_apply 域抽至 `src/storage/sync_apply.rs`（15 方法，5595→5153 行，`89446aeb`）+ metadata 域抽至 `src/storage/metadata.rs`（审计/元数据/embeddings/sys_config/用户模板四簇 20 方法，5151→4542 行，`508f9445`）+ profile 域抽至 `src/storage/profile.rs`（5 方法，4544→4433 行，`cef5776c`）；**③ lib.rs Builder 链按插件组分簇**（setup_app 命名函数 + 单分发器 + 5 簇，`a7d5925d`，见 §4.1.1 ③） |
| P224 | ⏸（①②③④⑤已闭环） | 巨型组件长期重构——**① TrashDetailPanel**（1282→313 + TrashDetailSections 575 + TrashSnapshotView 526，`bc395973`）、**② SyncPage**（848→276 + ConflictPanel 76 + PairingPanel 135 + DeviceListPanel 440 + SyncHistoryPanel 143，`8c74253c`）、**③ TemplateManagerPage**（810→328 + useTemplateEditor hook 371 + TemplateListSection 198 + TemplateEditorModal 100 + SampleGallerySection 50，`2bdc5fdd`）、**④ AboutPage**（738→195 + UpdateInfoCard 331 + LinksCard 75 + LegalFooter 19 + MandatoryUpdateOverlay 249，`084cfdd0`）与 **⑤ OcrPage**（738→385 + ScanDropZone 127 + OcrResultList 170 + OcrScanSettingsPanel 203，`fd70cc77`）全部完成，等价重构零行为变更 |
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
| R-4① 彻底关闭 | 55399364 | ✅ | 方案 2（config.json.pending 两阶段交换 + probe 判定）——`solosoul_vault::probe_data_key` 只读自由函数 + `recover_pending_reencrypt`（promote/discard/保留）+ 两调用方 pending 生命周期 + 生物识别/PIN 凭证同步 + 5 回归测试（详见 §4.2.2） |
| R-5 locale | 397f6d84 | ✅ | `settings:link_open_failed` 补入双语 |
| 方案B-1 objects HLC | 1a33f513 | ✅ | objects 域本地写统一 HLC + 节点规范化修复（死循环根因） |
| 方案B-2 三域 HLC | c32fbced | ✅ | trash/profile/user_template 域统一 HLC + 软删对象新 HLC 堵同步缺口 |
| 方案B-3 回填退休 | a8407226 | ✅ | 迁移 v23 存量 HLC 回填（wall 按各表回退语义逐字节复刻）+ 回退兜底保留，R-3 窗口关闭 |

---

## §4 未完成项详细讨论

> 本节仅保留**未完成/有意保留**项。已闭环项（N-10/P207、P133/P134/P135 等全部 80 项）的详细修复与验证已压缩归档至 §3（一行要点 + commit 指针），完整细节见 git 各修复 commit。
> 排序：P223/P224（长期重构，✅ 已全部完成）→ R-3/R-4①（✅ 已闭环）→ P209（决策保留）→ P206（已闭环）。

### 4.1 P223/P224：长函数与巨型组件长期重构（✅ 已全部完成，本节为实施归档）

**定位**：原报告明确「结构性拆分建议随功能迭代顺带、不单独安排修复轮次」——维持该定位。两轮复核（2026-08-02~03）未发现新增阻断缺陷，本版补齐**当前实测数据**与**逐文件分解预案**，供后续迭代直接取用。**进度**：P224-① TrashDetailPanel（`bc395973`）、P224-② SyncPage（`8c74253c`）、P224-③ TemplateManagerPage（`2bdc5fdd`）、P224-④ AboutPage（`084cfdd0`）、P224-⑤ OcrPage（`fd70cc77`）（分别见 4.1.2 ①-⑤）、P223-① host.rs 六簇分簇（`0f0a37ff`+`3494a8d3`，见 4.1.1 ①）与 P223-② objects/trash/snapshots/sync_meta/sync_changes/sync_apply/metadata/profile 八域（`005fbfdf`/`ae030551`/`ad244d7c`/`22e1a20f`/`14eff424`/`89446aeb`/`508f9445`/`cef5776c`，见 4.1.1 ②）全部完成拆分。

#### 4.1.1 P223 Rust 长函数（实测：host.rs 1746（已六簇分簇）/ storage.rs 4433（已拆八域）+ objects.rs 652 + trash.rs 281 + snapshots.rs 465 + sync_meta.rs 441 + sync_changes.rs 595 + sync_apply.rs 464 + metadata.rs 634 + profile.rs 140 / lib.rs 982（已按插件组分簇））

**① `crates/solosoul-plugin/src/host.rs`（1711→1746 行）——✅ 已于 2026-08-03 完成六簇分簇（`0f0a37ff` 重构 + `3494a8d3` 报告）**

- 原结构：`SoloHostState`（98-193）→ `register_watermark_fn`（194-263）→ **`register_host_functions`（264-1186，约 923 行，原全库最大函数）** → 独立助手函数 18 个（1187-1595）→ 测试（1596-1711）。
- **拆分结果（等价重构，零行为变更，闭包体逐字节搬运）**：
  - **`register_host_functions` 退化为 7 行调度器**：按簇调用 6 个注册函数。
  - **6 簇**：`register_field_access_fns`（request_field/list_objects/list_attachments/get_data_structure_tree/get_param）、`register_http_fns`（http_request/poll/read/close）、`register_output_fns`（prepare_attachment_copy/copy_output_file/write_output_file）、`register_watermark_host_fns`（image_watermark/pdf_watermark，cfg 双分支——桌面 `register_watermark_fn`/移动内联 NOT_IMPLEMENTED——随迁）、`register_interaction_fns`（request_consent/show_dialog/log）、`register_util_fns`（get_timestamp/get_locale/sleep/result/post_data）。
  - **命名规避**：既有 `register_watermark_fn`（单数参数化助手）不动，簇命名 `register_watermark_host_fns`；`list_attachments` 归 field 簇（与域表一致）。
  - **注册顺序变化对 runtime 零影响**：Linker 名→闭包映射无序敏感（名唯一），host 单测不依赖顺序。
- **验证**：块级等价性 diff 22/22 逐字节一致（0 差异）/ cargo check 0 错误 / clippy `--all-targets` 0 警告 / solosoul-plugin 56 测试全绿 / fmt 干净 / workspace + CLI check 0 错误 / code-reviewer GO。
- **暂缓微优化**：rate_limiter 检查抽 `check_rate(host, name)` 助手去重——✅ 已于 2026-08-04 完成（`57d10448`，7 处收敛，行为逐字等价）。

**② `crates/solosoul-vault/src/storage.rs`（7922 行 = 生产约 4500 + 测试约 3400）——按表域拆模块（收益最大，✅ 试点已完成）**

- **前提**：当前 ~100 个 `impl VaultStore` 方法已按业务聚簇、域边界清晰，拆分可机械化：

  | 域 | 生产行区间 | 代表方法 | 子模块建议 |
  |----|-----------|----------|-----------|
  | 基础/连接/迁移 | 178-505 | `open` / `init_schema` / `migrate_to_encrypted_format` / `reencrypt_all` / `lock` | 留 storage.rs 根（VaultStore 结构体 + SQL 常量 + 建表/迁移） |
  | Profile | 978-1064 | `save_profile(_tx)` / `load` / `delete` / `list` | `profile.rs` |
  | HLC + Peer 水印 | ✅ 1068-1486（已拆） | `record_hlc_or_fallback` / peer state / watermark / tombstone | **`sync_meta.rs`（已实施 `22e1a20f`）** |
  | 同步变更清单 | ✅ 1483-2060（已拆） | `list_sync_changes_since(_paginated)` ×4 表域 + keyset | **`sync_changes.rs`（已实施 `14eff424`）** |
  | 同步应用/冲突 | ✅ 2061-2501（已拆） | `apply_sync_records_batch` / conflicts / `hard_delete` | **`sync_apply.rs`（已实施 `89446aeb`）** |
  | 对象 | ✅ 2526-3186（已拆） | `list_objects` / `list_object_metadata` / `save_object(_tx)` / `search` | **`objects.rs`（已实施 `005fbfdf`）** |
  | 回收站 | ✅ 2559-2819（已拆） | `trash_and_soft_delete_batch` / `list_trash_items` / `cleanup_expired_trash` | **`trash.rs`（已实施 `ae030551`）** |
  | 快照 | ✅ 2558-3001（已拆） | `save_snapshot(_at)` / `list_snapshots` / `backfill` / `copy` | **`snapshots.rs`（已实施 `ad244d7c`）** |
  | 审计/元数据/embeddings/sys_config | ✅ 3894-4287（已拆） | `log_structured` / `list_audit_log` / `guide_embeddings` / `read|write_metadata` | **`metadata.rs`（已实施 `508f9445`）** |
  | 用户模板 | ✅ 4288-4500（已拆） | `save_user_template` / `list` / `check_field_usage` | **并入 `metadata.rs`（已实施 `508f9445`，用户决策合并）** |

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
- **✅ sync_meta 域实施（2026-08-03，`22e1a20f`）——同步元数据域抽至 `src/storage/sync_meta.rs`**：
  - 22 个方法（HLC 读写 `get|set_record_hlc(_tx)`/`record_hlc_or_fallback` + Peer 状态 `save|load_peer_state`/`list_peers`/`set_peer_trusted`/`delete_peer` + 水印 `update_peer_watermark(_with_cursor)`/`get_peer_watermark_cursor`/`get_peer_watermark` + 墓碑 `new_tombstone|local_hlc`/`max_hlc_wall_time_for_node`/`record_tombstone`/`list_tombstones_since` + `hlc_after_watermark`）逐行搬运，storage.rs 6589→6170 行。
  - **最高耦合域实证：12 个跨域私有助手提 `pub(crate)`**（隐私只向下可见，父模块必须 pub(crate) 才能访问子模块私有）——根模块 sync_changes（`record_hlc_or_fallback`/`parse_time_ms`/`list_tombstones_since`/`hlc_after_watermark`）、sync-apply（`get|set_record_hlc_tx`/`set_record_hlc`/`local_node_id`/`new_tombstone|local_hlc`/`record_tombstone`）、delete_profile 与模板删除（`record_tombstone`）跨域复用；`max_hlc_wall_time_for_node` 仅域内调用保持私有；9 个 pub API 可见性不变（solosoul-sync/CLI 跨 crate 调用）。
  - 共享常量经 `super::` 引用（HLC_GET_SQL/HLC_SET_SQL）；域内自调用（`record_tombstone`→`set_record_hlc`）pub(crate) 自洽。
  - **验证**：逐行保留性 diff 0 行丢失（12 处差异全为 pub(crate) 提升签名 + fmt 换行）/ fmt 干净 / clippy 0 警告 / solosoul-vault 123 测试全绿 / workspace + CLI check 0 错误 / code-reviewer GO。
- **✅ sync_changes 域实施（2026-08-03，`14eff424`）——同步变更清单域抽至 `src/storage/sync_changes.rs`**：
  - 8 个方法（pub 分发器 `list_sync_changes_since`/`list_sync_changes_since_paginated`——四表域 profiles/objects/user_templates/trash_items 路由 + N-1/R-1 keyset 分页；私有实现 `list_profile/object/user_template/trash_changes_since` 与 `list_object/trash_changes_since_limited`）逐行搬运，storage.rs 6171→5595 行。
  - **可见性决策：4 个 pub API 保持 pub（solosoul-sync delta.rs:60 跨 crate 调用 + src-tauri + 根测试模块），4 个私有实现保持私有（仅域内分发器调用）**——区别于 sync_meta 的 12 个 pub(crate) 提升（本域私有方法无跨域调用方）。
  - 共享设施经 `super::` 访问（`data_key()`/`OBJECT_COLUMNS`，隐私向下可见无需放宽）；`parse_time_ms`/`hlc_after_watermark` 属 sync_meta 域已是 pub(crate) 直接消费；`decrypt_field`/`decrypt_text_field` 从 `crate::encryption` 导入（根模块 use 块不自动传递）。
  - **验证**：逐行保留性 diff 42 处差异全为 4 空格缩进减少（impl 内方法随子模块降级，SQL 多行字符串续行缩进随之右移）零内容丢失 / fmt 干净 / clippy 0 警告 / solosoul-vault 123 测试全绿 / workspace + CLI check 0 错误 / code-reviewer GO。
- **✅ sync_apply 域实施（2026-08-03，`89446aeb`）——同步应用域抽至 `src/storage/sync_apply.rs`**：
  - 15 个方法（pub API ×8：`apply_sync_record`/`apply_sync_records_batch`/`save_sync_conflict`/`list_sync_conflicts`/`get_sync_conflict`/`get_sync_conflict_local_data`/`delete_sync_conflict`/`resolve_sync_conflict`；私有 ×7：`apply_sync_record_tx`/`record_hlc_is_newer`/`hard_delete_record`/`apply_profile|object|user_template|trash_sync_record_tx`）逐行搬运，storage.rs 5595→5153 行。
  - **可见性决策：8 个 pub API 保持 pub（solosoul-sync delta.rs 调用 `apply_sync_records_batch`/`save_sync_conflict`/`get_sync_conflict_local_data`；src-tauri commands/sync.rs 调用 `list_sync_conflicts`/`get_sync_conflict`/`resolve_sync_conflict`；根测试模块调用 `apply_sync_record`/`apply_sync_records_batch`），7 个私有实现保持私有（仅域内消费）**——同 sync_changes 模式。
  - 共享设施经 `super::` 访问根模块私有 `with_tx` 自由函数；跨域 pub(crate) 助手按原路径引用（objects 域 `save_object_tx`/`load_object_tx`、trash 域 `save_trash_item_tx`、sync_meta 域 `now_rfc3339`/`set_record_hlc*`/`get_record_hlc_tx`/`record_tombstone`）；`serde::Deserialize` 随域迁入（`ObjectRecord`/`UserTemplate` 的 `::deserialize` trait 调用需要；`TrashItem` 为固有方法故无需），根模块该导入因零消费移除。
  - **验证**：逐行保留性 diff 0 丢失 0 新增 / fmt 干净 / clippy 0 警告 / solosoul-vault 123 测试全绿（基线不变）/ workspace + CLI check 0 错误 / code-reviewer GO。
- **✅ metadata 域实施（2026-08-03，`508f9445`）——审计/元数据/embeddings/sys_config/用户模板四簇抽至 `src/storage/metadata.rs`**：
  - 20 个方法（四簇：① 审计日志 `log_structured` + 私有 `normalize_details_text` 助手 + `list_audit_log`；② 私有元数据存取 `read_metadata`/`write_metadata`（供根模块 sync 节点状态方法复用）；③ Guide embeddings for RAG `save|list|clear|count_guide_embeddings` 与 sys_config `get|set_sys_config`（snapshots 域跨域复用）；④ 用户模板 `save_user_template(_tx)`/`load_user_template(_tx)`/`list_user_templates`/`find_user_template_by_content_hash`/`delete_user_template`/`count_user_templates`/`check_field_usage`）逐行搬运，storage.rs 5151→4542 行。**用户决策：审计+元数据+用户模板合并为一域 metadata.rs（原预案 templates.rs 独立拆分取消）**。
  - **4 个跨域私有助手提 `pub(crate)`**：`read_metadata`/`write_metadata`（根模块 `get_sync_node_id` 等 sync 状态方法调用）、`save_user_template_tx`/`load_user_template_tx`（sync_apply.rs:450/440 兄弟域调用）；`normalize_details_text` 仅被 `log_structured` 消费随域迁入保持私有。
  - **边界修复记录**：首轮提取区间含 impl 闭合 `}` 与 `#[cfg(test)]` 行致根模块未闭合，已补回（结构健全，fmt/clippy/测试全绿佐证）；根模块 `}` 与 `#[cfg(test)]` 间补空行。
  - 共享设施经 `super::` 访问（`USER_TEMPLATE_SAVE_SQL`/`USER_TEMPLATE_LOAD_SQL` 根常量）；`decrypt_text_field`/`encrypt_text_field` + `DataEncryptionKey` 从 `crate::encryption` 导入；`record_tombstone` 属 sync_meta 域 pub(crate)；`data_key()` 隐私向下可见。
  - **验证**：逐行保留性 diff 差异仅 4 处 pub(crate) 提升 + `write_metadata` 签名 fmt 多行重排 + 续行缩进，零内容丢失 / fmt 干净 / clippy 0 警告 / solosoul-vault 123 测试全绿（基线不变）/ workspace + CLI check 0 错误 / code-reviewer GO。
- **产出（实测校准，已全部完成）**：7922 行 → 4433 根 + 652 objects + 281 trash + 465 snapshots + 441 sync_meta + 595 sync_changes + 464 sync_apply + 634 metadata + **140 profile（`cef5776c`，八域收尾）**模块；**八域全部拆分完毕，无剩余未拆域**。
- **收益**：后续 P109/P110/P213 类同步/对象性能优化与表结构变更的 diff 面缩小约 10×；`reencrypt_all`（740-972）等重函数随迁移收编。

**③ `src-tauri/src/lib.rs`（649→982 行）——✅ 已于 2026-08-03 完成 Builder 链按插件组分簇（`a7d5925d`）**

- 原结构：9 个 `setup_*` 助手已拆出，`run()`（338-649，约 311 行）承载 setup 闭包（10 步）+ invoke_handler 大列表（192 条命令）。
- **拆分结果（等价重构，零行为变更）**：
  - **setup 闭包抽出为命名函数 `setup_app(app: &mut tauri::App) -> Result<(), Box<dyn Error>>`**：10 步逐字搬运，含 4 处桌面端 cfg 分支。
  - **invoke_handler 由单个 `generate_handler!` 拆为「单分发器 + 5 簇」**：
    - **背景**：tauri 2.11 `Builder::invoke_handler` 为**覆盖式**（`self.invoke_handler = Box::new(...)`，多次调用互相覆盖，无法链式累加）；`generate_handler!` 展开为按命令名 match、未命中返回 false 的零捕获闭包。
    - **`dispatch_ipc` 前缀路由分发器**：`invoke.message.command()` 借用（NLL 使分支 move invoke 前借用结束，热路径零额外分配）→ 按前缀路由：`sync_/recovery_/mdns_`→同步簇；`ocr_/mobile_ocr_`→OCR 簇；`llm_/guide_`→LLM 簇（含 embed_model 模块三个 llm_ 前缀命令）；`plugin_`→插件市场簇；其余→核心簇（兜底）。
    - **5 个簇函数 `register_{core,sync,ocr,llm,plugin}_commands() -> impl Fn(tauri::ipc::Invoke<tauri::Wry>) -> bool + Send + Sync + 'static`**：192 条命令按语义归簇（脚本 Counter 精确 diff **零缺失零多余**），`desktop_check_update` 的 cfg 属性保留。
  - **新增 `#[cfg(test)] test_dispatch_cluster_prefixes_consistent` 守卫**：断言每簇命令名匹配路由前缀、core 兜底不与任何簇前缀重叠、192 条全覆盖——防未来新增命令被放进错误簇导致静默失配（返回 false 等同未知命令）。
- **验证**：命令级等价性 diff 192/192 零差异 / cargo check（桌面 + aarch64-linux-android 移动端）0 错误 / clippy `--all-targets` 0 警告 / fmt 干净 / solo_soul 365 测试全绿（+1 守卫）/ workspace + CLI check 0 错误；Android 6 warning 为基线预存（HEAD 基线 7）/ code-reviewer GO（前缀-簇一致、move 语义等价、类型正确、setup 签名一致；采纳 to_string→NLL 借用微优化 + 守卫测试建议）。
- **前缀路由约定已写入 dispatch_ipc doc 注释**：新增命令必须放入对应簇，否则会被路由到错误簇而失配。

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

**③ `src/pages/settings/TemplateManagerPage.tsx`（810 行）——✅ 已于 2026-08-03 完成拆分（`2bdc5fdd`）**

- 原结构：单组件 747 行，状态密集（20+ useState/useMemo/useCallback），已复用 `DynamicGroupConfig`、`useConfirm`、模板 store；编辑器状态（20 项）与操作（字段增删改/动态组/废弃确认/命名校验）交错在页面中。
- **拆分结果（等价重构，零行为变更，数据与回调经编排层透传）**：
  - **`hooks/useTemplateEditor.ts`（新，371 行）**：编辑器全部状态与操作收敛（字段增删改/动态组×4/废弃字段确认/命名校验），废弃确认 `useConfirm` 随迁（渲染位置与原 `{confirmDialog}` 一致），返回含 5 个基础 setter；`removeProperty` 的废弃确认/直接删除/降级废弃三分支逐字保留。
  - **`TemplateListSection.tsx`（新，198 行）**：搜索 + FilterChipGroup + 卡片列表 + 三态空位（加载/无模板/无筛选命中），导出 `ListTemplate`/`PageLabel` 类型（规范出处）。
  - **`TemplateEditorModal.tsx`（新，100 行）**：Dialog + TemplateEditor 外壳，props 为 `editor: ReturnType<typeof useTemplateEditor>`（类型安全、零 prop 漂移），标题随 isNewTemplate 切换。
  - **`SampleGallerySection.tsx`（新，50 行）**：SampleTemplateGallery + SampleTemplateDetail，`selectedSample` 为内部 state；onUse 失败保持详情打开（编排层 toast 后 rethrow，section 吞掉）与原语义逐字等价。
  - **`TemplateManagerPage.tsx`（810→328 行）**：缩为编排层——store 选择器/启动 effect/allTemplates/resolvePageLabel/pageOptions/filteredTemplates/删除确认流/详情弹窗/示例 `handleUseSample`/AppShell actions 全部保留；`pageFilter` 变更的 `if (v)` 守卫原样保留。
- **验证**：tsc 0 错误 / eslint 0 警告 / 全量 vitest 55 文件 484 用例全绿（基线不变）/ prettier 规范化；逐行保留性 diff 0 行丢失（24 处差异全为 prop 改名/prettier 行合并/参数改名 sample，grep 逐条核实）；code-reviewer GO（① hook 与页面各自 select 同一 store 无行为差异 ② async fn 赋 void 回调可赋 ③ confirmDialog 渲染位置一致 ④ 详情弹窗 onEdit 闭包不变）。

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
   - 其次：✅ **P223-② storage.rs 表域拆分**（`005fbfdf` objects 域试点 + `ae030551` trash 域 + `ad244d7c` snapshots 域 + `22e1a20f` sync_meta 域 + `14eff424` sync_changes 域 + `89446aeb` sync_apply 域 + `508f9445` metadata 域，已完成 → 模式已确立，下一域 Profile 按此推进）；
   - 然后：✅ **P224-② SyncPage 四面板**（`8c74253c`）、✅ **P224-④ AboutPage 四面板**（`084cfdd0`）与 ✅ **P224-⑤ OcrPage 三面板**（`fd70cc77`，均 2026-08-03 完成）→ 编排层 + 数据经 hook/props 透传模式已确立；
   - 最后：✅ **P224-③ TemplateManagerPage（hook + 三面板）**（`2bdc5fdd`，2026-08-03 完成，状态密集型拆分节奏已确立）、✅ **P223-① host.rs 六簇分簇**（`0f0a37ff`，2026-08-03 完成，最大函数 923 行→调度器+6 簇）与 ✅ **P223-③ lib.rs 收尾**（`a7d5925d`，2026-08-03 完成，Builder 链按插件组分簇：setup_app + 单分发器 + 5 簇 + 前缀路由守卫测试）。
4. **产出约束**：每个拆分**单独 commit**（一项一提交），commit message 注明「纯移动/等价重构」；本报告 §3 归档表随拆分补充新行。

**当前建议**：不单独安排修复轮次；**P224-①②③④⑤、P223-① host.rs 六簇、P223-②（objects/trash/snapshots/sync_meta/sync_changes/sync_apply/metadata）与 P223-③ lib.rs 全部完成**（`bc395973`/`8c74253c`/`2bdc5fdd`/`084cfdd0`/`fd70cc77`/`0f0a37ff`/`005fbfdf`/`ae030551`/`ad244d7c`/`22e1a20f`/`14eff424`/`89446aeb`/`508f9445`/`a7d5925d`），剩余前端巨型组件清零、Rust 长函数全部收编；P223/P224 剩余仅 P223-② 未拆域（Profile），下次触碰相关文件时按上述预案顺带执行。

### 4.2 已声明残余窗口：R-3 / R-4①（✅ 均已闭环）

两处窄窗口此前为修复人已声明的工程取舍；R-3 随方案 B 阶段 3 关闭、R-4① 经方案 2 两阶段交换 + probe 判定关闭（详见各小节实施记录）。

#### 4.2.1 R-3：同步中断后等值组尾部 at-least-once 缺口（低）

- **窗口**：N-1/R-3 修复后，会话**中断**（断网/崩溃/退出）时已持久化水印停在等值组最大值而页游标随同落库（R-3 已修）；残余为——中断后、续传前出现**同毫秒新行且 id < 旧游标**时该行被永久跳过。
- **概率**：需「同毫秒」+「随机 UUID 序逆序」双条件，天文概率，仅影响回退等值组；中断时已存在的行均按 id 序投递完毕。
- **彻底解法（方案 B，已立项 2026-08-03）**：**本地写入统一写 HLC**——`new_local_hlc()`/`new_tombstone_hlc()`（sync_meta.rs:311/327，`wall = now.max(本节点最大HLC+1)` 严格递增）已存在，只需在各域写方法事务内调用 `set_record_hlc_tx`。回退路径退休后等值组概念消失（HLC 单调递增），R-3 窗口自然关闭。
  - **影响面（实测）**：objects 域（`save_object_tx`/`delete_object`/`restore_object`，墓碑用 `new_tombstone_hlc`）+ trash 域（`trash_and_soft_delete_batch`/`save_trash_item_tx`/`delete_trash_item`）+ metadata 域（`save_user_template_tx`/`delete_user_template`）+ profile 域（`save_profile_tx`/`delete_profile`）；snapshots 域不参与同步可不动。**commands/CLI 层零改动**（全部经 vault 层方法间接调用）。
  - **成本**：1–2 天。分三阶段：① objects 域验证模式；② trash/profile/user_template 域；③ 回退路径退休（sync_changes.rs:133/424 的 `record_hlc_or_fallback` fallback 分支与 SQL `h.wall_time_ms IS NULL` 分支简化）。风险点：`delete_object` 墓碑与 `trash_and_soft_delete_batch` 的 HLC 生成时序、25+ 既有同步测试适配（部分断言依赖回退行为）。
  - **备选方案 A（等值组尾部回扫）成本 4–6h（基础）/ 8–10h（含防重复投递设计），已否决**——回扫会把已投递行重复投递，需额外记录已投递 id 集合，收益低于成本；方案 B 是根治方向。
  - **当前状态**：**阶段 1/2/3 全部闭环**（2026-08-04）——阶段 3（存量回填迁移 v23 + 回退兜底保留）实施见下，R-3 窗口随本地写统一 HLC 整体关闭（仅保留「用户选择保守退休」的兜底安全网）。
  - **✅ 阶段 1 实施记录（2026-08-04）**：
    - **objects 域本地写统一 HLC**：`save_object`/`save_objects_batch`/`delete_object`/`restore_object` 在写事务外层生成 HLC（`new_local_hlc`/`new_tombstone_hlc`）并在事务内 `set_record_hlc_tx` 落库。**`save_object_tx` 保持不动**（sync_apply 远端应用路径复用、自写 HLC），故本地写 HLC 必须在外层生成。
    - **关键修复（normalize_sync_node_id）**：初版本地写 HLC 行 node 为 raw `local_node_id()`（生产 `node_<32hex>` 40 字符 / 测试 `"unknown"`），与 sync 层 session.rs 的 hex 规范化节点（`hex::encode(Hlc::parse_node_id_bytes(...))`）及水印落库格式**编码错配** → keyset 等值组判定 `node == 水印 node` 永不成立，同 wall 行经 strict `>` 反复通过、id 游标不推进 → **分页死循环**（sync crate `test_generate_delta_paginated_keyset_production_encoding` 实测挂起，修复后 0.03s 通过；生产 `get_or_create_sync_identity` 路径同款触发）。修复：`new_local_hlc`/`new_tombstone_hlc` 生成时经新增 `normalize_sync_node_id`（sync_meta.rs，与 session.rs 逐字节一致：32 字符按 hex 解码、其余取前 16 字节补零）规范化 node。
    - **测试适配**：3 个依赖「本地写无 HLC」回退语义的 keyset 测试适配方案 B（`test_paginated_keyset_fallback_false_positive_isolation`/`test_peer_watermark_cursor_resume_delivers_equal_hlc_tail`/`test_list_object_changes_since_watermark_pushdown`）；新增防回归 `test_local_write_hlc_node_normalized_production_format`（生产格式 node id 下节点规范化 + wall 严格递增 + keyset 分页终止）；`collect_paginated_object_ids` 加页数上限守卫（keyset 回归从「挂起」转「快速失败」，防 CI 挂死）。
    - **验证**：vault 124 测试全绿（+1）/ sync 47 全绿 / fmt 干净 / clippy 0 警告 / workspace + CLI check 0 错误 / code-reviewer GO。
    - **阶段边界缺口（已声明，阶段 2 已收编）**：trash 域 `trash_and_soft_delete_batch` 软删 objects 曾不带 HLC → 该路径继续走回退；阶段 2 后软删对象也落新 HLC，缺口关闭。
  - **✅ 阶段 2 实施记录（2026-08-04）**：
    - **trash 域**：`save_trash_item` 落 trash_items HLC；`trash_and_soft_delete_batch` 批内两组 HLC——trash 条目落 trash_items HLC + **软删对象落 objects 新 HLC**（先软删作用域块释放 stmt 借用再统一落 HLC）。关键语义：阶段 1 后对象已有 save 时 HLC，不更新则对端永远看不到 `is_deleted=1`（对象行保留，updated_at 回退路径不再生效）——本改动堵住该同步缺口。
    - **profile/user_template 域**：`save_profile`/`save_user_template` 落对应表 HLC。`delete_profile`/`delete_user_template` 不改（已通过 `record_tombstone` 落墓碑+HLC）。
    - **测试适配**：`test_trash_changes_since_honors_millisecond_deleted_at`（R-2）改经 `save_trash_item_tx` 直插（不落 HLC）保留回退 ms 解释覆盖；`test_paginated_trash_keyset_equal_deleted_at_completeness` 显式 `set_record_hlc` 等值组保留 keyset 边界覆盖；新增防回归 `test_local_write_hlc_stage2_domains`（四写路径落库 HLC + 节点规范化 + 软删对象 HLC 晚于 save + 软删对象以 `deleted:true` 出现在变更清单）。
    - **验证**：vault 125（+1）/ sync 47 全绿 / fmt 干净 / clippy 0 警告 / workspace + CLI check 0 错误 / code-reviewer GO。
    - **既有缺口记录（超范围，不扩大）**：`delete_trash_item`（trash 条目永久删除）与 objects 硬删不传播——trash 变更清单不合并墓碑且 `apply_trash_sync_record_tx` 不处理 deleted 记录，加 HLC 即成死代码；属既有同步缺口，单独立项评估。
  - **✅ 阶段 3 实施记录（2026-08-04，用户决策方案 B：回填迁移 + 保留兜底）**：
    - **迁移 v23 `migrate_v23`**（migration.rs，`CURRENT_SCHEMA_VERSION` 22→23）：为升级前创建、无 sync_hlc 行的存量行（objects/profiles/trash_items/user_templates）回填 HLC。**wall 语义按各表真实回退路径逐字节复刻**：objects 用 keyset SQL 同款 `julianday(updated_at)→ms`；trash_items 用 `deleted_at` 原值；profiles/user_templates 用 Rust `parse_time_ms`（chrono RFC3339→ms，解析失败/NULL→0）——Rust 层逐行计算（**julianday 浮点对部分时间戳差 1ms，不可混用**，测试实测抓出 1ms 差异后修正）。counter=0，node=规范化本地节点（读 `metadata` 表 base64 明文 `sync_node_id`，无则 `"unknown"`，经与 session.rs 逐字节一致的 `normalize_sync_node_id`）。LEFT JOIN + INSERT OR IGNORE 双保险，已有 HLC 行不动。
    - **兜底保留（保守退休核心）**：`record_hlc_or_fallback` 与 keyset SQL 的 `h.wall_time_ms IS NULL` 分支**不删除**——未来任何直写 SQL 路径产生的无 HLC 行仍可经回退同步（安全网）。
    - **去重**：`normalize_sync_node_id` 提升 `pub(crate)`（sync_meta.rs），迁移层 `parse_time_ms`/`normalize_sync_node_id` 均复用 `VaultStore` 关联函数（同一 crate 无循环依赖）。
    - **测试**：新增防回归 ×2——`test_migration_v23_backfills_hlc_for_legacy_rows`（四表回填 wall/counter/node 逐字节断言 + 已有 HLC 行不动 + 幂等不新增）+ `test_migration_v23_backfill_uses_stored_sync_node`（已配置节点时 node 取规范化本地节点而非 `"unknown"`）。
    - **验证**：vault 127（+2）/ sync 47 全绿 / fmt 干净 / clippy 0 警告 / workspace + CLI check 0 错误 / code-reviewer GO。
    - **语义注记（声明，非缺陷）**：从未同步过的新库（无 `sync_node_id`）回填 node 为 `normalize("unknown")`，后续本地写用真实生成节点——排序仍全序（游标 id 决胜），非正确性 bug，仅记录以免意外。
  - **✅ R-3 关闭收尾验证（2026-08-04，全库写路径扫描）**：
    - **`_tx` 变体调用方全量核对**：`save_object_tx`/`save_profile_tx`/`save_trash_item_tx`/`save_user_template_tx` 生产调用方仅 `sync_apply.rs` 远端应用路径（接收方自写 HLC，合法）+ `storage.rs:2282` 测试直插（R-2 回退 ms 语义覆盖，合法）。**无业务本地写走 `_tx`**。
    - **四表写路径全覆盖确认**：本地写全部经落 HLC 的公开方法——`save_object`/`save_objects_batch`/`delete_object`（软/硬删）/`restore_object`/`save_profile`/`save_trash_item`/`save_user_template`。专项核查：import（core `export_import.rs:472` `save_objects_batch` + src-tauri `import.rs` `save_object`/`save_user_template`）、backup restore（`backup.rs:248` `save_profile`）、模板播种（`template_service.rs:187/287` `save_user_template` + `migrate_contract_bindings` `save_object`）、CLI（profile/backup/history 全部经 vault 层方法，CLI 零直接 SQL 写）。
    - **`repair_restored_objects`（snapshots.rs:279）判定为合法例外**：open 时一次性迁移修复（REPAIR_FLAG 守卫，每 Vault 仅一次），发生在 `run_migrations`（v23 回填）**之后**（storage.rs:196-224 顺序）——被修对象已有 HLC，修后 `updated_at` 刷新但 HLC 不变。**正确性**：触发条件本身是「account_id='imported' 的隐形对象」，修复前从未被正确同步，归位后首次同步即投递，R-3 关闭不受影响。与 v23 回填同属「存量数据修复，不写 HLC」模式。
    - **`migrate_to_encrypted_format` 重加密路径（559-895）**：内容不变，正确不落 HLC（非新变更）。快照表（object_snapshots）不参与同步，无需 HLC。
    - **结论**：**方案 B 三阶段覆盖完整，无残留本地写不落 HLC 路径**；`delete_trash_item` 与 objects 硬删不传播为既有缺口（已登记，trash 清单不合并墓碑，加 HLC 即成死代码）。

#### 4.2.2 R-4①：reencrypt commit 后、config 写前进程崩溃（低）——✅ 已于 2026-08-04 彻底关闭（方案 2 + probe 判定）

- **窗口**（现状）：`reencrypt_all`（SQLite 单事务）提交成功、config.json 原子写入完成前进程崩溃 → 账户「数据已换新钥、config 仍记旧参数」不可用（下次解锁旧参数通过 verify 但数据 GCM 解密失败）。P135 原子写仅消除「config 写一半」，此窗口仍需 journal 类机制。
- **协同解法（与 P135 联动，已实施）**：见 §3.1 P135 归档——config 写入已全部接入 `safe_storage::write_atomic`（.tmp+rename），「写一半」→「要么旧、要么新」，崩溃后孤儿 tmp / 损坏主文件由 `read_config_with_recovery` → `recover_or_load` 恢复；风险从「账户不可用」降为「下次解锁重新升级」。
- **核心难点（评估结论）**：reencrypt 与 config 分属两个持久化域（SQLite 事务 commit vs 文件写入），跨域无法原子化。任何彻底方案都需两要素：**① 持久化的意图记录**（崩溃后仍可知「曾有一次未完成的 reencrypt」及其目标 config）；**② 崩溃阶段判定**（判断 reencrypt 事务是否已提交，决定「完成」还是「放弃」）。「两阶段 config 交换」若不解决要素②，仅靠 rename 原子交换并不能关闭窗口（rename 前后崩溃仍是同一错位态）。
- **方案对比（已评估）**：方案 1（reencrypt 侧 journal，DB marker 判定）与方案 2（两阶段 config 交换，pending 文件即意图记录）收敛于同一机制——意图记录 + 阶段判定。差异仅在载体与判定实现；方案 2 的 pending 文件本身即合法 `AccountConfig`、成功路径为一次原子交换，成本更低且不触碰已审计的 `reencrypt_all`。**用户 2026-08-04 决策：方案 2 + probe 判定**。
- **✅ 实施记录（2026-08-04，一项一提交）**：
  - **vault crate：`solosoul_vault::probe_data_key(db_path, key)` 只读自由函数**——独立连接（`OpenFlags::READ_ONLY` + `PRAGMA query_only`），**不走 `VaultStore::open`**（后者触发迁移/一次性回填的写副作用，用错误密钥探测会写坏数据）；依次 probe profiles/objects/trash_items/user_templates 第一行非空加密字段，解密成功 → true，全空 → true。
  - **vault_service：`recover_pending_reencrypt`**（`unlock`/`verify_password` 入口调用，常态零开销）——pending 存在时用密码分别派生 pending（新）与 active（旧）两钥 probe：新钥可解 → **promote**（pending 原子写为 active + 删 pending + **同步刷新生物识别/PIN 凭证**）；旧钥可解 → **discard**（删 pending，数据保持旧钥）；双败 → **保留 pending** 返回带提示的密码错误（UX 提示「interrupted key rotation pending」）。
  - **`change_password` / `unlock_with_kdf_upgrade`**：reencrypt 前 `write_config_pending`（原子），成功后删 pending；reencrypt 失败 → 删 pending 返回；config 写失败 → 回滚成功删 pending / **回滚失败保留 pending**（数据可能已换新钥，作为下次 promote 的恢复线索）。
  - **`unlock_with_session_key`**（生物识别/PIN）加 pending 守卫：pending 存在时拒绝并引导密码解锁（会话密钥无密码可派生，无法自行恢复）。
  - **回归测试 ×5**：promote（reencrypt 已提交→新钥解锁→pending 升为 active+删除）/ discard（未提交→旧钥解锁→pending 删除）/ 密码错误保留 pending（含正确密码重试恢复）/ `change_password` 成功无残留 / `unlock_with_session_key` 拒绝。
  - **验证**：core 162（+5）/ vault 127 全绿 / fmt 干净 / clippy 0 / workspace + CLI check 0 / code-reviewer GO（阻断项已修复：promote 后凭证同步、READ_ONLY probe、verify 文档契约更新、UX 提示）。

### 4.3 P209：LEGACY_XOR_KEY（已决策保留）

- **现状**：`LEGACY_XOR_KEY` 仅用于 `legacy_xor_decrypt` 一键解密 <2.0 旧版 XOR 凭证文件并原子迁移为 AES-256-GCM；`legacy.rs` 不可删（`FileBiometricStorage` 为 macOS/iOS 回退的活动存储后端）；当前 `save`/`update` 已只写新格式，**XOR 路径零写入面**。
- **威胁面**：攻击者需同时持有编译产物与 0600 权限的旧文件，且内容为会话密钥非主密钥。
- **决策（2026-08-02 用户确认）**：迁移窗口未关闭，保留 XOR 路径，接受已充分记录的低危风险。
- **关闭条件**：迁移窗口关闭（<2.0 凭证全部完成迁移）后删除 `legacy.rs` 的 XOR 三件套（`LEGACY_XOR_KEY`/`legacy_xor_decrypt`/`is_legacy_key_file`；`FileBiometricStorage` 是活动后端需保留）——建议在下个大版本发布后评估。**诊断日志已就位（`066fa785`）**：`BiometricManager::count_legacy_key_files()` + `setup_app` 步骤 1.5 启动扫描，`RUST_LOG=solo_soul=trace` 输出存量计数（0 即窗口关闭信号），含四态防回归测试。

### 4.4 P206 遗留观察：PDF embed 与 object-src CSP（✅ 已闭环，方案 A）

- **问题**：`AttachmentPreviewOverlay` 的 PDF `<embed src=data:>` 受 `object-src`（缺省继承 `default-src 'self'`）管辖，现行 CSP 下本就被拦截——桌面端 PDF 附件预览实际不可用（2026-08-03 核实为真实功能缺陷：桌面端 `AttachmentPreviewOverlay.tsx:223` 以 `<embed src=data:>` 渲染 PDF，移动端已正确走系统打开）。
- **决策（2026-08-03 用户选择方案 A）**：CSP 增加 `object-src data:`（`d446dc0e`），桌面端 PDF 应用内预览恢复。XSS 面评估：`src` 全部来自 `fs_read_file_as_data_url` 读取的本机附件文件，全库无 dangerouslySetInnerHTML/innerHTML、markdown 统一 SafeMarkdown 净化，风险被现有面压制。
- 备选方案 B（弃用 embed 改系统打开）已评估未采用；R-3/R-4①/P209 维持现状（见 §4.2/§4.3）。

### 4.5 #1 objects/trash 硬删（purge）不传播的同步缺口——✅ 已于 2026-08-04 闭环（三步同构，N-13）

- **现状（2026-08-04 R-3 收尾验证登记，评估确认）**：
  - **产生端**：`delete_object(id, false)`（`storage/objects.rs` 硬删分支）与 `delete_trash_item(id)`（`storage/trash.rs`，回收站 purge）只执行 `DELETE FROM …` + `set_record_hlc`，**不写 `sync_tombstones` 表**。
  - **变更清单端**：`list_object_changes_since_limited` / `list_trash_changes_since`（`storage/sync_changes.rs`）**不合并 `sync_tombstones`**——对端唯一可感知删除的机制（profiles `:153-154`、user_templates `:438-440` 均已有 `list_tombstones_since` 合并，objects/trash 两域缺失）。
  - **应用端**：对端 `apply_object|trash_sync_record_tx` 直接反序列化 data，**无 deleted 分支**（profiles/user_templates 已有）——墓碑（data=null）会反序列化失败。
- **后果**：A 端 purge → 行消失、HLC 推进但清单无墓碑 → **B 端永不收到删除，条目永久残留/数据回魂**，极端情况触发无意义删除冲突。
- **✅ 实施记录（2026-08-04，一项一提交，三步同构）**：
  - **步骤1 产生端（`a539935f`）**：`delete_object(id,false)` 硬删分支与 `delete_trash_item` 删行后补 `record_tombstone(table, id)`——`with_tx` 返回 affected，`affected>0` 才记墓碑（重复 purge/幂等清理不产生幽灵墓碑）；墓碑 HLC 由 `new_tombstone_hlc` 生成（wall 严格大于本节点既往值），对端 conflict 裁决时删除胜出。已知取舍：DELETE 提交与 record_tombstone 非原子（与 delete_profile 既有模式一致，R-4① 同款窄窗口已有 vault_service pending 兜底）。
  - **步骤2 清单端（`4caaa79c`）**：`list_object_changes_since_limited` / `list_trash_changes_since_limited` 返回前经新增私有 helper `merge_tombstones` 合并本表墓碑（deleted=true, data=null），按 (HLC, id) 全序排序后 `truncate(limit)`（两处逐字重复收敛，P223 去重惯例）。排序/截断正确性：墓碑 HLC 通常大于在册记录 HLC，页满截断只截页尾墓碑；watermark 推进到页内最大 HLC，下页按新 watermark 过滤墓碑仍可取到不丢失；保留 HLC 最小的 limit 条，被截断行 HLC 恒 >= 页内最大 HLC，keyset 下页正确续取。
  - **步骤3 应用端 + 回归测试（`9165b5c7`）**：`apply_object_sync_record_tx` / `apply_trash_sync_record_tx` 在 deserialize 前检查 `record.deleted && record.data.is_null()` → DELETE 本地行（不重记本地墓碑，远程 HLC 保持权威删除时间戳）。**软删对象（is_deleted=1、全量 data）deleted=true 但 data 非空，不会被误判为墓碑**（关键误分类防护）。回归测试×6：对象硬删产生墓碑 / 硬删不存在行无幽灵墓碑 / trash purge 产生墓碑 / 墓碑应用到对端删除本地行 / 分页合并墓碑（keyset 全收集）/ 软删对象应用不误判为墓碑。
  - **验证**：fmt 干净 / clippy 0 / solosoul-vault 133（+6）+ sync 47 全绿 / workspace + CLI check 0 / code-reviewer GO（采纳：merge_tombstones 去重、软删误判回归测试、非原子窗口注释）。
- **遗留评估点（✅ 已闭环 2026-08-04）**：墓碑生命周期——`sync_tombstones` 无清理策略，硬删累积可致表无限增长（profiles/user_templates 同受此限，本缺口未引入新问题）。**方案 C 已按三步实施并提交（详见下节「§4.5.1 墓碑生命周期清理策略」）**。

### 4.5.1 墓碑生命周期清理策略（✅ 已闭环，2026-08-04 方案 C 三步实施）

> **实施记录（一项一提交 ×3 + 报告归档）**：
> - `6428d375` **Step1 前置修复**：`delete_peer` 改为 with_tx 单事务内 DELETE `sync_peers` + DELETE `sync_watermarks` 联动（消除评估发现的「忘记设备水位残留永远保住墓碑」遗留坑）。
> - `2bdcffc1` **Step2 核心**：`cleanup_expired_tombstones()` 新方法——单事务内逐条墓碑判定：**水位老化**（该表存续 peer 水位 MIN ≥ 墓碑 HLC ⇔ `!hlc_after_watermark`，JOIN `sync_peers` 存活过滤双保险）或**时间兜底**（该表无任何水位行=纯单机/未配对，`created_at` 365 天老化，绝不越权）；与 `sync_hlc` 解耦不联动（重建覆盖/无害孤儿）。
> - `28904507` **Step3 触发 + 测试**：`send_paginated_deltas` 四表循环完成、peer 水位全部推进后、send Done 前调用（刚 ack 的墓碑立即可清，失败仅 warn 不阻断会话）；回归测试 ×7（水位落后保留/全部越过清除/任一落后保留/存续 peer 无水位行不阻断/纯单机 400 天旧墓碑清除/纯单机新墓碑保留/delete_peer 联动删水位）。
> - 本报告归档 commit（见 git 历史）。
> **验证**：fmt 干净 / clippy 0 / solosoul-vault 140（+7）/ solosoul-sync 47 全绿 / workspace + CLI check 0 / code-reviewer GO（含补测）。

**评估结论（2026-08-04 仅分析，后按方案 C 实施）**：

**问题**：`sync_tombstones` 每硬删一条记录即插入一行且**永不清除**，表无限增长。删除对象的正确性是同步协议核心（防止对端数据回魂），清理必须保证**不破坏任何 peer 的收敛性**。

**关键安全约束（评估结论）**：
- 墓碑 HLC 由 `new_tombstone_hlc` 生成（wall 严格大于本节点既往值），投递条件为 `hlc_after_watermark`（严格大于 peer 水位）。
- **安全删除条件 = 所有「曾同步过该表且仍受信任」的 peer 水位 ≥ 墓碑 HLC**。水位达到即证明该 peer 已收到墓碑并应用（协议按水位线性推进，无乱序）。
- **新 peer 不需要墓碑**：新配对 peer 从零水位全量同步时，对象行已不存在，发不发墓碑对端结果一致（都不含已删对象）——因此清理只需考虑 `sync_watermarks` 中**已存在水位行**的 peer。
- 遗留坑：`delete_peer`（sync_meta.rs:209）只删 `sync_peers` 行，**不联动删该 peer 的 `sync_watermarks`**——若清理逻辑直接扫描 watermarks，被忘记/删除设备的水位残留会永远「保住」其名下墓碑，导致清理失效。实施时须先补 delete_peer 联动（或清理条件 JOIN sync_peers 过滤存活 peer）。

**方案对比**：

| 方案 | 机制 | 优点 | 缺点 | 判定 |
|------|------|------|------|------|
| A 按水位老化 | 对每个墓碑，若所有存续 peer 的该表水位 ≥ 墓碑 HLC 则删 | 严格正确，永不漏删（线性推进保证收到即应用） | 需逐墓碑×逐 peer 比较（O(墓碑×peer)），且依赖水位行完整性 | 推荐主方案 |
| B 按时间老化 | `created_at` 早于 N 天（如 30/90 天）即删 | 实现极简 | **不安全**：离线 >N 天的 peer 回归后漏收墓碑→对端对象回魂（数据丢失），违反收敛性 | ❌ 不推荐独立使用 |
| C 混合 | 主：水位老化；辅：时间上限（如 365 天）仅对**无任何水位行**的墓碑生效（纯单机/未配对） | 兼顾正确性与单机清理 | 逻辑分支多，需防边界 | 推荐（A 为主 + C 兜底单机） |

**推荐方案（C 的实现要点）**：
1. **先修 `delete_peer` 联动删 `sync_watermarks`**（或清理时 JOIN `sync_peers` 存活过滤）；
2. **`cleanup_expired_tombstones()` 新方法**（VaultStore）：单事务内对四表墓碑逐条取 `MIN(watermark)`（仅存续 peer）比较；
3. **触发时机**：同步会话结束（send_paginated_deltas 完成后）+ `delete_trash_item`/`delete_object(硬删)` 时惰性触发（计数阈值如每 100 条墓碑触发一次，避免每次硬删全表扫）；
4. **边界**：墓碑与 `sync_hlc` 耦合——`record_tombstone` 同时写 `sync_hlc`（`set_record_hlc`），清理墓碑时 `sync_hlc` 对应行仍保留（被后续重建记录覆盖或成为无害孤儿，删除需谨慎，暂不联动）；
5. **回归测试**：多 peer 水位未达不清、达到即清、新 peer 不受影响、delete_peer 后残留水位不阻断清理。

**影响面确认**：profiles/user_templates 墓碑与 objects/trash 同机制同表，清理方法统一覆盖四表，一处实现全部受益。

---

## §5 结论与后续建议

1. **审计闭环状态**：80 项问题**全部闭环**并经两轮独立复核（70 项首轮 + 16 项二轮补验 + P133/P134/P135 三项用户决策处置）验证，测试用例较修复前净增 60+；**所有 N/R 项复核发现均已闭环**。
2. **遗留未完成/待跟进 4 类**（本报告 §4）：
   - **P223/P224**：长函数/巨型组件长期重构（§4.1 含逐文件分解预案与执行顺序）；不单独安排修复轮次，随功能迭代顺带执行。**P224-①②③④⑤ TrashDetailPanel/SyncPage/TemplateManagerPage/AboutPage/OcrPage（`bc395973`/`8c74253c`/`2bdc5fdd`/`084cfdd0`/`fd70cc77`）、P223-① host.rs 六簇（`0f0a37ff`）、P223-② objects/trash/snapshots/sync_meta/sync_changes/sync_apply/metadata/profile 八域（`005fbfdf`/`ae030551`/`ad244d7c`/`22e1a20f`/`14eff424`/`89446aeb`/`508f9445`/`cef5776c`）与 P223-③ lib.rs 收尾（`a7d5925d`）均已完成**，无剩余未拆域。
   - **R-3/R-4①**：已声明残余窗口——**均已于 2026-08-04 彻底关闭**（R-3 方案 B 三阶段统一 HLC + v23 回填，R-4① config.json.pending 两阶段交换 + probe 判定，见 §4.2）。
   - **P209**：legacy XOR 迁移窗口保留，建议下个大版本发布后评估关闭（见 §4.3）。
   - **P206**：PDF embed 与 object-src CSP 遗留观察——✅ 已于 2026-08-03 按方案 A 闭环（CSP 增加 `object-src data:`，`d446dc0e`），桌面端 PDF 附件预览恢复（见 §4.4）。
   - **收尾清扫（2026-08-04 已完成）**：P223-① 预案的 `check_rate(host, name)` 助手收敛 7 处 rate_limiter 重复检查（`57d10448`）；P138 附带的 `sync_discover` 历史遗留死命令降级为内部助手（`2b91e2c6`）。
   - **同步缺口 #1（✅ 已闭环 N-13）**：objects/trash 硬删（purge）不传播——三步同构修复（产生端墓碑 `a539935f` + 清单端合并 `4caaa79c` + 应用端识别 `9165b5c7`，6 回归测试），详见 §4.5。
2. **同步墓碑生命周期（新评估点）**：`sync_tombstones` 无清理策略，硬删累积可致表无限增长（profiles/user_templates 同受此限）。建议下轮按 watermark 老化清理单独立项。
3. **验证指针**：本报告 §3 归档表含全部修复 commit（`f1970c67` 起，N/R 项见 §3.2；P133=`6e74f691`、P134=`f75605ae`、P135=`b721270c`、N-10=`70e766ee`）；完整修复细节与两轮验证记录在 git 历史中可追溯。

## §6 测试基线参考（当前 HEAD）

- 前端：tsc 0 错误 / eslint 0 警告 / vitest 55 文件 484 用例全绿。
- Rust：`cargo fmt --check` 通过 / `cargo clippy --workspace --all-targets` 零警告 / `cargo test --workspace` 全绿（solo_soul 365、core 156 默认（`future-keychain` 开启时 +4）、crypto 34、plugin 56、sync 47、vault 133（#1 墓碑传播 +6））；`solosoul_cli` cargo check 0 错误。
- 工作区干净，当前分支 `main` 与 `origin/main` 同步。
