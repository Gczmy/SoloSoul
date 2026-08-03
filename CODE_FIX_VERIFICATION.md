# 修复验证报告（CODE_FIX_VERIFICATION）

> 生成时间：2026-08-02
> 验证对象：`CODE_ANALYSIS_REPORT.md` 各问题的修复 commit（`f1970c67` 起）
> 验证方式：9 组并行代码审查（diff + 修复后完整代码 + 编译/测试佐证），逐项对照原报告修复期望
> 说明：修复人仍在进行中，本文档为独立验证结论，**不修改原报告**

## 基线验证（当前 HEAD）

| 检查 | 结果 |
|------|------|
| `npx tsc --noEmit` | ✅ 0 错误 |
| `npm run lint` | ✅ 0 错误 0 警告（原 warning 已随 P220 消除） |
| `npm run test` | ✅ 54 文件 / 482 用例全部通过（较修复前 +52 用例） |
| `cargo fmt --check` / `cargo clippy --workspace --all-targets` | ✅ 零警告 |
| `cargo test --workspace` | ✅ 全部通过 |

## 总览

- **已验证 70 项**：✅ 修复正确 60 项 · ⚠️ 部分修复 9 项 · 🔺 修复引入新问题 1 项（N-1 已于 2026-08-03 修复闭环，见下方修复记录）
- **已提交未验证 2 项**：P227、P231（验证期间提交，`66e9c259`、`03336ffb`）
- **修复进行中（未提交）1 项**：P225（工作区 5 个 Rust 文件，行解密闭包/重复块收敛）
- **未修复/暂缓 7 项**：P133-P135（死模块删除，原报告注明破坏性操作暂缓）、P223/P224（长函数/巨型组件，原报告建议随迭代顺带）、P226（前端组件重复 3 对）、P228（2 处循环依赖）

## 🔺 必须优先处理：P110 修复引入同步永久停滞缺陷

**位置**：`crates/solosoul-vault/src/storage.rs:1613-1639` + `1729-1733`、`crates/solosoul-sync/src/delta.rs:68`、`session.rs:492-494`

**缺陷链条**：P110 把 `LIMIT/OFFSET` 下推到 SQL 层，但 P109 的回退行粗筛是"宁多勿漏"——同一秒内 ≤ 水印的已同步记录也会通过 SQL WHERE，设计上靠 **LIMIT 之后**的 Rust 精确过滤兜底：

1. 本地写入从不写 `sync_hlc` 行，几乎所有本地对象永久走回退路径（wall=updated_at, counter=0, node=local）。
2. **触发场景一（稳定复现）**：删除一个含 >100 对象的页面（`trash_and_soft_delete_batch` 给全部软删行同一个 `now`，storage.rs:3166-3173）→ 第 1 页同步 100 条、水印推进到 `(T,0,local)`；第 2 页 LIMIT 100 取回的全是平局记录，Rust 严格 `>` 判定**全部丢弃** → 空页 → `finished=true` → break。**剩余删除记录永远不会同步到对端**，之后每次会话都同样停滞。
3. **触发场景二**：一秒内批量创建/更新 >100 个对象（批量导入/模板迁移/开户播种），同样空页 break 永久停滞。
4. 旧实现先全量 Rust 过滤再内存 skip/take，不存在此问题——**是 P110 新引入的回归**。现有回归测试全部使用显式 `set_record_hlc` 的互异 HLC，未覆盖回退假阳性 + LIMIT 组合。

**修复方向**：回退行过滤做成 SQL 精确（WHERE 中用 `julianday(updated_at)` 推导 wall 做完整三元组比较）；或空页时按"SQL 原始行数 < limit"判定 finished；或 LIMIT 改在 Rust 过滤后施加。**建议修复并补回归测试后再发布。**

## N-1 修复记录（2026-08-03）

**修复方案**（三处配合，keyset 分页替代 OFFSET）：

