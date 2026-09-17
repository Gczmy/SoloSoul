# 代码分析修复报告

> 最后更新：2026-09-17（轮次 6：分析和基线检查完成，P037 修复及报告获授权提交推送）
> 修复轮次：6（前端、Rust 核心、CLI、平台桥接及发布脚本）
> 上轮：4 —— 云同步设置页渲染崩溃修复：CSS module 类名字符串误展开为 style 索引属性
> （`style={{ ...styles.input }}`），WebKit 抛「Cannot set indexed properties on this object」；
> 隐患自 Phase 2 创建页面即存在，非 P007 拆分引入。新增页面级渲染冒烟测试锁定回归。
> 历史轮次：1 初始分析 → 2 逐项修复（25/30）→ 3 外部核验补修（P005/P008/P011/P021② 等 5 项）
> 当前分支：`main`

## 历史轮次说明（2026-08-24）

- 按用户要求：**重新全量分析生成新报告，不延续旧报告，生成后不进行修复**。
- 分析范围：`tauri/`（Rust `src-tauri/src` + `crates/`，前端 `src/`），跳过 `node_modules/`、`target/`、`dist/`、`.vite/`、`gen/` 等生成目录。
- Git 状态：仅 2 个未跟踪的 `bugreport-*.zip` 文件（位于 `tauri/`），无代码改动，未做任何提交。

## 历史基线检查结果（2026-08-24，`npm run check-all`）

| 检查项 | 结果 |
|--------|------|
| TypeScript `tsc --noEmit` | ✅ 通过 |
| Rust `cargo fmt --check` | ✅ 通过 |
| Rust `cargo clippy -- -D warnings` | ✅ 通过 |
| Rust `cargo test` | ✅ 通过 |
| ESLint | ✅ 通过 |
| Vitest（108 文件 / 928 用例） | ✅ 通过 |
| `check-markdown-chunk-boundary.mjs` | ✅ 通过 |
| `check_acl_consistency.py` | ❌ **失败**（8 个 cloud_sync 命令未登记 ACL；`ui_get_preferences` 白名单遗留）→ P001/P002 |
| `check_pref_keys_sync.py` | ⏭ 未执行（前一步失败后中断） |

## 问题清单（按优先级 P0 > P1 > P2）