1. **storage.rs `list_object_changes_since_limited` / `list_sync_changes_since_paginated`**：
   - 签名 `offset: usize` → `last_row_id: Option<&str>`（页游标 = 本页最后一条 id），`ORDER BY` 增加 `o.id` 末位决胜，构成 (有效 HLC, o.id) 全序。
   - WHERE 以 (三元组, o.id) > (水印, 游标) 推进：有 HLC 行按 (wall, counter, node) 三元组，无 HLC 回退行按 `(CAST(julianday(updated_at) ms), 0, local)` 三元组**精确过滤**（替代 P109 的整秒粗筛 + Rust 兜底）——假阳性不再占用 LIMIT 预算，空页即真结束，消除「空页但 finished=false、max_hlc=None、水印永不推进」死循环。
   - 等值组尾部（三元组 == 水印 且 id > 游标）允许通过，解决同 ms 批量行跨页边界被严格 `>` 永久跳过的问题。
   - 非分页 wrapper `list_object_changes_since` 传 `None` 退化为严格三元组 `>`，语义不变。Rust 最终裁决与 SQL 谓词逐字一致（`hlc_after_watermark || (等值水印 && is_some_and(|c| c < id))`）。
2. **session.rs `send_paginated_deltas` 节点编码对齐**（修复后经独立复核确认的必要补充）：
   - 存储层回退行节点必须与 peer watermark 落库格式一致（hex 编码的 16 字节节点）。原先传原始随机 UUID，经 `parse_node_id_bytes` + `watermark_to_vault` 的 hex 往返后与 UUID 字符串永不相等 → keyset 等值组分支在生产编码下永不触发（等 ms 回退行组依旧死循环或静默漏发）。
   - 修复：`let local_hlc_node = hex::encode(Hlc::parse_node_id_bytes(node_id));` 作为 local_node_id 传入——与落库水印节点逐字节一致，keyset 分支正式生效；对 32 位 hex 节点幂等、对 UUID 有确定规范化，对端水印比较保持对称一致。
3. **回归测试 ×3**：
   - storage.rs `test_paginated_keyset_fallback_equal_hlc_completeness`：7 个同 updated_at 回退行逐页无缺漏无重复、组内按 id 升序；
   - storage.rs `test_paginated_keyset_fallback_false_positive_isolation`：同秒假阳性排除、真阳性全投递、循环必然终止；
   - delta.rs `test_generate_delta_paginated_keyset_production_encoding`：完整生产路径（generate_delta_paginated → watermark_to_vault 落库 → get_peer_watermark 读回 → vault_to_watermark，UUID 节点规范化后 7 条全投递）。

**验证**：fmt 干净 / clippy 0 / solosoul-vault 117（+2）+ solosoul-sync 47（+1）全绿 / workspace + CLI check 0 错误。

**已声明残余限制**：会话**中断**（断网/崩溃/退出）时，已持久化水印停在等值组最大值而页游标丢失，重启以 NULL 游标重查会跳过三元组 == 水印的组尾行（at-least-once 缺口）。需同时满足「会话中断」+「在飞等 ms 组」才触发；修复前每次同步都丢/循环，属严格改善。后续可把页游标 id 并入 peer watermark 持久化彻底关闭。

## N-9 修复记录（2026-08-03）

**修复方案**：P205 残余——ipc.test.ts 中针对已删命令的陈旧 mock 测试。

1. 后端已确认：`encrypt_bytes`/`decrypt_bytes`/`derive_key` 三个命令在 P132/P205 中已从 backend 移除（`commands/crypto.rs` 整文件删除），allowlist 同步清理。
2. ipc.test.ts `describe('Crypto', ...)` 块整体删除（含 2 个用例：encrypt/decrypt 参数传递 + deriveKey 全参传递），保留注释说明删除原因，防止后人误以为这些命令仍存在。
3. `get_state` 测试保留——该命令仍存在（vault.rs:38）。

**验证**：tsc 0 错误 / eslint 0 警告 / ipc.test.ts 14 用例全绿。

## N-8 修复记录（2026-08-03）

**修复方案**：P129 残余——App/index.tsx 与 lib/notification.ts 两处绕过 helper 直写③。

1. `settingsStore.ts`：`syncPlaintextPref` 由模块私有改为 `export`（注释补充说明供跨模块 UI 偏好写入复用）。
2. `App/index.tsx` `finishOnboarding`：`invoke('ui_update_preference', { key: 'hasSeenOnboarding', value: 'true' })` → `void syncPlaintextPref('hasSeenOnboarding', 'true')`（失败仅记日志，localStorage 兜底已先行 setItem）。
3. `lib/notification.ts` `requestNotificationPermissionOnce`：`invoke('ui_update_preference', ...).catch(logger.warn)` → `void syncPlaintextPref('notificationPermissionRequested', 'true')`（helper 内部已记日志）。
4. 两处 `invoke` 其余用途保留（ui_get_preferences / vault_list_accounts / user_data_get_preferences / backup_list），未产生未使用导入。