| ID   | 优先级 | 类别 | 文件位置 | 描述 | 状态 |
|------|--------|------|----------|------|------|
| P037 | P1 | 安全/发布 | `../scripts/build_macos_release.sh`、`../scripts/sign_artifacts.sh` | 发布脚本的 shell xtrace 输出签名私钥，且 signer 的命令行参数携带完整私钥 | `[x]` 已修复，与本轮报告一同交付 |
| P038 | P1 | 安全/数据丢失 | `crates/solosoul-plugin/src/store.rs:17` | 插件 ID 校验允许 `.`/`..`，卸载时递归删除越过插件目录边界 | `[ ]` 待修复 |
| P039 | P1 | 安全/会话隔离 | `src/stores/`、`src/App/AppRoutes.tsx` | 锁定后晚到响应回填解密缓存，插件运行结果未清除且可跨账户展示 | `[ ]` 待修复 |
| P040 | P1 | 安全/存储 | `crates/solosoul-vault/src/storage/sync_apply.rs:142` | 同步冲突的双方完整数据明文写入普通 SQLite，绕过 Vault 应用层加密 | `[ ]` 待修复 |
| P041 | P1 | 数据可用性 | `crates/solosoul-vault/src/storage/reencrypt.rs` | 改密/KDF 升级漏重加密聊天表，切换密钥后历史对话不能解密 | `[ ]` 待修复 |
| P042 | P1 | 安全/授权 | `crates/solosoul-plugin/src/field.rs:615` | 插件 list_objects 直接返回全部属性，未按字段声明与授权过滤 | `[ ]` 待修复 |
| P043 | P1 | 安全/数据丢失 | `crates/solosoul-core/src/vault_service/account.rs` | 恢复账户 ID 未校验，`.` 等值可令失败恢复的清理删除 Vault 根内数据 | `[ ]` 待修复 |
| P044 | P1 | 安全/展示 | `src/components/trash/TrashDetailSections.tsx`、`TrashSnapshotView.tsx` | 回收站字段及历史快照只展示敏感度标签，受保护字段默认明文展示 | `[ ]` 待修复 |
| P048 | P1 | 安全/插件生命周期 | `crates/solosoul-plugin/src/field.rs:91`、`sandbox.rs:76` | 插件缓存命中不检查锁定状态，执行入口未使用会话过期信息 | `[ ]` 待修复 |
| P001 | P1 | 规范/CI | `src-tauri/permissions/solo-soul/default.toml` | 8 个 cloud_sync 命令已注册 handler 且前端在调，但未登记 ACL 白名单；`check_acl_consistency.py` exit 1，CI（pr_check.yml）必红，运行时 ACL 拒绝 | `[x]` 已修复 |
| P002 | P1 | 架构 | `src-tauri/src/lib.rs:46-193`（定义于 `commands/settings.rs:166-181`） | `ui_get_preferences` 有 `#[tauri::command]` 定义且进了 ACL，但从未注册进 `generate_handler!`，前端三处调用永远失败并被静默吞掉 | `[x]` 已修复 |
| P003 | P1 | 漏洞 | `crates/solosoul-core/src/cloud_sync/webdav.rs:48-65` | WebDAV 连接器允许 `http://` + Basic 认证，账号密码明文传输；与 LLM/OCR 的「非回环强制 https」策略不一致，错误文案与行为自相矛盾 | `[x]` 已修复 |
| P004 | P1 | 安全/规范 | `src/lib/searchShared.tsx:175-183` + `crates/solosoul-core/src/search_filter.rs:11` | 搜索结果中 internal 级字段命中值明文渲染，无 `useRevealState`/`maskValue`，违反 P036 掩码统一约定 | `[x]` 已修复 |
| P005 | P1 | 安全 | `src/components/layout/SearchPopover.tsx:49-63` | 最近搜索词明文持久化 localStorage（`solosoul_recent_searches`），不按账户隔离，Vault 锁定/退出时不清除 | `[x]` 已修复（核验补修：调用点漏传 accountId 致功能失效，已恢复） |
| P006 | P1 | 性能/事务 | `crates/solosoul-vault/src/storage/snapshots.rs:547-584` | `copy_snapshots` 循环逐条 INSERT 无事务包裹（中途失败留半成品），且同钥 decrypt→encrypt 纯浪费，应直接复制密文 | `[x]` 已修复 |
| P007 | P1 | 可维护性 | `src/pages/settings/CloudSyncPage.tsx:51-632` | 单组件约 535 行非注释代码，承载 WebDAV 配置/保留策略/连接器/入站列表全部逻辑 | `[x]` 已修复 |
| P008 | P1 | 可维护性 | `src/components/forms/DatePicker.tsx:168-609` | 主组件约 385 行，段落解析+键盘处理+滚轮渲染混杂 | `[x]` 已修复（核验补修：Calendar 新增的 common:hour/minute 双语键已入库） |
| P009 | P1 | 可维护性 | `src/pages/settings/useExportImportPage.tsx:33-454` | 导出/导入 hook 状态机约 371 行 | `[x]` 已修复 |
| P010 | P1 | 可维护性 | `src/pages/settings/VaultDirectorySection.tsx:27-423` | 单组件约 369 行 | `[x]` 已修复（2026-09-10 核实既有拆分，补正状态） |
| P031 | P1 | 规范/构建 | `crates/solosoul-core/src/export_import.rs:209` | 权限 helper 的参数仅在 Unix 分支使用，Windows Clippy 在 `-D warnings` 下报 unused variable 并中断 | `[x]` 已修复 |
| P032 | P1 | 生命周期 | `src/pages/settings/useVaultDirectory.ts` 等 7 处模块 | 异步订阅晚于卸载时监听器泄漏；目录进度在 StrictMode 重建后不消失，旧计时器还会隐藏新操作 | `[x]` 已修复 |
| P033 | P1 | 规范/CI | `../solosoul_cli/src/commands/security.rs:5` | 生物识别导入顺序不符合 rustfmt，CLI 格式检查失败 | `[x]` 已修复 |
| P034 | P1 | 规范/构建 | `package.json`、`scripts/check_acl_consistency.py` | check-all 硬编码 python3，本机只有 python；ACL 中文输出在 cp1252 下异常 | `[x]` 已修复 |
| P035 | P2 | 测试稳定性 | `src/components/attachment/PhotoAlbumOverlay.test.tsx` | 懒加载断言允许等待 8 秒，但测试仍在默认 5 秒被终止，异步操作干扰下一用例 | `[x]` 已修复 |
| P036 | P2 | 规范/测试 | `crates/solosoul-vault/tests/p025_baseline.rs:115`、`crates/solosoul-core/tests/cloud_sync_webdav_e2e.rs:332` | 测试存在未用循环变量和局部导入，扩大 Clippy 覆盖到测试时会失败 | `[x]` 已修复 |
| P011 | P2 | 安全 | `crates/solosoul-core/src/export_import.rs:221-237,1218-1242,2214` | 导出/导入附件临时明文落共享 temp 目录（可预测目录名、未设 0700/0600）；同仓库其他路径均已收紧权限，此处是离群点 | `[x]` 已修复（核验补修：残留测试改按前缀扫描恢复效力；一次性空目录用后即删） |
| P012 | P2 | 安全（加固） | `crates/solosoul-sync/src/recovery.rs:269-279,184` | Recovery 主机指纹校验可选（手动输入路径无 MITM 防线），且主机端接受裸 PIN 认证；已有限流/一次性 nonce 缓解，建议加固 | `[ ]` 延期（沿用既有处置决定） |
| P013 | P2 | 性能/事务 | `crates/solosoul-vault/src/storage/snapshots.rs:369-414` | `repair_invisible_objects` 循环内逐行 query_row + UPDATE 无事务（有一次性标记兜底，仅跑一次，故 P2） | `[x]` 已修复 |
| P014 | P2 | 性能 | `crates/solosoul-vault/src/storage/objects.rs:448` | `save_object_tx` 无条件克隆整棵 properties JSON，即使无需注入 `__templateName`；可加 `template_id.is_some()` 惰性克隆 | `[x]` 已修复 |
| P015 | P2 | 性能 | `src-tauri/src/commands/export_import/import.rs:130-135` | `import_decrypt_preview` 整包读入内存（上限 100MB，峰值约 3×100MB）；主导入路径已流式化，预览路径遗留 | `[x]` 已修复 |
| P016 | P2 | 重复代码 | `crates/solosoul-vault/src/storage/reencrypt.rs:54,106,126` | 三个 reencrypt 函数仅表名不同、函数体逐字节相同；`storage.rs:595` 已有通用版可收敛 | `[x]` 已修复 |
| P017 | P2 | 重复代码 | `crates/solosoul-core/src/objects.rs:1102` vs `src-tauri/src/commands/attachment/mod.rs:114` | `load_all_referenced_attachment_ids` 跨 crate 双实现（后者 test-only），建议保留一个共享实现 | `[x]` 已修复 |
| P018 | P2 | 重复代码 | 全库 93 处 | `conn.lock()` + `ok_or("Vault is locked")?` 守卫样板 93 处，可考虑宏/helper 收敛（设计惯性，非 bug） | `[x]` 已确认设计保留（沿用既有渐进治理决定） |
| P019 | P2 | 可维护性 | 详见下文清单 | Rust 过长函数 Top10（>50 行非注释，最长 159 行） | `[x]` 已核实既有拆分，其余编排函数按设计保留 |
| P020 | P2 | 可维护性 | 详见下文清单 | Rust 深层嵌套（≥5 层）多处，最深 `auto_sync_core.rs:87` 达 8 层 | `[x]` 已核实审计样板收敛，其余结构性嵌套保留 |
| P021 | P2 | 可维护性 | 详见下文清单 | 前端超长组件第二梯队（6 个 300+ 行组件 + `pluginStore.runPlugin` 137 行 + `syncStore` 内嵌监听器 120 行） | `[x]` 已核实逻辑拆分，其余内聚组件按设计保留 |
| P022 | P2 | 性能 | `src/components/attachment/PhotoAlbumGrid.tsx:153` | 相册网格 `items.map` 全量渲染 DOM，无窗口化上限（缩略图已懒加载缓解） | `[x]` 已修复 |
| P023 | P2 | 性能 | `src/pages/scan/ScanLocalPage.tsx:107` | 目录导入 `Promise.allSettled` 并发无上限，大目录瞬时打出大量 IPC | `[x]` 已修复 |
| P024 | P2 | 死代码 | 详见下文清单 | 6 个 `export` 仅在定义文件内部使用，可去掉 export（无整文件级死代码） | `[x]` 已修复 |
| P025 | P2 | 重复代码 | 详见下文清单 | 已有共享 `CopyButton.tsx`，仍有 ≥7 处自行实现「复制到剪贴板+已复制反馈」 | `[x]` 已修复（hook 方案） |
| P026 | P2 | 重复代码 | 详见下文清单 | `visibleLimit`+slice+「加载更多」增量分页模式重复出现于 5+ 文件，可抽公共 hook | `[x]` 已修复 |
| P027 | P2 | 规范偏离 | `src/components/editor/FieldSuggestions.tsx:47,125-131` | 字段推荐对 internal 级明文展示（有意设计且有注释），与 P036 不一致，建议确认例外或写回 AGENTS.md | `[x]` 已确认例外并写入 design_map/12 规范 |
| P028 | P2 | 规范偏离 | `src/components/sync/SyncConflictDialog.tsx:373-433` + `src-tauri/src/commands/sync.rs:371` | 同步冲突对话框明文渲染 sensitive/critical 字段差异值（场景可辩护），建议对受保护字段加揭示交互 | `[x]` 已修复 |
| P029 | P2 | 架构 | `src-tauri/src/commands/vault_directory.rs:160-166` | `vault_set_directory` 后端锁定 Vault 但不 emit `vault-locked` 事件，前端认证态不失效，后续命令报锁错误但 UI 仍显示已解锁 | `[x]` 已修复 |
| P030 | P2 | i18n | `src/pages/settings/CloudSyncPage.tsx:603` | `common:enabled` / `common:disabled` 双语均缺 key 且无 defaultValue，UI 直接渲染原始 key 字符串 | `[x]` 已修复 |
| P045 | P2 | 安全/内存 | `crates/solosoul-vault/src/encryption.rs:13`、`storage.rs:728` | 密钥仅实现清零标记 trait，未实现 Drop；Vault 配置还保留锁定时未擦除的密钥副本 | `[ ]` 待修复 |
| P046 | P2 | 兼容/迁移 | `src/stores/settingsStore.ts:434` | 旧页面迁移不保留 ID，且部分成功后失败页面不再重试 | `[ ]` 待修复 |
| P047 | P2 | 安全/临时文件 | `crates/solosoul-plugin/src/manager.rs:564`、`host.rs:1322` | 插件解密附件使用普通临时目录和默认文件权限，共享临时目录环境可暴露明文 | `[ ]` 待修复 |
| P049 | P2 | 移动端导航 | `src/hooks/useOverlayBackGuard.ts:24` | 用对象引用识别 History 标记，真实浏览器结构化克隆后会把活跃相册当残留额外后退 | `[ ]` 待修复 |

#### 修复说明（续）
- **P027**：核实代码注释（推荐场景用途=引用同名字段快速填入，internal 在编辑页
  本就以揭示态呈现）后，将例外正式写入 `docs/design_map/12_敏感度等级规范.md`
  §2.3。
- **P028**：新增 `extractFieldLevels`（从 __fields.sensitivityLevel 提取字段→
  敏感度映射，本地/远程取更严格者）+ `ProtectedValue` 组件——受保护字段
  默认 MASK_PLACEHOLDER，点击临时揭示（useRevealState 1 分钟 TTL）；标量行与
  diffEntry 叶子行共 6 处值渲染全部包裹。TSC/Vitest(928) 回归通过。

## 修复进度

- 已关闭：36 / 49（含已修复及已确认设计例外；本轮完成 P037）
- 未关闭：13 项（P012、P038–P049）
- 当前处理：本轮发现 13 项（9 项 P1、4 项 P2），P037 修复已验证；用户已明确授权提交推送本轮报告和现有修复。其余 12 项新问题及历史 P012 保留待修复/延期状态，尚未达到最终验收条件。