**验证**：`npx tsc --noEmit` 0 错误 / eslint 3 文件 0 警告 / settingsStore.test.ts 19 用例全绿。

## N-7 修复记录（2026-08-03）

**修复方案**：P120/P122/P125 三个修复新增的 4 个文案 key 补入双语 locale。

1. `settings.json`（zh-CN + en-US 同步）新增：
   - `export_scope_load_failed`（P120，ExportImportPage 导出范围加载失败 toast）；
   - `trash_detail_load_failed`（P122，TrashPage 回收站详情加载失败 toast）；
   - `debug_log_exported` / `debug_log_export_failed`（P125，DebugLogPage 诊断包导出成功/失败 toast）。
2. 三处 defaultValue 中文兜底保留（双重保险）；英文 UI 不再回退中文。

**验证**：两个 locale JSON 解析通过 / `npx tsc --noEmit` 0 错误。

## N-6 修复记录（2026-08-03）

**修复方案**：`ocr_scan_mrz`（desktop）移入 `spawn_blocking`。

1. 原残余：`get_ocr_engine`（含首次 ONNX session 初始化，数百 ms）+ `engine.scan_mrz`（秒级推理）在 tokio worker 上同步执行——与 P113 修复前 `ocr_scan_image` 完全相同的问题，阻塞单线程 runtime 上所有并发命令（含网络下载/同步循环）。
2. 修复：引擎加载 + 锁 + `scan_mrz` 整体包进 `tokio::task::spawn_blocking(move || {...})`，join 错误映射为 `"OCR MRZ task join error: {e}"`，`??` 双层解包（join error + 业务错误）。文件路径检查、审计日志仍在 async 侧，与 `ocr_scan_image` 的 P113 模式逐字对齐。

**验证**：fmt 干净 / clippy 0 / solo_soul ocr 测试 22 全绿 / cargo check 通过。

## N-5 修复记录（2026-08-03）

**修复方案**：P104 残余的 tiny/medium 档 sha256 清单补全。

1. **清单扩至三档 12 文件**（ocr.rs `PINNED_MODEL_SHA256` 4→12 条）：
   - tiny/medium 哈希由下载官方 Hugging Face 仓库 `PaddlePaddle/PP-OCRv6_{tiny,medium}_{det,rec}_onnx` 的 `resolve/main/inference.onnx`/`inference.yml` 计算（8 文件，共约 145MB，尺寸与官方 content-length 逐字节一致）。
   - **权威性交叉验证**：同一官方仓库下载的 small 四文件哈希与代码中已钉死（并获随包资源回归守护）的 small 哈希**完全一致**——证明 HF 即 P104 假设的权威源，tiny/medium 哈希可信。
   - 清单注释记录来源修订号（2026-06-18：tiny_det `2ba1506c`、tiny_rec `2612ab37`、medium_det `61323801`、medium_rec `50c7eaca`）及两个既定取舍：官方 main 更新后合法下载会报校验失败（需同步更新清单）；重导出/重优化 ONNX 的自定义镜像会被拒绝。
2. **测试更新**：`test_pinned_manifest_covers_small_downloads` 更名 `test_pinned_manifest_covers_all_tiers_downloads`（三档 × 4 文件全覆盖断言）；`test_pinned_manifest_hashes_match_bundled_resources` 跳过非 small key（tiny/medium 不随包，无本地资源可比对）；64-hex 测试自动覆盖 12 条。

**验证**：fmt 干净 / clippy 0 / 3 个 pinned 测试全绿 / solo_soul 全测试无失败。

## N-4 修复记录（2026-08-03）

**修复方案**（登记门禁原生确认 + embedding 通道对齐，三处配合）：

1. **provider.rs `llm_save_provider` 登记门禁闭环**：
   - 原残余：P102 的「已登记」检查含**保存进 config 的任意 https URL**——XSS 可两步绕过：① `llm_save_provider` 登记恶意 URL（仅 scheme/host 校验），② `llm_send_message_stream` 借已登记地址外传会话数据。
   - 修复：命令新增 `app: tauri::AppHandle` 参数；保存前 `!is_registered_provider_url(config, base_url)`（非内置默认 ∪ 非已保存 config）时弹出**系统级原生确认对话框**（`app.dialog()` + `OkCancelCustom("确认登记"/"取消")` + oneshot 回调桥接），用户取消返回 Err「已取消 AI Provider 登记」——webview 内 XSS 无法程序化点击原生对话框，两步绕过被彻底堵死。已登记地址的再次保存（编辑内置/既有 provider）不弹窗，正常路径零打扰。
   - 评审补强：对话框等待加 120s 超时兜底（对话框异常未回调时命令不再永久挂起）。