## 第六轮全面复审（2026-09-17）

- **依据与基线**：按 `review_code_process.md` 执行，基线提交 `39e43634`；保留历史清单并复核唯一未关闭项 P012。审查前端、Tauri Rust workspace、CLI、平台桥接、权限边界及发布脚本，跳过依赖、构建和生成目录。静态扫描和重点调用链复核不等于每个源文件逐行审计。
- **工作区边界**：开始时已有 `src-tauri/Cargo.toml` 的 macOS feature 变更、Android 打包插件资产和两个诊断 ZIP。生成资产和诊断包不纳入审查提交；不为取得干净状态发布诊断资料或覆盖用户文件。Cargo 差异单独核验，不混入修复。每项修复按明确文件路径独立提交并推送。
- **CLI 检查副产物**：独立 Cargo 检查因锁文件滞后自动同步了路径依赖版本和已有依赖；测试完成后已保存生成副本并恢复检查前的 `solosoul_cli/Cargo.lock`，不把该派生变更混入审查。
- **备份**：修复前原文件保存于 `/private/tmp/solosoul-audit-20260917/.backup/`，不将备份提交到仓库。
- **检查进度**：TypeScript、Rust fmt、Clippy、ESLint 通过。首次 Rust 测试中 5 项因沙箱禁止绑定本地回环端口失败，允许本地监听后完整 `cargo test` 通过，GUI 460 项已全部执行，不再有旧报告的 Windows DLL 启动阻断。首次 Vitest 因 Node 实验性 Web Storage 与测试环境冲突出现 42 个用例失败及 3 个未处理错误；使用 `NODE_OPTIONS=--no-experimental-webstorage npm run test` 后 **117 文件 / 975 用例全部通过**。Markdown 分块、211 条 ACL、22 个偏好键检查通过。CLI fmt/Clippy 通过，166 个单元测试和 2 个集成测试通过，1 个文档示例按既有设置忽略。上述环境失败与修正运行条件均保留记录，不将环境故障当作产品代码修复。
- **P012**：复核仍有可选指纹、裸 PIN 兼容；沿用旧延期状态。已询问本轮是否改变手动恢复的产品约束，未将其当成已修复。
- **提交授权**：首次 P037 的提交推送被自动审批拒绝，当时操作没有执行；理由为全面审查未被认定包含发布授权。随后用户明确要求“提交推送”，本次交付包含 P037 的两个签名脚本、假密钥回归测试及两份审查报告，推送目标为 `origin/main`。其余发现的代码尚未修改，不创建审查通过标签。
- **覆盖和边界**：Git 跟踪的前端 TS/TSX/CSS 664 文件、GUI Rust 96 文件、共享 Rust 98 文件、CLI Rust 79 文件纳入静态扫描；另检查构建与发布脚本、CI、capabilities。crate 本地依赖图无环。未发现新的裸文件对话框调用、`eval` 或 `dangerouslySetInnerHTML`，近期添加页面定位快照和材质平台隔离未发现新的确定问题。未做原生 Android/Windows 运行验证、真实网络恢复攻击、真实签名或用户数据库读取，也不以函数行数/重复率直接判定缺陷。

### P037 — 发布签名私钥泄露到调试日志及进程参数（P1）

- **触发与证据**：`build_macos_release.sh --verbose` 开启 `set -x`，读取私钥、检查非空及调用 signer 时都会展开并记录私钥。两个脚本均使用 `--private-key "$TAURI_SIGNING_PRIVATE_KEY"`，使私钥进入 signer 进程参数；`bash -x scripts/sign_artifacts.sh` 同样泄露到 trace。
- **影响**：构建日志及进程参数可能包含可签发伪造更新的私钥。本轮仅用假密钥进行回归，不读取真实密钥，也不断言历史日志已实际泄露。
- **修复方案**：在脚本开头关闭 shell xtrace，详细模式只传递给 Tauri 构建命令；利用 signer 已支持的环境变量输入私钥，移除私钥命令行参数。回归覆盖详细模式、外部 `bash -x` 及环境传递。
- **修复说明**：两个脚本开头 `set +x`，构建详细模式改为 Tauri 的 `--verbose`；signer 从导出的 `TAURI_SIGNING_PRIVATE_KEY` 读取私钥，命令参数不再带其内容。签名工具环境变量支持已核对本地 CLI 帮助。
- **验证**：`bash -n`、`git diff --check` 通过；`python3 scripts/tests/test_signing_secrets.py` 的 3 项测试共 14 个子场景通过，覆盖文件/环境变量输入、正常/详细/外部追踪模式及缺失私钥。测试运行临时工程与假工具，仅使用假密钥，验证 stdout/stderr/argv 不含私钥且 signer 环境获得正确值；未运行真实发布构建或签名。

### P038 — 插件点路径卸载可删除数据根目录内容（P1）

- **证据与触发**：`solosoul-plugin/src/store.rs:17–30` 的白名单允许 `.` 与 `..`；`plugin_dir` 直接 join，`delete_plugin:153–156` 递归删除。GUI 的 `plugin_uninstall` 经 manager 进入该共享入口，无补充校验。独立 Rust 临时沙箱复用当前规则，传 `..` 后同级假账户文件被删除，调用返回成功；没有接触用户目录。
- **修复与验证要求**：在共享 ID 校验拒绝点路径及路径分隔符，不能只修 UI。为保存、读取、卸载添加无副作用失败测试，验证根目录和其他插件/假账户哨兵保留。

### P039 — 前端会话缓存晚到回填及插件输出串账户（P1）

- **证据与触发**：`AppRoutes.tsx:275–295` 在锁定时 clear；object/template/trash/profile store 的异步 action 在 await 后无会话代次检查，再次写入已清空缓存。对真实 store 注入延迟 Promise，四类 store 均复现「load→clear→resolve，0 条重新变 1 条」。settings/sync/llmStats 有同类路径；旧 `loadSettings().then(...)` 还会发起新一轮旧账户页面加载。
- **独立表现**：pluginStore 的结果、日志、交互请求按插件 ID 保存，锁定入口没有清理；A 账户运行插件后锁定，B 登录打开插件页可显示 A 的结果。
- **修复与验证要求**：共享会话代次守卫，clear 令旧请求失效，覆盖成功、失败与后续加载；列表请求还需防乱序。插件清运行态、保留公开安装/市场信息，并拒绝旧运行事件和最终结果回填。回归包括 A→锁定→B、旧成功/失败晚到、重复列表读取、保存失败不得回滚 B 设置。该项与后端插件访问控制 P048 分开处理。

### P040 — 同步冲突完整内容绕过 Vault 加密（P1）

- **证据与触发**：`sync/delta.rs:190–240` 从已解密对象/资料/会话构造冲突；`storage/sync_apply.rs:142–180` 序列化后直接写入 `sync_conflicts.local_data/remote_data`。存储使用普通 `Connection::open`，无 SQLCipher；详情读取也不解密。冲突发生后，即便锁定，数据库中的这两列仍为明文 JSON。
- **修复与验证要求**：写入前加密、详情读取解密；对既有 `encryption_version=1` 数据另做幂等事务迁移，并纳入密钥轮换。验证旧库迁移、原始 SQL 无明文、upsert、锁定、详情与解决冲突、损坏密文整事务回滚。不能仅修改首次加密迁移，否则现有用户早退漏迁移；旧 WAL/空闲页需单独考虑，不能声称历史备份已清除。

### P041 — 更换主密码/KDF 后聊天记录仍用旧密钥（P1）

- **证据与触发**：`conversations.rs:93–100` 用 Vault data_key 加密聊天；`reencrypt.rs:28–35` 的整库换钥清单遗漏 `llm_conversations`。改密及 KDF 升级都调用该入口，然后切换新密钥，旧聊天读取和同步解密失败。
- **修复与验证要求**：把聊天 blob 表纳入同一重加密事务；`probe_data_key` 也需覆盖仅有聊天/冲突的崩溃恢复场景。测试真实改密后锁定重开可读取聊天、损坏行整体回滚。已被旧版改密损坏的聊天需要旧密码和旧盐值另行恢复，不能把预防修复描述成自动修复所有历史损坏。