2. **rag.rs embedding 通道 URL 门禁**：
   - 原残余：`get_embedding_source` 直接以 `active.base_url` 构造 Cloud source，`embed_text`/`embed_texts` 发送前无任何校验——配置被异常值污染后借 embedding 通道（指南/查询内容）外传。
   - 修复：新增 `pub(crate) validate_embedding_base_url(config, base_url)` 纯函数（`validate_llm_base_url` + `is_registered_provider_url` 双检），`get_embedding_source` 构造 Cloud source 前调用——三条入口（`llm_search_guide_chunks`/`llm_rebuild_guide_embeddings`/`llm_check_embedding_available`）全覆盖。原 `config.active_provider_id.ok_or` 改 `clone().ok_or` 避免部分移动（下方需借整份 config）。
3. **前端 `LlmConfigPage.tsx` `handleSaveProvider`**：原 `.catch` 吞错后仍无条件 `setProviders` + `onSuccess`（用户取消登记会误报「保存成功」）——改 try/catch：失败（含取消）仅 `logger.warn` + 非取消类失败 toast `llm_save_provider_failed`（zh/en 双语 key 已入 locale），本地列表仅成功后更新。

**验证**：fmt 干净 / clippy 0 / 新增 4 测试全绿（内置默认接受/已保存接受含尾斜杠/未登记拒绝/非法 scheme+userinfo 拒绝）/ cargo check 通过 / solo_soul 全测试通过 / tsc+eslint 干净 / locale JSON 校验通过。

## N-3 修复记录（2026-08-03）

**修复方案**：`llmStore.streamBuffer` 纳入 vault-locked 清理链。

1. **AppRoutes.tsx vault-locked 监听链**：在既有 `useOcrScanStore.getState().clearOnVaultLock()` 与 `searchCache.clear()` 之间新增 `useLlmStore.getState().reset()`。`reset()` 一次性完成：① 清空 `streamBuffer`（在飞 LLM 输出明文）与 `streamError`；② 取消进行中的 `llm-stream-chunk` 事件订阅（同步 `unlisten` + 待决 `unlistenPromise` 双路径清理，与 `stopStream` 一致）；③ 重置 `isStreaming`/`streamingConvId`。
2. 选择 `reset()` 而非新增 `clearOnVaultLock`：其语义恰为「全清 + 退订」，与锁定清理目标完全一致；重复实现只会复制 listener 清理逻辑。

**验证**：`npx tsc --noEmit` 0 错误 / `npx eslint src/App/AppRoutes.tsx` 0 警告 / `llmStore.test.ts` 10 用例全绿（reset 清空 streamBuffer 既有用例覆盖）。

## N-2 修复记录（2026-08-03）

**修复方案**（storage 事务化 + 调用方两阶段回滚，两处配合）：

1. **storage.rs `reencrypt_all` 事务化**：
   - 历史 bug：闭包内任一行解密/重加密失败返回 Err 时，函数仍**无条件 `tx.commit()`**——失败前已处理的行用新钥落库、失败行仍为旧钥的混态，改密/KDF 升级后账户部分数据永久不可解密。
   - 修复：闭包结果捕获为 `let result: Result<(), String> = (|| {...})();`，随后 `match result`——Ok 才 `tx.commit()`；Err 仅记日志并返回 Err（drop tx 自动回滚，整体保持旧密钥）。
   - 新增 `pub fn set_data_key(DataEncryptionKey)`：替换内存密钥（不改磁盘），供回滚时先读回再写回，与既有 roundtrip 测试内部手动替换行为一致。
   - 新增回归测试 `test_reencrypt_all_failure_rolls_back`：破坏一个对象行密文（GCM 认证失败）→ 断言返回 Err、事务整体回滚（profile 仍以旧钥解密、损坏行密文字节未被写入）。