### P042 — 插件列对象 API 绕过字段声明（P1）

- **证据与触发**：`field.rs:615–662` 两条 list_objects 路径都直接返回全部 `properties`；host `solosoul_list_objects` 无附加字段过滤。插件只声明地址街道字段，也能从同对象得到未声明的敏感字段；旧路径还能请求其他类型。request_field 的声明检查没有覆盖该入口。
- **修复与验证要求**：统一按声明字段、contract/role 绑定投影属性，拒绝未声明类型与空权限。需兼容只声明 contracts/roles 的现有到期提醒插件，不能简单全部禁用。回归覆盖允许 street、拒绝 secret/其他模板、自定义角色映射、通配及嵌套字段。

### P043 — 恢复账户 ID 可指向 Vault 根目录（P1）

- **证据与触发**：恢复协议接收网络 account_id，`vault_service/account.rs:307–334` 未做路径语义校验就创建账户。ID 为 `.` 或 `./` 时可写到根目录；随后故意无效的恢复包导入失败，`commands/recovery.rs:413` 清理账户，`delete_account:335–347` 最终递归删除根内文件。此链经静态完整调用链核对，未运行真实恢复攻击。空 ID 在创建阶段被 `/config.json` 拒绝，不能用空 ID 描述完整远程复现；但删除入口单独接受空 ID 仍不安全。
- **修复与验证要求**：共享账户 ID 校验覆盖创建、删除、清单加载、配置读写和解锁，且在锁定/缓存/KDF/磁盘副作用前拒绝非法输入。兼容现有 `acc_...`、`acc-1`、`acc_restore_same_name`；不要禁止文件系统抽象合法的根相对路径操作。用 TempDir 哨兵验证恶意 ID 不改变根目录、其他账户、缓存和解锁状态。

### P044 — 回收站/快照受保护字段默认明文（P1）

- **证据与触发**：打开回收站详情无需揭示操作；`TrashDetailSections.tsx:266,298`、`TrashSnapshotView.tsx:403,520` 只放敏感度标签，真实值直接交给 ValueContainer。真实组件 SSR 注入合成 critical 字段，默认 HTML 中包含明文且无掩码。
- **修复与验证要求**：复用 useRevealState、统一敏感度与验证组件，覆盖普通值、动态子字段和快照。必须同时保护 ValueContainer 的复制/展开值，不能只用 CSS 遮住。验证 public 可见、其余默认 8 圆点、揭示验证失败仍隐藏及到期回隐；模板字段类型说明不应误掩码。

### P045 — 密钥自动擦除实现和配置副本遗漏（P2）

- **证据**：`encryption.rs:13–23` 仅手工实现 `ZeroizeOnDrop` 标记，未实现 Drop；该 trait 本身不负责擦除。`storage.rs:728` 从可复制的 `config.data_key` 创建会话密钥后还保存完整 config，lock 只擦会话字段，配置副本保留。该项为静态实现缺口，没有读取进程内存或真实密钥。
- **修复与验证要求**：实现实际清零 Drop；open 时取走配置密钥，避免持久保存副本。用析构调用/配置内容的安全测试验证，不使用读取已释放内存的未定义行为测试。

### P046 — 旧自定义页面部分迁移后无法补齐（P2）

- **证据与触发**：`settingsStore.ts:434–497` 只要 objects 已有任一 page 就提前返回，迁移创建又未保留旧 ID。真实 store 模拟 A/B 两旧页、A 成功 B 失败：首轮成功页仍用悬空旧 ID；再次加载遇到 A 即返回，B 不再重试。保留旧 preferences 的旧修复没有闭合重试链。
- **修复与验证要求**：保留稳定 ID、按 ID 差集迁移并合并，全部确认成功才清旧数据。覆盖部分失败→重启→补齐、迁移成功立即导航、已有新页与旧页并存。

### P047 — 插件明文附件临时文件权限与异常清理（P2）

- **证据与条件**：`manager.rs:564–569` 用普通 create_dir_all 创建插件临时工作区；`host.rs:1322–1326` 解密附件，`attachment_crypto.rs:66` 使用默认 File::create。共享 `/tmp` 且 umask022 时目录/文件可被其他本地用户读取；macOS 默认私有 TMPDIR 有外围保护，不能泛称所有平台均暴露。spawn join 出错路径还会跳过后续清理。
- **修复与验证要求**：0700 临时目录与 0600 明文文件，RAII 管理所有失败退出清理。测试仅用临时假附件，检查权限及成功/异常路径；不读取真实附件。

### P048 — 插件缓存与会话过期未重新校验（P1）

- **证据与触发**：`field.rs:91–160` 的三个 cached_* 仅确认持有 Vault/account 引用，缓存命中后直接返回明文，不查看 Vault 当前锁定状态。插件先读数据再锁定后继续执行，仍可读取已缓存内容。`sandbox.rs:76` 将 session 参数标为未使用，列表清理过期会话不等于 Host 访问被撤销。
- **修复与验证要求**：在实际 Host 数据访问时校验会话有效期和对应 Vault 解锁状态，锁定/撤销后失效缓存及运行访问。测试预热→锁定→再读拒绝，以及会话过期拒绝，不能仅依赖前端 P039 清理。

### P049 — 浮层历史所有权误依赖对象身份（P2）

- **证据与触发**：`useOverlayBackGuard.ts:24–32` 假设 pushState/popstate 保持同一对象引用；原对象放入 Set，sweeper 用事件对象查询。隔离真实 Chrome 空白页实测 history.state 与 marker 不同，popstate.state 也不等于原对象。全屏查看器返回相册层时，活跃相册被判成残留，额外 history.go(-1)，导致多退一层。
- **修复与验证要求**：用稳定字符串 marker ID 判断所有权，保留 StrictMode 和卸载竞态保护。补真实浏览器或结构化克隆语义的回归，验证内层→相册→页面逐层返回及锁定残留跳过。未在真实 Android WebView 上运行本轮验证。

## 本轮核验（2026-09-10）

- **复审结果**：本轮 P031–P036 六项实际修复与 P010/P018–P021 五项历史状态校正均已逐项独立提交，清单 35/36 已关闭。P012 仍存在：不提供可信预期指纹时没有主机身份校验，PIN/nonce/限流不防御主动中间人；本轮未收到变更旧延期决定的答复，继续保留为 P2，不标为已修复。
- **补充验证**：`cargo test --workspace --exclude solo_soul --no-fail-fast -j 2` 编译及执行成功，527 passed / 0 failed / 3 ignored；随后 P036 的性能基线以 `--ignored` 显式运行并通过。WebDAV 依赖外部服务的用例因缺少 URL 自行跳过。GUI 的 STATUS_ENTRYPOINT_NOT_FOUND 仍未定位，故 check-all 不计为全链通过，不创建 audit-passed 标签。
- **提交边界**：仅提交本轮明确路径；用户原有子模块指针、NSIS 位图及搜索索引改动保留。`git push origin main` 曾被自动审批拒绝，远程推送未执行；审批理由为未明确授权将具体报告/项目内容发送到 github.com/Gczmy/SoloSoul 的 main。

- **P036**：性能基线循环使用 `_` 表达不读取索引，WebDAV 测试去除局部未用 Pin 导入（顶层仍有真实用途的导入保留）。验证：cargo fmt 通过，两份测试各自 `cargo clippy --test ... -- -D warnings` 通过；显式运行通常被忽略的大数据集基线 1/1 通过，WebDAV 测试目标 9/9 返回成功，其中依赖外部服务的用例因未设置 URL 自行跳过网络操作，不计为真实 WebDAV 服务验证。

- **P021**：核实 pluginStore.applyPluginRunEvent、syncStore.refreshAfterInbound、RecoveryManualEntryPanel 已落地；同步合并分支亦无旧的重复 loadStatus。PageGuide、附件/相册预览及对象编辑器沿用内聚保留决定；补查 PluginDashboardPage 已委托卡片、结果、日志、参数及授权对话框，并用 useMemo 处理派生数据，无仅因行数而继续拆分的依据。该项仅校正报告状态。验证：源码与调用点核对、`git diff --check`；本项只修改报告，不重复运行已通过的代码测试。

- **P020**：核实 commands/biometric.rs 的 write_biometric_audit 与 unlock_audit_action_type 已收敛点名的六处审计样板；tokio::select! 分支及 SQL 链式访问保留既有结构。该项关闭表示已完成针对性重构并保留已评估结构，不表示消除全部五层以上嵌套。验证：源码与调用点核对、`git diff --check`；本项只修改报告，不重复运行已通过的代码测试。

- **P019**：核实 import_attachments 三个阶段 helper、attachment_download 源/目标校验 helper、build_upgraded_config、map_trash_change_row、build_page_delete_trash_items、build_preview_object_summaries 均已存在并被调用。handle_inbound、recovery_restore_from_host、export_objects_document 仍为分阶段编排，沿用旧报告的保留决定；biometric 审计重复归入 P020。关闭落后的待修复状态，不为函数行数重复拆分。验证：源码与调用点核对、`git diff --check`；本项只修改报告，不重复运行已通过的代码测试。

- **P018**：核对 storage.rs 的 with_tx 与 LockHoldObserver，显式锁守卫承载锁生命周期和观测边界；原报告已确认这些样板不属于行为缺陷，既有处置决定为触碰存储层时渐进收敛。本轮沿用该决定关闭机械迁移建议，未声称完成 93 处改写。验证：源码与调用点核对、`git diff --check`；本项只修改报告，不重复运行已通过的代码测试。

- **P035**：相册 3 个用例内部已有 8 秒懒加载等待，但默认用例超时仅 5 秒，冷启动时等待尚未结束便被终止。将这 3 个用例的总预算明确为 12 秒，保留原断言和 8 秒等待上限，不调整全局测试超时。验证：Prettier、TSC、ESLint 通过；最终完整 Vitest 回归 **111 文件/939 用例全部通过**（maxWorkers=2，164.79 秒）。

- **P034**：增加跨平台 `run-python-check.cjs`，按平台选择 Python 3 入口并启用 UTF-8；仅 ENOENT 才尝试下一个解释器，保留真实检查失败的退出码。check-all 统一调用 check:acl/check:pref-keys，ACL 脚本直接运行时也设置 UTF-8 输出。验证：Node 语法检查通过；本机无 python3 时两个 npm 检查分别通过（205 命令、20 键），直接运行 ACL 也通过；注入 `sys.exit(7)` 确认退出码 7 原样传递。

- **P033**：CLI `security.rs` 生物识别导入按 rustfmt 排序；修改前该行使 `cargo fmt --manifest-path solosoul_cli/Cargo.toml --check` 失败，修改后通过。仅导入排序，无运行时行为变化，不新增重复测试。

- **P032**：新增 `trackAsyncListener`，统一接管 7 个模块的 8 个异步订阅，组件先卸载时也会释放晚到的句柄；回调采用 effect 局部存活标记，避免 StrictMode 旧回调复活。目录同步进度监听与翻译依赖分离，清理卸载计时器，并在新进度到达时取消旧计时器。原实现的 3 个回归用例均先复现失败，修复后新增 8 个定向用例全部通过（含语言切换与异常清理）；TSC、ESLint 通过。完整 Vitest 回归 111 文件/939 用例中 937 通过，未修改的相册文件出现 5 秒超时及后续交互失败；独立复跑该文件 12/12 通过，保留首次失败记录。

- **P031**：本机 `npm run check-all` 已通过 TypeScript 与 Rust fmt，随后 Clippy 因 `tighten_file_perms(path)` 的非 Unix 未用参数失败。将参数及 Unix 分支引用统一命名为 `_path`，保留现有权限行为；验证：`cargo fmt --check`、全 workspace `cargo clippy -- -D warnings` 通过；`cargo test -p solosoul-core --lib` 202/202 通过。全量 `cargo test` 编译完成，但 GUI 测试程序启动以 `0xc0000139 (STATUS_ENTRYPOINT_NOT_FOUND)` 退出，尚未执行用例；直接 `--list` 同样失败，具体 DLL 入口仍待定位，不计为全量测试通过。

- **P010**：既有提交 `77e53977` 已将目录设置的状态与处理器迁入 `useVaultDirectory.ts`，`VaultDirectorySection.tsx` 仅组合展示。核对两个文件及 Git 历史后关闭遗漏的待修复状态，本次未重复拆分。文档变更用 `git diff --check` 验证。
- 启动时工作区已有 NSIS 位图、搜索索引和插件市场子模块指针改动，本轮按明确路径暂存，保留这些既有改动。本轮前端基线：TSC、ESLint、109 文件/931 用例、Markdown 分块、205 条 ACL、20 个偏好键、双语 0 缺键均通过。Vitest 初次 7 个 worker 启动超时，降低并发后这 7 个文件的 183 个用例全部通过。Windows 上 Python 检查使用 `python -X utf8` 运行，原 check-all 入口的兼容问题另项处理。

## 历史延期项处置决定（2026-08-24 审查轮收尾）

以下为当时的处置记录；2026-09-10 当前状态以问题清单与本轮核验为准：

| ID | 类别 | 延期理由 | 建议时机 |
|----|------|----------|----------|
| ~~P007~~ ✅ | CloudSyncPage 拆分 | **已完成**：631 行 → 主组件 96 行 + useCloudSyncPage(251) + 7 个 section（26~116 行） | — |
| ~~P008~~ ✅ | DatePicker 拆分 | **已完成**：609 行 → 主组件 357（分段输入逻辑）+ helpers 143（纯函数逐字节保真）+ Calendar 189 | — |
| ~~P009~~ ✅ | useExportImportPage 拆分 | **已完成**：454 行 → 341 行主 hook + useExportExecution(190) + guide 配置(45)；导入/范围/估算本已委托子 hook | — |
| ~~P010~~ ✅ | VaultDirectorySection 拆分 | **已完成**：423 行 → 展示层 261 + useVaultDirectory(216)；顺带收敛目录切换成功后的重复收尾为 afterDirectorySwitched | — |
| P012 | Recovery 指纹强制化 | UX 流程变更（手动输入路径要求录指纹），需产品确认 + GUI/CLI 双端改造 + i18n + 测试 | 产品决策后单独排期 |
| P018 | 93 处 lock 守卫样板宏收敛 | 报告自述「设计惯性非 bug」；93 处机械替换 churn 大、回归面广、零行为收益 | 触碰 storage 层时渐进采用新 helper，不做一次性迁移 |
| ~~P019~~ ✅ 6/9 | Rust 过长函数 Top10 | **已完成**：import_attachments→3 阶段函数、attachment_download→2 校验函数、unlock_with_kdf_upgrade→build_upgraded_config、list_trash_changes→map_trash_change_row、page_delete→build_page_delete_trash_items、import_decrypt_preview→build_preview_object_summaries。**handle_inbound / recovery_restore_from_host / export_objects_document 维持现状**——已是「命名 helper 的纯编排层」（下载→建户→导入各阶段自成函数），再拆只是搬参数表反而降低可读性 |
| ~~P020~~ ✅ 点名项 | Rust 深层嵌套 | biometric 系 6 处审计样板收敛 write_biometric_audit + unlock_audit_action_type（报告点名「值得优先重构」项）；其余为 tokio::select!/SQL 链式结构性嵌套（报告自述实际风险低），维持现状 |
| ~~P021~~ ✅ 逻辑类 3/6 | 前端超长组件第二梯队 | pluginStore.runPlugin 巨型 switch→applyPluginRunEvent 纯函数；syncStore 入站刷新尾收敛 refreshAfterInbound（消除双分支重复）；RecoveryQrContent 手动面板拆出 RecoveryManualEntryPanel（391→205+233）。**PageGuide / AttachmentPreviewOverlay / PhotoAlbumOverlay / useObjectEditorPage 维持现状**——手势拖拽与预览生命周期高内聚，拆分损害内聚性且视觉回归无法在此环境验证 |

## 历史收尾验证基线（2026-08，含结构性拆分轮）

- `cargo fmt --check` / `clippy -D warnings`：✅
- Rust workspace：**994 passed / 0 failed**
- TSC / ESLint：✅（0 error 0 warning）
- Vitest：**928 passed**
- `check_acl_consistency.py`：✅ 205 命令全登记
- `check-missing-i18n.mjs`：✅ 双语 0 缺失