2. **vault_service.rs 两阶段原子性（`change_password` + `unlock_with_kdf_upgrade`）**：
   - 两阶段风险：reencrypt 成功后 config 写入失败 → 账户「数据已换新钥、config 仍记旧参数」不可用。
   - 修复：两调用方在 reencrypt **之前**读取并解析旧 config（备份 + 校验）；config 写失败时调用新助手 `rollback_reencrypt_and_config`——恢复旧 config 内容 → `set_data_key(new)` → `reencrypt_all(new, old)` 重加密回旧钥 → `set_data_key(old)`，保持账户一致可用（当前会话未换钥，回滚后继续以旧钥工作），随后上抛失败原因。
   - **评审补强**：`change_password` 的 config 备份读取原置于 reencrypt 之后（读取失败会留下混态），已移至 reencrypt 之前与 `unlock_with_kdf_upgrade` 一致；回滚错误文案改为「已尝试自动回滚」避免过度承诺。

**验证**：fmt 干净 / clippy 0 / solosoul-vault 118（+1）+ solosoul-core 152 全绿 / workspace + CLI check 0 错误。

## 逐项判定表

### P0 安全（A 组）

| ID | 判定 | 说明 |
|----|------|------|
| P001 | ✅ | 指纹取自 Noise 握手认证值（noise.rs:171-175 `get_remote_static`），双角色握手后强制比对（session.rs:557-584），不覆盖已有绑定；旧无指纹 peer 走 TOFU 放行（注释明示的兼容性权衡：升级后首次同步先连先绑，存在理论抢占窗口）；mDNS 明文广播 node_id 为有意保留 |
| P002 | ✅ | 真 DPAPI（windows.rs:157-236），魔数检测+原子迁移旧凭证，升级后生物识别不失效；残余为 DPAPI 固有限制（同用户上下文进程可自行解密，非 TPM 绑定） |
| P003 | ⚠️ | 主逻辑正确：debug/release 判定正确（kdf.rs:47-52），旧账户按落库参数验证再升级不会锁死，CLI 兼容。**但放大两处原子性隐患**：① `reencrypt_all` 失败仍无条件 commit（storage.rs:952-953，6 月引入的历史 bug，现在每次旧账户解锁自动触发）；② 重加密与 config 写入两阶段非原子，中间崩溃可致永久"Invalid password"。建议优先修 ①（一行级改动） |

### P0 前端（B 组）

| ID | 判定 | 说明 |
|----|------|------|
| P004 | ✅ | 清理链接入 trashStore.clearOnVaultLock（AppRoutes.tsx:484），真实清空，无竞态 |
| P005 | ✅ | searchCache.clear() 挂接（:487），Map 立即清空；其他模块级缓存排查无敏感明文 |
| P006 | ✅ | toast + logger + 弹窗保持打开，i18n key 双语齐全，正常路径无误伤 |
| P007 | ✅ | 三处保存失败统一 toast + 日志，i18n 双语 + defaultValue 兜底 |

**B 组新发现（潜在问题 N-3）**：`llmStore.streamBuffer`（llmStore.ts:16）未纳入锁定清理链——流式进行中触发自动锁定，解密 LLM 输出明文残留全局 Zustand。属 P005 同类遗漏。

### 安全中危（C 组）

| ID | 判定 | 说明 |
|----|------|------|
| P101 | ✅ | 188 条显式 allowlist，与前端全部 invoke 命令比对零缺失（6 个缺失均为测试 mock/注释） |
| P102 | ⚠️ | URL 校验与登记检查存在，但**注册门禁可绕过**：`llm_save_provider` 接受任意 https URL 且在 allowlist 中，XSS 可先登记恶意 provider 再发送；embedding 通道（rag.rs:96-100）发送时无校验。若要闭环需对登记加用户确认或限定内置白名单 |
| P103 | ✅ | 信任检查前仅回错误帧、peer 落库延迟，配对事件链完整；46 测试通过。小瑕疵：syncStore.ts:321 注释与新行为不符 |
| P104 | ⚠️ | URL 校验+重定向白名单+流式双限+原子 rename 齐全，**但 sha256 清单仅 small 档 4 文件，tiny/medium 无哈希校验**（commit 已自认，待官方哈希） |
| P105 | ✅ | SOLC v2 头部纳入 AAD，v1 自动探测回退，旧导出包/附件可解密，34 测试通过 |
| P106 | ✅ | 头部一致性校验正确；备注：SOLO v3 blob 格式当前无生产调用方（理论面加固） |
| P107 | ✅ | 基目录收窄正确，核心流程无破坏；两处轻微降级：目录外选附件 sizeBytes=0、遗留外部 vaultPath 预览报错（有优雅错误 UI） |
| P108 | ✅ | copy/stat 仅 $APPCACHE/$TEMP，所有调用点核实无破坏 |