> 结构性拆分轮新增提交（按序）：CloudSyncPage 拆分 → DatePicker 拆分 →
> useExportImportPage 拆分 → VaultDirectorySection 拆分 → import_attachments/
> attachment_download/unlock_with_kdf_upgrade/list_trash_changes/page_delete/
> import_decrypt_preview 六处过长函数拆分 → biometric 审计样板收敛 →
> runPlugin/syncStore/RecoveryQrContent 三处前端逻辑拆分 → fmt 归一。

#### 补充修复说明（历史条目存档）

#### 修复说明（续）
- **P026**：新增 `useIncrementalWindow(initial, step)`（limit/hasMore/showMore/
  reset/setLimit），5 个站点迁移：OperationLogPage、DebugLogPage、HistoryPage、
  useTrashPage、useObjectWorkspaceData。PhotoAlbumGrid（P022 新增）暂保留内联
  实现（组件局部 state，无 reset 语义）。ESLint exhaustive-deps 全部补齐。
  TSC/ESLint/Vitest(928) 回归通过。

#### 修复说明（续）
- **P023**：handleImportAll 改 CONCURRENCY=4 分批 allSettled，失败统计语义
  不变。TSC 回归通过。

#### 修复说明（续）
- **P022**：PhotoAlbumGrid 加增量窗口（INITIAL_VISIBLE_LIMIT=200，
  「加载更多」按钮步进 200，items 变化重置）；组件测试 4 项回归通过。

#### 修复说明（续）
- **P024**：6 处冗余 export 全部去除（buildPdfPreviewSrc/SWIPE_THRESHOLD/
  FORMAT_FILTERS/CONFLICT_VALUE_MAX_LEN/prefetchWarmupTasks/MockResizeObserver）。
  TSC/ESLint/Vitest(928) 回归通过。

#### 修复说明（续）
- **P017**：core 版改 `pub` 导出；solo_soul 侧 #[cfg(test)] 重复实现删除，
  测试改为 `use solosoul_core::objects::load_all_referenced_attachment_ids`
  （两实现语义一致：对象 __attachments 引用集合）。solo_soul 465 测试回归
  通过。

#### 修复说明（续）
- **P016**：reencrypt_profiles / reencrypt_trash_items / reencrypt_object_snapshots
  收敛为 `reencrypt_blob_table(tx, table, old_key, new_key)`（表名参数化，SQL
  format! 拼接——表名为编译期常量字面量无注入面）；调用点改三行单行调用。
  solosoul-vault 172 测试回归通过。

#### 修复说明（续）
- **P015**：preview 命令改调主路径 `decrypt_package`（流式解密至数据目录内
  NamedTempFile + `serde_json::from_reader`），删除 read_file_from_zip +
  decrypt_chunked_from_bytes + from_slice 链路；解密失败错误码由
  decrypt_zip_entry_streaming 内部映射，前端 i18n 行为不变。solo_soul 465
  测试回归通过。

#### 修复说明（续）
- **P014**：properties 改 `Cow<'_, serde_json::Value>` 借用原值，仅在实际
  注入 __templateName 时 `to_mut()` 触发克隆；批量保存路径（无模板对象占多数）
  零拷贝。solosoul-vault 172 测试回归通过。

#### 修复说明（续）
- **P013**：`repair_restored_objects` 的 SELECT 先收集、stmt 提前 drop，
  修复循环整体包 `with_tx`（失败回滚不留半改状态）；REPAIR_FLAG 仅在
  事务成功后落位。solosoul-vault 测试回归通过。

#### 修复说明（续）
- **P011**：新增 `create_private_temp_dir`（0700 + UUID 随机目录名）与
  `tighten_file_perms`（0600），替换导出（write_attachment_entries）与导入
  （import_vault）两处生产路径的共享固定目录；测试内第 3 处为快照对比用途，
  不涉明文，保持原样。export_import 17 测试回归通过。

#### 修复说明（续）
- **P029**：`vault_set_directory` 在 `svc.lock()` 之后 emit `vault-locked`
  （与 commands/vault.rs::lock 对齐），前端 AppRoutes 监听链自动失效认证态。
  Clippy 回归通过。

#### 修复说明（续）
- **P006**：`copy_snapshots` 包 `with_tx`（失败整体回滚）；循环内去掉同钥
  解密→重加密，直接复制密文行；`data_key()` 保留作解锁态校验。
  solosoul-vault 172 测试回归通过。

#### 修复说明（续）
- **P005**：双保险——① 存储键改为 `solosoul_recent_searches:{accountId}`
  按账户隔离；② authStore `lock()`/`logout()` 调 `clearRecentSearches()`
  清除全部前缀键。TSC/ESLint/Vitest(928) 回归通过。

#### 修复说明（续）
- **P004**：MatchHint 的 fieldValue 分支抽出 `FieldValueHint` 子组件——
  `sensitivityLevels` 任一非 public 即渲染 `MASK_PLACEHOLDER`（点击揭示，
  复用 useRevealState 1 分钟 TTL）；SearchPage/SearchPopover 共用路径一次收敛。
  后端 search_filter.rs 保持不变（匹配仍覆盖 internal 值，仅展示层掩码）。
  TSC/ESLint/Vitest(928) 回归通过。

#### 修复说明（续）
- **P003**：新增 `is_local_http_host` 判定——http 仅允许回环/RFC1918 私网/IPv6
  unique-local/.local 主机名（局域网 NAS 明文属可接受用户选择）；公网 host http
  返回 `ConfigMissing` 类型化错误并提示改用 https。单测 `test_p003_http_policy`
  覆盖 9 种地址形态。E2E 9/9 回归通过（本地 wsgidav 即 127.0.0.1 回环）。

#### 修复说明
- **P001**：default.toml 按字母序补入 8 个命令；`check_acl_consistency.py` 现报
  「205 个命令均已登记」，exit 0。
- **P002**：lib.rs generate_handler! 补注册 `commands::settings::ui_get_preferences`
  （测试断言字符串清单本就含此命令，印证遗漏）；settingsStore/notification/onboarding
  三处读链路恢复。

#### 补充说明（P025 方案选择）

未采用「全部换用 plugin-market 的 CopyButton 组件」——其样式/文案形态与各站点差异大
（图标按钮、键控多目标、toast 反馈），强行替换会改变视觉与交互。改为抽取共享
`useCopyToClipboard` hook（copy 返回布尔 + 键控 copied 态 + execCommand fallback），
6 个站点保留各自样式仅收敛逻辑：GuideCodeBlock、PluginResultPanel、
useObjectDetailModal（copiedField 键控）、useLlmChatCore（copiedIndex 键控）、
AccountSettingsPage（toast 驱动）、SyncShowQrDialog（addr/pin 双键，含 fallback）。
RecoveryQrContent 为纯展示组件（props 驱动），随 SyncShowQrDialog 一并受益。

## 初始问题描述与修复指引（历史证据，行号与计数未代表当前代码）

### P001 — cloud_sync 命令未登记 ACL 白名单（P1，规范/CI）

- **证据**：`python3 scripts/check_acl_consistency.py` exit 1，报错：`cloud_sync_delete_config / cloud_sync_get_config / cloud_sync_list_incoming / cloud_sync_mark_applied / cloud_sync_now / cloud_sync_save_config / cloud_sync_test_connection / cloud_targets_detect 未登记到 default.toml`。
- 这些命令已注册进 handler（`src-tauri/src/lib.rs:135-142`），前端确实在调（`src/pages/settings/CloudSyncPage.tsx:87,110,204`、`src/hooks/useExportImportPage.tsx:45`）。
- **影响**：该脚本在 CI（`.github/workflows/pr_check.yml`）与 `npm run check-all` 中强制执行，当前 main 下次跑 CI 直接失败；Tauri v2 对未列入 allow 的命令默认拒绝，云同步页全部 IPC 运行时报 "not allowed by ACL"。
- **诱因**：`default.toml` 最后更新 2026-08-21，cloud-sync 系列命令是 8-21~8-23 新增，加命令时漏同步 ACL。
- **建议修复**：将 8 个命令加入 `src-tauri/permissions/solo-soul/default.toml` 的 `allow-all-custom-commands` 列表。

### P002 — `ui_get_preferences` 未注册进 handler（P1，架构）

- **证据**：命令定义于 `src-tauri/src/commands/settings.rs:166-181`，已进 ACL 白名单（`default.toml:199`），但 `generate_handler!` 列表（`src-tauri/src/lib.rs:46-193`）只有 `ui_update_preference`（:143），没有 `ui_get_preferences`；`check_acl_consistency.py` 同步报 WARN「白名单中存在但 handler 中未注册」。
- **影响（均被 catch 吞掉，无崩溃但功能降级）**：
  - `src/stores/settingsStore.ts:271` — 明文层 `ui_preferences.json` 的主题/语言永远读不回来（write 通、read 断），WebView 缓存清除后登录前主题/语言回退默认。
  - `src/lib/notification.ts:34` — 通知权限"已请求"标记读不到。
  - `src/App/index.tsx:56-75` — `hasSeenOnboarding` 读失败回落 `false`。
- **建议修复**：在 `register_core_commands` 的 `generate_handler!` 中补注册 `ui_get_preferences`。

### P003 — WebDAV 允许 http + Basic 明文凭证（P1，漏洞）

- **证据**：`crates/solosoul-core/src/cloud_sync/webdav.rs:48-65` — `WebDavConnector::new` 仅校验 scheme ∈ {http, https}，随后无条件构造 Basic base64 认证头。用户填 `http://`（自建 NAS 常见）时，每次同步在公网/局域网上明文发送账号密码。
- **对比**：LLM `validate_llm_base_url`（`commands/llm/request.rs:190`）与 OCR `validate_model_base_url`（`commands/ocr.rs:1005`）均强制非回环 https；WebDAV 错误文案写「需形如 https://」却实际放行 http，文案与行为矛盾。
- **建议修复**：与 LLM/OCR 对齐——非回环 host 拒绝 http，或显式警告并要求用户确认。

### P004 — 搜索结果 internal 字段明文渲染（P1，安全/规范）

- **证据**：`src/lib/searchShared.tsx:175-183` 直接明文渲染 `matchedValue`，无 `useRevealState`/`maskValue`；后端 `crates/solosoul-core/src/search_filter.rs:11` 的 `PROTECTED_SENSITIVITIES = ["sensitive", "critical"]` 不含 internal，即 internal 字段值参与搜索匹配并明文返回展示。
- **冲突**：AGENTS.md 规定「仅 public 永不掩码，internal/sensitive/critical 一律掩码」（P036 已收敛）。SearchPage 与 SearchPopover 共用此路径。
- **建议修复**：internal 命中值按 P036 规则掩码 + 点击揭示，或将该例外显式写回约定文档。

### P005 — 最近搜索词明文 localStorage 残留（P1，安全）

- **证据**：`src/components/layout/SearchPopover.tsx:49-63` — `solosoul_recent_searches` 保留 3 条明文，不按账户隔离，Vault 锁定/退出登录时不清除。
- **影响**：搜索词可能含证件号、姓名等敏感片段；同机换账户登录后可在搜索弹层看到前一账户的搜索历史。与 `ocrScanStore`「锁定即清空明文」、syncStore「仅落非敏感元数据」的既定做法不一致。
- **建议修复**：锁定/退出时清除，或按账户隔离存储。

### P006 — `copy_snapshots` 无事务 + 同钥多余重加密（P1，性能/事务）

- **证据**：`crates/solosoul-vault/src/storage/snapshots.rs:547-584` — 恢复回收站对象时逐条 `insert.execute(...)`，每条隐式独立事务，中途失败留半成品快照集；且每条 `decrypt_field`→`encrypt_field`（同一把 key）纯浪费，直接复制 `raw_data` 密文即可。同文件 `trash.rs:70`、`sync_apply.rs:149` 均已事务化，此处是漏网。
- **建议修复**：包 `with_tx`，去掉同钥解密-重加密，直接复制密文行。

### P007–P010 — 前端超长组件（P1，可维护性）

| ID | 文件：行 | 非注释行数 | 说明 |
|----|----------|-----------|------|
| P007 | `src/pages/settings/CloudSyncPage.tsx:51-632` | ~535 | 单组件承载 WebDAV 配置、保留策略、连接器选择、入站文件列表全部逻辑 |
| P008 | `src/components/forms/DatePicker.tsx:168-609` | ~385 | 段落解析+键盘处理+滚轮渲染混杂 |
| P009 | `src/pages/settings/useExportImportPage.tsx:33-454` | ~371 | 导出/导入 hook 状态机过长 |
| P010 | `src/pages/settings/VaultDirectorySection.tsx:27-423` | ~369 | Vault 目录设置区单组件 |

注：计数含内联 style 对象，实际复杂度略低于行数；建议按职责拆子组件/子 hook。

### P011 — 附件临时明文 temp 目录权限未收紧（P2，安全）

- **证据**：`crates/solosoul-core/src/export_import.rs:221-237`（导出）、`:1218-1242` 与 `:2214`（导入）— 解密后附件明文写入 `std::env::temp_dir().join("solosoul_export_att")` 等可预测目录名，仅 `create_dir_all`，未设 0700/0600；多用户系统上本地其他用户可预占目录或在明文窗口期读取。
- **对比**：`decrypt_to_temp_dir`（`commands/attachment/mod.rs:489-503`，0700/0600）与 `write_payload_to_temp`（`export_import.rs:955-970`，落在 0700 vault 数据目录内）均已正确示范。导入侧崩溃残留有 `cleanup_orphan_import_temps` 兜底。
- **建议修复**：复用 tempfile + 0700/0600 的既有模式。

### P012 — Recovery 指纹校验可选 + 裸 PIN（P2，安全加固）

- **证据**：`crates/solosoul-sync/src/recovery.rs:269-279`（客户端仅 `Some(expected_fp)` 才校验指纹）、`:184`（主机端放行裸 PIN）。Noise_XX 用临时身份密钥，指纹是唯一 MITM 防线；`fingerprint=None` 时主动中间人可透明中继拿到 PIN 与 32 字节恢复密码。
- **缓解**：6 位 PIN + 一次性 nonce + 全局限流（`GLOBAL_MAX_ATTEMPTS = 10`）+ served 一次性标记，暴力破解不可行；但被动 relay 不需要猜 PIN。
- **建议修复**：手动输入路径要求一并输入指纹（QR 已含），或对无指纹连接在主机端弹确认。

### P013–P015 — Rust 性能遗留（P2）

- **P013**：`snapshots.rs:369-414` `repair_invisible_objects` 循环内逐行 `query_row` + `UPDATE` 无事务；有 `sys_config` 一次性标记兜底，实际只跑一次，建议包 `with_tx`。
- **P014**：`objects.rs:448` `save_object_tx` 无条件 `obj.properties.clone()`，可加 `if obj.template_id.is_some()` 惰性克隆。
- **P015**：`import.rs:130-135` `import_decrypt_preview` 整包读入内存（上限 100MB，峰值约 3×100MB）；主导入路径（`import.rs:816`）已流式化，预览路径遗留。

### P016–P018 — Rust 重复代码（P2）

- **P016**：`reencrypt.rs:54/106/126` 三个 reencrypt 函数仅表名不同、函数体逐字节相同；`storage.rs:595` 已有通用版 `rewrite_blob_table_encrypted`，可收敛为一个参数化函数。
- **P017**：`load_all_referenced_attachment_ids` 跨 crate 双实现（`solosoul-core/src/objects.rs:1102` 与 `commands/attachment/mod.rs:114`，后者 test-only），建议保留一个共享实现。
- **P018**：「Vault is locked」守卫样板 93 处（非测试代码），设计惯性非 bug，可考虑宏/helper 收敛。

### P019 — Rust 过长函数 Top10（P2）