### Rust 性能（D 组）

| ID | 判定 | 说明 |
|----|------|------|
| P109 | ✅ | 水印下推语义逐字节等价，边界不漏不重；隐含假设：updated_at 恒为 UTC `+00:00` 格式 |
| P110 | 🔺 | **见上方专节——同步永久停滞缺陷** |
| P111 | ✅ | 7 处切换调用方全部核实无误用，metadata 字段覆盖全部消费场景 |
| P112 | ✅ | 附件树输出与旧实现等价，边界处理正确且更健壮 |
| P113 | ⚠️ | 本项正确（引擎缓存跨线程安全），**但 `ocr_scan_mrz`（ocr.rs:412-416）遗漏未移入 spawn_blocking**，阻塞问题在该路径完整保留 |
| P114 | ✅ | VaultStore 全 Mutex 字段线程安全，错误回传正确，auto_sync 触发时机正确 |
| P115 | ✅ | 事务语义正确，单条失败语义与旧实现逐字节等价，无部分提交风险 |

### 前端性能（E 组）

| ID | 判定 | 说明 |
|----|------|------|
| P116 | ✅ | memo props 稳定，复制功能正常；小瑕疵：rehypePlugins 内联数组致复制点击时全量重解析（严格优于修复前，可改模块常量） |
| P117 | ✅ | 字段无漏订阅，effect 逻辑逐字等价，生产代码整店订阅清零 |
| P118 | ✅ | 7 个 useCallback deps 正确，卡片行为逐项等价 |
| P119 | ✅ | 过滤→分页顺序正确，批量操作仍作用于全部 filtered；注意：加载更多后 items 变化游标回缩（commit 声明的有意设计） |

### 错误处理/架构（F 组）

| ID | 判定 | 说明 |
|----|------|------|
| P120 | ⚠️ | toast 已有，但期望的错误态/重试 UI 未实现（失败仍渲染空树）；locale key 缺失 |
| P121 | ✅ | 失败计数 toast + 日志，i18n 双语齐全 |
| P122 | ⚠️ | toast 已有，但错误占位+重试未实现（与"无数据"同态）；locale key 缺失 |
| P123 | ✅ | verify 失败与 invoke 异常正确区分，key 双语存在 |
| P124 | ✅ | 异常细节保留并上抛；备注：靠错误消息正则匹配，轻微脆弱 |
| P125 | ✅ | 成功/失败均有 toast，取消路径不误报；locale key 缺失（defaultValue 中文兜底） |
| P126 | ✅ | 仅 not-found 返回 null，调用方已适配，有防回归测试 |
| P127 | ✅ | await + catch 补齐 |
| P128 | ✅ | 第 5 写入点消除，ST_UI_PREFS 唯一写入点收敛 |
| P129 | ⚠️ | helper 中央化正确（vault 写成功后才同步②③，失败回滚），但 `App/index.tsx:91`、`lib/notification.ts:50` 仍绕过 helper 直写③，「唯一写入点」非代码级强制（两 key 不在四副本矩阵内，实际无漂移风险） |
| P130 | ✅ | confirmWithPause try/finally，取消/异常路径均正确 resume；无残留裸调 |
| P131 | ✅ | invokeClient 错误原样透传不改变调用方预期，无漏迁；备注：统一日志仅 dev 生效、`requireUnlocked` 守卫暂无调用方（预留） |

**F 组共性问题**：P120/P122/P125 三个新增文案 key 未入 zh-CN/en-US locale 文件，英文 UI 显示中文 defaultValue。

### 死代码/去重（G 组）

| ID | 判定 | 说明 |
|----|------|------|
| P132 | ✅ | 8 命令前端/CLI 零残留，注册与 allowlist 同步清理 |
| P136 | ✅ | 自我纠错正确：CLI 依赖方法全部恢复，最终仅删 15 个零调用方法，与原文件对比恢复完整 |
| P137 | ✅ | serde 字段逐字段一致（camelCase/default/skip 全保留），config 反序列化兼容 |
| P138 | ✅ | 11 对合并正确，真有差异的 2 对保留；附带发现：`sync_discover` 系历史遗留未注册死命令（非本次引入） |
| P139 | ✅ | shared.rs 与三处原函数逐字节一致，语义等价 |
| P140 | ✅ | 两页行为逐分支等价，10 个防回归测试；边角：切换语言后错误文案语言滞后（仅文案） |
| P141 | ✅ | 防抖/缓存/过滤/渲染逐分支一致 |
| P142 | ✅ | 删除的两处与 hook 实现逐字一致，placement 参数化正确 |

### 低危/杂项（H 组）

| ID | 判定 | 说明 |
|----|------|------|
| P201 | ✅ | 三重防线齐全 |
| P202 | ✅ | 旧包 balanced 回退兼容有真实测试，新包声明参数优先；已知前向不兼容（新包旧应用打不开）已声明 |
| P203 | ✅ | 调试日志删除/脱敏 |
| P204 | ✅ | Zeroizing 包裹，语义不变 |
| P205 | ⚠️ | 后端命令面清除彻底零调用，**但 `ipc.test.ts:116-145` 残留 3 个针对已删命令的 mock 测试**，应清理 |
| P206 | ✅ | frame-src data: 移除，零 iframe 确认 |
| P207 | ⚠️ | 校验逻辑正确（硬拒+7 个真实签名测试），**但编译期公钥为 None 且无任何环境注入，默认构建下防护未激活（warn-only）**，待 `EMBED_REGISTRY_PUBKEY_B64` 填入才闭环 |
| P208 | ✅ | stdio 黑洞，无消费方回归 |
| P209 | ✅ | 纯文档决策记录 |
| P210 | ✅ | 递归匹配为旧预筛超集不漏放，数字/布尔/嵌套覆盖；理论 Unicode 边角可忽略 |
| P211 | ✅ | 删除语义等价，失败语义从"部分成功"变"整体回滚"属改进 |
| P212 | ✅ | SkipExisting/Overwrite 语义等价，事务正确；两处已声明边界（同包重复 id 末条胜、损坏行跳过） |
| P213 | ✅ | SQL 常量逐字等价，with_tx 回滚正确 |
| P214 | ✅ | public 筛选输出与旧全量解密筛选等价，有防泄漏测试 |
| P215 | ✅ | 环形截断 UI 完整，选择器覆盖一致 |
| P216 | ✅ | 折叠/卸载双路径持久化，滚动恢复功能正常 |
| P217 | ✅ | 重命名等价且优于旧实现（防双提交），8 个防回归测试 |
| P218 | ✅ | memo/useMemo/分页正确 |
| P219 | ✅ | 零残留，测试同步删除 |
| P220 | ✅ | 基线 lint warning 已消除 |
| P221 | ✅ | 抽查 4 项均零调用 |
| P222 | ✅ | 25 处降级与消费关系一致 |

### 未提交改动审查（I 组）

验证期间 P229/P230 已分别提交（`5f0967f6`、`c67f71c4`），工作区转为干净后又出现 P225 的在途改动。

| ID | 判定 | 说明 |
|----|------|------|
| P229 | ✅ | `isSafeExternalUrl` 显式白名单（GuideRenderer.tsx:77-87），javascript:/data:/vbscript:/协议相对/控制符变体全覆盖，锚点/相对路径无误伤，4 组 7 断言测试 |
| P230 | ✅ | `clearOnVaultLock` 清空全部结果数据保留 UI 偏好（ocrScanStore.ts:155-162），清理链注册位置/风格与 P004/P005 一致，在途扫描无复活窗口 |

## 潜在问题清单（新发现，供修复人跟进）