| # | 文件：行 | 函数 | 非注释行数 | 嵌套深度 |
|---|----------|------|-----------|---------|
| ~~1~~ | `crates/solosoul-core/src/export_import.rs:1109` | `import_attachments` | ✅ 已拆分（build_attachment_meta_map / write_imported_attachment / write_back_imported_attachments；145→约 96 行编排层，此前报告「~70 行」有误） | — |
| 2 | `crates/solosoul-vault/src/storage/sync_changes.rs:541` | `list_trash_changes_since_limited` | 123 | 5 |
| 3 | `src-tauri/src/commands/attachment/mod.rs:223` | `attachment_download` | 123 | 4 |
| 4 | `crates/solosoul-core/src/vault_service/unlock.rs:428` | `unlock_with_kdf_upgrade` | 118 | 4 |
| 5 | `crates/solosoul-sync/src/session.rs:305` | `handle_inbound` | 116 | 3 |
| 6 | `src-tauri/src/commands/biometric.rs:383` | `biometric_save_credential` | 112 | 4 |
| 7 | `src-tauri/src/commands/export_import/import.rs:117` | `import_decrypt_preview` | 110 | 5 |
| 8 | `src-tauri/src/commands/object/trash.rs:271` | `page_delete` | 107 | 5 |
| 9 | `src-tauri/src/commands/recovery.rs:302` | `recovery_restore_from_host` | 106 | 4 |
| 10 | `src-tauri/src/commands/export_import/export_docx/mod.rs:274` | `export_objects_document` | 105 | 4 |

另：`storage.rs:808 create_schema_tables`（149 行纯 DDL）与 `lib.rs:44 register_core_commands`（133 行纯注册样板）属线性样板，未计入。

### P020 — Rust 深层嵌套（P2）

- 深度 8：`src-tauri/src/sync/auto_sync_core.rs:87` `spawn_scheduler`（主要为 `tokio::select!` 分支结构）。
- 深度 7：`commands/search/query.rs:88`、`solosoul-sync/src/attachments.rs:383`、`solosoul-sync/src/manager.rs:321`、`solosoul-vault/src/storage/objects.rs:669`、`solosoul-core/src/export_import.rs:482`、`solosoul-core/src/objects.rs:954`、`commands/export_import/helpers.rs:197`。
- 深度 6（代表性）：`commands/biometric.rs:573 biometric_unlock`（两个 `.map_err` 闭包几乎逐字重复，值得优先重构）、`solosoul-sync/src/delta.rs:124`、`commands/object/mod.rs:1120`、`sync/cloud_auto_sync.rs:628`。

### P021 — 前端超长组件第二梯队（P2）

- `src/components/attachment/AttachmentPreviewOverlay.tsx:34-444`（~388 行，0 处 memo）
- `src/pages/ai/PluginDashboardPage.tsx:35-451`（~386 行）
- `src/components/sync/RecoveryQrContent.tsx:19-391`（~362 行，几乎全静态 markup）
- `src/components/attachment/PhotoAlbumOverlay.tsx:33-396`（~357 行）
- `src/pages/editor/useObjectEditorPage.ts:65-480`（~352 行）
- `src/components/guide/PageGuide.tsx:43-432`（~350 行）
- `src/stores/pluginStore.ts:203` `runPlugin` action 约 137 行
- `src/stores/syncStore.ts:587-707` `initSyncCompletedListener` 内嵌事件处理器约 120 行

### P022–P023 — 前端性能（P2）

- **P022**：`PhotoAlbumGrid.tsx:153` 相册网格全量渲染 DOM 节点，无窗口化上限；缩略图已由 IntersectionObserver 懒加载缓解。主要列表（对象工作区、历史、回收站、日志、聊天）均已窗口化或后端限量，**无系统性问题**。
- **P023**：`ScanLocalPage.tsx:107` 目录导入 `Promise.allSettled(files.map(handleImport))` 并发无上限，建议加并发上限。全库无「循环内顺序 await invoke」模式（批量操作均已 Promise.all/分批并行化）。

### P024 — 冗余 export（P2，死代码边缘）

无整文件级死代码（431/540 模块被引用，其余为测试/入口/CSS）。以下 6 个 `export` 仅在定义文件内部使用，可去掉 export：

- `src/components/attachment/useAttachmentPreview.ts:33` — `buildPdfPreviewSrc`
- `src/components/attachment/usePhotoViewer.ts:12` — `SWIPE_THRESHOLD`
- `src/components/export/useExportDocumentSection.ts:21` — `FORMAT_FILTERS`
- `src/lib/conflictFieldMeta.ts:330` — `CONFLICT_VALUE_MAX_LEN`
- `src/lib/prefetch/warmup.ts:36` — `prefetchWarmupTasks`
- `src/test/setup.ts:58` — `MockResizeObserver`（测试基建）

TODO/FIXME 注释：全库 **零**。

### P025–P026 — 前端重复模式（P2）

- **P025**：已有共享组件 `src/components/plugin/shared/CopyButton.tsx`，仍有 ≥7 处自行实现 `navigator.clipboard.writeText` + copied 状态：`GuideCodeBlock.tsx:18`、`PluginResultPanel.tsx:202`、`useObjectDetailModal.tsx:191`、`SyncShowQrDialog.tsx:157`、`useLlmChatCore.ts:254`、`AccountSettingsPage.tsx:38`、`RecoveryQrContent.tsx`（双份）。
- **P026**：`visibleLimit` + `slice(0, visibleLimit)` + 「加载更多」增量分页重复出现于 5+ 文件：`useObjectWorkspaceData.ts`、`OperationLogPage.tsx:56`、`useTrashPage.tsx`、`DebugLogPage.tsx`、`HistoryPage.tsx:97`，各约 15 行，可抽公共 hook（如 `useIncrementalWindow`）。

### P027–P028 — 掩码约定偏离（P2，有意设计待确认）

- **P027**：`FieldSuggestions.tsx:47,125-131` 字段推荐对 internal 级明文展示（注释自述「内部级在推荐场景与公开同权」），建议确认例外是否写回 AGENTS.md 或收敛。
- **P028**：`SyncConflictDialog.tsx:373-433` + `commands/sync.rs:371` 冲突对话框明文渲染本地/远端字段差异（含 sensitive/critical），冲突解决需看清差异属可辩护设计，建议至少对受保护字段加揭示交互。

### P029 — `vault_set_directory` 锁定不发事件（P2，架构）

- **证据**：`commands/vault_directory.rs:160-166` 目录切换前 `svc.lock()` 但不 emit `vault-locked`（对比 `commands/vault.rs:13-15` 会 emit，`AppRoutes.tsx:250` 有完整监听清理链）。前端 `VaultDirectorySection.tsx:97-107` 仅显示重启提示，不重置 `isAuthenticated`，用户可离开本页继续操作，后续命令报「No account is currently unlocked」但 UI 仍显示已解锁。
- **缓解**：有重启提示 UI 与各命令错误 toast，不会静默损坏数据。
- **建议修复**：后端补 emit `vault-locked`，或前端成功后重置认证态。

### P030 — i18n 缺键（P2）

- **证据**：`node scripts/check-missing-i18n.mjs` 实测 zh-CN、en-US 各缺 `common:enabled`、`common:disabled`；使用点 `CloudSyncPage.tsx:603` 无 defaultValue，缺失时直接渲染原始 key 字符串。
- **建议修复**：补两份语言的 key。

## 初始分析未发现问题的维度（历史留档，不作为本轮保证）

- **Rust 安全**：无不安全 `unsafe`（4 处均为必要平台 FFI）；无命令注入（Command 全分离参数 + 用户名白名单）；路径遍历防护完整有测试；无硬编码密钥；无 `serde(untagged)`；加密无误用（nonce 唯一、KDF 参数正确、ct 比较、Zeroizing 贯穿）；无 SQL 注入（全参数化）。
- **Rust 死代码**：无确凿发现（全部非 command 非测试函数均有真实调用点；唯一 `#[allow(dead_code)]` 是 RAII 锁句柄的有意保留）。
- **前端安全**：无 `dangerouslySetInnerHTML`/`eval`/`new Function`；Markdown 统一经 `SafeMarkdown` 消毒；无敏感数据写日志；文件对话框全部经 `lib/dialog.ts` 封装（18 个调用方无裸调）。
- **架构**：crates 依赖为单向 DAG 无循环；capabilities 无过度授权（fs/shell 均最小权限）；IPC 统一走 `ipcClient.ts` 不吞错；Zustand↔Rust 事件同步主链路完整（唯一缺口即 P029）。

## 初始分析备注（历史，不适用于本轮执行）

- 按用户要求，本轮**不进入阶段 3 修复流程**，所有问题保持 `[ ]` 待修复状态。
- P001/P002 为 CI 阻断项，建议优先处理；P003 为唯一安全策略不一致项，建议紧随其后。