| 编号 | 严重度 | 位置 | 问题 |
|------|--------|------|------|
| N-1 | ✅ 已修复 | storage.rs:1613-1639 + delta.rs:68 | P110 引入的同步永久停滞已闭环（keyset 分页 + 回退行 SQL 精确过滤 + 会话层节点编码对齐，2026-08-03 提交，见下方修复记录）；残余：会话中断时等值组尾部 at-least-once 缺口（已声明） |
| N-2 | ✅ 已修复 | storage.rs:952-953 + vault_service.rs | `reencrypt_all` 无条件 commit 与 reencrypt→config 两阶段非原子已闭环（事务化 reencrypt + config 前置备份 + 写失败自动回滚，2026-08-03 提交，见下方修复记录） |
| N-3 | ✅ 已修复 | stores/llmStore.ts:16 + AppRoutes.tsx | streamBuffer 未纳入 vault-locked 清理链已闭环（`useLlmStore.getState().reset()` 接入清理链：清空 streamBuffer/streamError 并取消 llm-stream-chunk 订阅，2026-08-03 提交，见下方修复记录） |
| N-4 | ✅ 已修复 | commands/llm/provider.rs:62 + rag.rs | P102 残余已闭环：① `llm_save_provider` 对未登记新 URL 强制**原生确认对话框**（XSS 无法程序化点击，杜绝两步绕过，2026-08-03 提交，见下方修复记录）；② embedding 通道发送前强制已登记地址校验 |
| N-5 | ✅ 已修复 | commands/ocr.rs:800-813 | P104 残余已闭环：sha256 清单扩至三档共 12 文件（tiny/medium 哈希取自官方 HF 仓库并经 small 交叉验证，2026-08-03 提交，见下方修复记录） |
| N-6 | ✅ 已修复 | commands/ocr.rs:412-416 | P113 残余已闭环：`ocr_scan_mrz` 的引擎加载+推理整体移入 `spawn_blocking`（与 P113 的 `ocr_scan_image` 同一模式，2026-08-03 提交，见下方修复记录） |
| N-7 | ✅ 已修复 | ExportImportPage/TrashPage/DebugLogPage | P120/P122/P125 新增文案 key 已入 zh-CN/en-US locale（2026-08-03 提交，见下方修复记录） |
| N-8 | ✅ 已修复 | App/index.tsx:91、lib/notification.ts:50 | P129 残余已闭环：两处直写③收敛到导出的 `syncPlaintextPref`（2026-08-03 提交，见下方修复记录） |
| N-9 | ✅ 已修复 | src/lib/ipc.test.ts:116-145 | P205 残余已闭环：Crypto 块整体移除（2026-08-03 提交，见下方修复记录） |
| N-10 | 低 | commands/embed_model.rs:18 | P207 残余：minisign 公钥未注入，默认构建防护未激活 |
| N-11 | 低 | ExportImportPage.tsx:152-161、TrashPage.tsx:243-251 | P120/P122 期望的错误态/重试 UI 未实现，失败与"无数据"同态 |

## 结论与建议

1. **修复质量整体很高**：60/70 项完全正确，去重类（G 组 8 项）与性能类（E 组）全部 ✅，多数修复带防回归测试；测试用例较修复前净增 52 个。
2. **N-1/N-2/N-3/N-4 已于 2026-08-03 修复并提交**（见上方修复记录）：N-1 keyset 分页替代 OFFSET、回退行 SQL 精确过滤、会话层节点编码对齐（残余的「会话中断时等值组尾部跳过」缺口已声明，建议后续把页游标 id 并入 peer watermark 持久化彻底关闭）；N-2 `reencrypt_all` 事务化全有或全无 + config 前置备份 + 写失败自动回滚（评审补强：`change_password` 的 config 备份读取移至 reencrypt 之前）；N-3 llmStore.streamBuffer 接入 vault-locked 清理链（清明文 + 退订 llm-stream-chunk）；N-4 provider 登记原生确认对话框（XSS 无法点击，堵死两步绕过）+ embedding 通道发送前强制已登记校验。
3. **⚠️ 项的残余差距均已被 commit 声明或属低危**：N-6（中危）已修复，N-7 至 N-11 为小项。
6. **N-5/N-6/N-7/N-8/N-9 已于 2026-08-03 修复并提交**：N-5 sha256 清单补全 tiny/medium 档（官方 HF 仓库哈希 + small 交叉验证）；N-6 `ocr_scan_mrz` 移入 spawn_blocking（P113 残余闭环）；N-7 P120/P122/P125 新增文案 key 补入 zh-CN/en-US locale；N-8 App/index.tsx 与 notification.ts 两处直写③收敛到导出的 `syncPlaintextPref`（P129 唯一写入点代码级强制）；N-9 ipc.test.ts 中针对已删命令（encrypt_bytes/decrypt_bytes/derive_key）的陈旧 mock 测试整体移除。
4. **暂缓项决策待用户确认**：P133-P135（死模块删除）、P223/P224（结构性拆分）建议维持暂缓；P226/P228 可排入下一轮；P225 修复人正在进行中。
5. P227/P231 提交于验证之后，未在本次审查范围内，建议下一轮补验。
