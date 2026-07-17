# 移动端开发计划实施审查报告（2026-07-17）

> **审查对象**：开发人员声称已完成的 `docs/mobile/mobile-development-plan.md` 中 P0–P4 全部任务与 P5-01。
> **审查基准**：master `7f67c190`，工作树干净。
> **审查方式**：分里程碑并行只读代码审查（P0/P1/P2/P3/P4+P5-01 五路），关键阻断证据由审查负责人亲自逐行复核；两路审查各自用 `cargo check` 在移动 target 上实际复现编译错误。前端 `npx tsc --noEmit` 与 `npx vitest run`（44 文件 400 测试）实测通过。
> **未修改任何被审查代码**；仅按需求在开发计划文档中标注 P5-02~P5-04 暂缓。

---

## 0. 总体结论

**「P0–P4 和 P5-01 已完成」的声明整体不成立。**

- P0、P2 的实现基本属实，质量良好（响应式断点从 6 个增至 20 个样式表且内容实质；无 git 冲突标记残留；前端类型检查与 400 个单测全绿）。
- 但 **移动端 Rust target 当前编译失败**（§1.1），P3 及之后的全部 Rust 改动从未编译通过；**`ci_cd.yml` 因非法 `needs` 引用整体无效**（§1.2），Android 发布链路走不通。
- P3 判定为「不成立」：除编译失败外，同步功能链路未闭环、生物识别凭证存储未达到计划的安全要求。
- P4/P5-01 代码骨架均在，但建立在编译不过的基础之上，多项验收要件（spike 文档、断点续传、真机/模拟器验证）缺失。

健康、无需担心的部分：桌面 host 编译通过（`cargo check -p solosoul-sync`）；Noise/同步协议层零分叉；P0-04 的 9 处判定语义逐处合理；P2 的 CSS 改动未见桌面端（>768px）布局被误改。

---

## 1. 阻断级问题（必须最先修复）

### 1.1 移动端 Rust target 编译失败

已亲自逐行复核，两路审查独立实测复现（`cargo check --target aarch64-apple-ios`；`aarch64-linux-android` 同样失败，因代码按 `cfg(any(android, ios))` 共享）：

| 错误 | 位置 | 说明 |
|---|---|---|
| E0252 | `tauri/crates/solosoul-sync/src/mobile.rs:6` 与 `:20` | `use crate::noise::NoiseKeys;` 与 `pub use crate::noise::NoiseKeys;` 重复导入同名 |
| E0599 | `tauri/crates/solosoul-sync/src/mobile.rs:205` vs `:247` | 字段 `running: AtomicBool`（非 Arc），`:247` 调用 `self.running.clone()`，AtomicBool 无 Clone |
| E0603 | `tauri/crates/solosoul-sync/src/lib.rs:40` | 伴随的再导出错误 |

**修复后必然暴露的第二层错误**（当前被 sync crate 错误遮挡）：

- `tauri/src-tauri/src/commands/biometric.rs:19-26`：`map_bio_error`/`bio_err` 仅 `#[cfg(desktop)]` 定义，移动端 5 个命令分支调用它（:199,220,224,229,342-352,445-457）→ E0425。
- `tauri/src-tauri/src/commands/embed_model.rs:32-34`：`fetch_registry` 仅 `#[cfg(desktop)]`，而共享命令 `llm_get_embed_models`/`llm_download_embed_model`（:296,:312）无条件调用 → E0425。另 `embed_model.rs:9-10`、`ocr.rs:12` 的 `mobile_not_supported(_with)` 导入在移动端已变为未使用（警告）。

**影响范围**：P3-01、P3-02、P4-01、P4-02、P4-03、P5-01 的全部 Rust 代码从未编译通过；CI 的 `cargo ndk check`（`build-android.yml:55-59`）与 iOS check（`pr_check.yml:41-66`）下次触发必红。修复 commit `98807cb4` 未覆盖上述任何一处——说明「修复后」从未真正验证过。

### 1.2 `ci_cd.yml` 整体无效

- `.github/workflows/ci_cd.yml:256`：`needs: [build-macos, build-windows, build-android-release]`——`build-android-release` 定义在 `build-android.yml:76`，**`needs` 不允许跨 workflow 引用**。GitHub Actions 判定 "Job 'release' depends on unknown job"，**ci_cd.yml 全部 7 个 job（frontend-check / rust-test / build-macos / build-windows / release 等）都无法运行**。已亲自核对原文。
- 即便修复 `needs`：`ci_cd.yml:272-276` 用裸 `actions/download-artifact@v4` 按名字下载 `solosoul-android-release`，该动作只能下载**当前 workflow run** 的 artifact，而 AAB 产自 build-android.yml 的独立 run——需 `run-id` + `github-token`，或重构为单 workflow。
- 叠加路径触发不一致：`build-android.yml:4-8` 仅在 `tauri/**`、`docs/android/**` 变更时触发，`ci_cd.yml` 无 paths 过滤——只改其他文件的 push 不会产生 AAB。

**修复方向**：把 AAB job 并入 ci_cd.yml，或 release job 改用 `workflow_run` 触发/「artifact 存在才附加」的容错写法。

---

## 2. 里程碑逐项判定

| 里程碑 | 判定 | 说明 |
|---|---|---|
| P0（4 项） | ✅ 属实 | 权限声明、资源排除、判定统一均达标 |
| P1（7 项） | ⚠️ 4 完成 / 3 部分 | P1-01/02/03/04 ✅；P1-05 CI 集成坏（§1.2）；P1-06 用例仅 2 条且未接入 CI；P1-07 有埋点无实测数据 |
| P2（7 项） | ⚠️ 大部分属实 | P2-01/02/04/05 ✅；P2-03 残留 21 文件未包裹 hover；P2-06 缺 E2E 断言；P2-07 冷启动两处丢事件缺陷 |
| P3（3 项） | ❌ 不成立 | 编译失败（§1.1）+ 同步链路未闭环（§3.1）+ 生物识别未达 Keystore 要求（§3.2） |
| P4（2 项） | ⚠️ 骨架在、验收缺 | ML Kit 路线代码在；无 spike 文档、无续传重试、OcrPage 零移动端适配 |
| P4-03（计划外） | ✅ 静态合理 | R8/keep 规则齐全（原为**空模板**，实为修复）；但未经一次真实 release 构建验证 |
| P5-01 | ⚠️ 工程配置齐 | gen/apple 33 文件入库、脚本/CI 齐备；从未编译通过，无模拟器运行证据 |

### P0 明细（全部 ✅）

- **P0-02**：`tauri/src-tauri/gen/android/app/src/main/AndroidManifest.xml:9` 已声明 `POST_NOTIFICATIONS`。
- **P0-03**：`tauri.conf.json:52-54` 的 resources 只剩 docs；models 已入 `tauri.macos.conf.json:6`、`tauri.windows.conf.json:5`、`tauri.linux.conf.json:5`（原有 pdfium/plugin_market 均保留）；android/ios overlay 不含 models。
- **P0-04**：`grep useIsMobile tauri/src/` 零结果；`hooks/useIsNarrowViewport.ts:3` 保留 768 断点；9 个使用点语义逐处核对合理（布局用视口、功能门控用 `isMobilePlatformSync()`）；`main.tsx:27-28` 在 render 前完成 `initPlatform()`。桌面专属路由守卫 `routes.tsx:44-60` 特意用异步 `isMobilePlatform()`，比 sync 版稳健。
- **P0-01**（顺带确认）：自动锁定实现完整未受损。

### P1 明细

- ✅ **P1-01**：`build.gradle.kts:22` `minSdk = 28`。
- ✅ **P1-02**：`build.gradle.kts:35-45` env 注入签名、缺失时回退；`:59` release 引用。无 keystore 入库；`.gitignore:53-54` 与 gen/android/.gitignore 覆盖 `*.jks`/`*.keystore`/`local.properties`；全库无明文密码。
- ✅ **P1-03**：`build.gradle.kts:25-26` versionCode = 基础值 + `GITHUB_RUN_NUMBER`；`docs/release_process.md:264-313` 已补 Android 发版规则。**偏差**：实际基础值 2005012 与文档示例 20512 不一致（见 §4 低级问题）。
- ✅ **P1-04**：`build-android.yml:52-59` cargo-ndk 安装 + `cargo ndk -t aarch64-linux-android check`，位于构建 APK 之前，PR/push 均触发。
- ⚠️ **P1-05**：release job 本体齐全（`build-android.yml:76-135`），但 ci_cd.yml 集成无效（§1.2）；AAB 未按 `SoloSoul_<version>_android.aab` 重命名。
- ⚠️ **P1-06**：mobile project（`playwright.config.ts:19-26`）、平台 mock 开关（`tauriMock.js:141-142`）、`test:e2e:mobile` 脚本均有；但 `mobile-smoke.spec.ts` 仅 2 条用例（登录+底部导航、设置页），计划要求的对象列表/详情/新建对象未覆盖，且**未接入任何 CI workflow**。
- ⚠️ **P1-07**：T1/T2 埋点已落地（`main.tsx:18-19`、`LoginPage.tsx:70-77`、`HomePage.tsx:173-180`），`android-port-guide.md:220-240` 新增基线章节；**但无实测数据落盘**，且埋点语义与指标定义不符（见 §3 中级问题）。

### P2 明细

- ✅ **P2-01**：767px 断点样式表 6 → 20 个；抽查 ObjectDetailModal（窄屏全屏页式、键值上下排列）、WorkspaceObjectCard（头部竖排）、CardGrid（390px 恰 1–2 列）均实质。注：「批量操作栏改底部固定」N/A——该页面无批量选择 UI。
- ✅ **P2-02**：`SearchPopover.module.css:12` 已改 `min(520px, calc(100vw - 32px))`；四大弹层 + Dialog + Toast 安全区全覆盖（safe-area-inset 共 30 处/12 文件）。
- ⚠️ **P2-03**：功能入口型 hover 已消除（如 ObjectDetailModal 的 hoveredField 改常驻）；`@media (hover: hover)` 已引入 15 个文件；**但 21 个文件残留未包裹**，其中 2 处移动端可达：`LlmChatPage/index.tsx:271-291`（tooltip 仅 hover）、`OcrQuickScanPopover.tsx:260-274`（内联 style 未包裹）。
- ✅ **P2-04**：全局基线（`global.css:238-244` 窄视口 44px）+ 8 个组件文件显式规则。
- ✅ **P2-05**：LoginPage 可滚动容器、编辑器 sticky 操作栏、Dialog max-height 等 CSS 方案合理，`adjustResize` 未动；真机回归无法从代码证实。
- ⚠️ **P2-06**：`DesktopOnlyGuard`（`routes.tsx:44-60`）实现正确；当前实际守卫仅 `/plugins`（P3/P4 按预期解除其余三条，非回退）；**缺计划要求的 E2E 守卫断言用例**，可选 toast 未实现。
- ⚠️ **P2-07**：shortcuts.xml/manifest/MainActivity/前端监听全链路齐备，但冷启动两处缺陷（见 §3 中级问题）。实现选了计划允许的备选（evaluateJavascript 注入）而非首选 Tauri event。

### P3 明细

- **P3-01**：插件依赖/注册/capabilities（新建 `capabilities/mobile.json`）/前端入口/「不存主密码明文」均 ✅；移动端分支已是真实实现但编译失败（§1.1 第二层）；**凭证存储用 `FileBiometricStorage` + HKDF(SHA256(account_id)) 作文件密钥（`biometric/legacy.rs:88-103`）——account_id 非秘密，加密实为混淆，未达到计划「Keystore 加密的 SharedPreferences」要求；未绑定 CryptoObject，硬件变更无失效回退**。
- **P3-02**：`NsdPlugin.kt:52-223` 完整、Rust 桥与注册齐备、snow/x25519 已全平台、协议层零分叉 ✅；**但同步链路未闭环（§3.1）**，编译失败（§1.1），权限申请缺失（§3 中级问题）。
- **P3-03**：锁定提醒（`useAutoLock.ts:58-62`，默认关）、备份提醒设置/解锁后检查/超期提醒均有 ✅；**但 toast 不可点击跳转（`uiStore.ts:3-9` 无 action）、生物识别/PIN 解锁路径绕过提醒检查（`LoginPage.tsx:297`）、`initLlmNotificationListener` 仍在启动即申请权限（`main.tsx:11` + `notification.ts:47-51`），与「首次触发点申请」相悖**。

### P4 明细

- **P4-01**：OCR 模型状态/下载/删除命令移动端已真实实现（`ocr.rs:571-586,722-732,761-772`），落盘 `BaseDirectory::Data/models/` ✅；embedding 命令共享化但 `fetch_registry` cfg 缺口（§1.1 第二层）；设置页存储占用/删除**仅桌面可见**（`OcrSettingsPage.tsx:130,162`），占用值为硬编码 '30MB' 非实测；**断点续传/失败重试未实现**（`ocr.rs:778-818` 无 Range/重试，`embed_model.rs:151-153` 删半成品重下）。
- **P4-02**：主线 B（ML Kit）——`MobileOcrPlugin.kt`（`text-recognition-chinese:16.0.1`）+ Rust 桥 `mobile_ocr_plugin.rs`，`ocr_scan_image` 移动端路由到插件（`ocr.rs:382-391`）✅；路由守卫已解除 ✅；embedding 降级为关键词搜索符合预期 ✅。**但 spike 报告未归档（docs/android/ 无任何 ort/NNAPI 可行性记录）；OcrPage 零移动端适配且 PDF 过滤器仍在（`OcrPage.tsx:107-108`，ML Kit 无法处理 PDF 必失败）；移动端扫描无 vault 解锁检查、无审计日志（桌面 `ocr.rs:329,370` 都有）；`mobile_ocr_plugin.rs:127` 同步 command 阻塞 tokio runtime 线程；`MobileOcrPlugin.kt:42-44` 每次新建 recognizer 不 close、置信度恒 1.0 占位；`LlmConfigPage.tsx:118-141` embedding 面板移动端无降级守卫**。
- **P4-03**（计划外）：`isShrinkResources`/`isCrunchPngs`/`.so` 去符号/META-INF 排除 + R8 fullMode ✅；ProGuard keep 规则齐全（tauri/JNI/ML Kit/注解/枚举）✅——**注意原 proguard-rules.pro 是空模板，本次实为必要修复**；commit message「移除过宽的全局 keep」与事实不符；因 §1.1 从未经 release 构建验证；`keepDebugSymbols.clear()` 后 native 崩溃无法符号化且无符号上传机制。

### P5-01 明细

- ✅ `gen/apple/` 33 文件被 git 跟踪；`package.json:15-17` 有 `tauri:ios:*` 脚本；`tauri.ios.conf.json` 与 Android overlay 一致不含模型；`pr_check.yml:41-66` 有 macos runner 的 iOS `cargo check`；无 iOS 专属 cfg 分叉（沿用 `any(android, ios)`）。
- ❌ 但 iOS check 在 master 实际失败（§1.1）；gen/apple 无构建产物、`assets/` 为空（`tauri ios dev` 从未跑到拷贝前端资源阶段）——**「模拟器启动 + 修复首批编译问题」与验收闭环（创建账户→解锁→列表）均无证据**；iOS 工程定制点文档未写。
- ⏸️ **P5-02 / P5-03 / P5-04 暂缓**：暂无 iOS 版本发布计划（已在 `docs/mobile/mobile-development-plan.md` §1/§7/§8 标注）。

---

## 3. 其余潜在问题（按严重度）

### 高

**3.1 同步功能链路未闭环（P3-02）** —— NSD 插件本身完整，但三处接线全断：

- Android 端**从不广播自己**：`commands/discovery.rs:143-144` 的 mobile `mdns_advertise` 仍是桩；`nsd_plugin.rs:79 register_service` 全库无调用方 → 桌面无法发现 Android。
- 前端**从不调用 `mdns_discover`**（`tauri/src` grep 无结果）→ NSD 发现结果不进 UI；mobile `sync_discover` 只回持久化 peers（`sync.rs:117-138`）。
- 移动端监听端口**无任何命令暴露**（`mobile.rs` 无 port getter，`enable()` 丢弃 `start()` 返回值 :54）→ 连手动输地址都拿不到端口。
- mobile `mdns_discover` 逻辑缺陷：`start_discovery()` 后**立即** `get_discovered_services()`（`discovery.rs:101-103`），NSD 解析是异步回调，首次调用几乎必为空；`timeout_ms` 被忽略。

**3.2 生物识别凭证无硬件级保护（P3-01）** —— 见 P3 明细。攻击者拿到应用数据（root/备份）即可由 account_id 推导文件密钥解密主密钥。须在文档/设置页如实降级声明当前保护级别，或迁移 Keystore + `setInvalidatedByBiometricEnrollment(true)`。

### 中

- **NSD 权限**：`NsdPlugin.kt` 无 `checkPermissions/requestPermissions`；`NEARBY_WIFI_DEVICES`（API 33+ dangerous）缺 `android:usesPermissionFlags="neverForLocation"`（计划明确要求）；`ACCESS_FINE_LOCATION`（≤32）未申请；未持有 `WifiManager.MulticastLock`（部分设备收不到 mDNS 组播）。「权限拒绝时 UI 有明确提示」未实现。
- **P2-07 冷启动丢事件**：前端 `AppRoutes.tsx:280-286` 消费 pending 时**先无条件 removeItem 再判 isAuthenticated**——冷启动未登录时 pending 被吞；Kotlin 侧 `MainActivity.kt:41` WebView 未挂树即静默 return，且无 pending/重试。首次使用快捷方式的典型场景恰是断链路径。
- **移动端 OCR**：扫描不做 vault 解锁检查、不写审计日志（与桌面不对称）；OCR 四个模型文件下载无 checksum（embedding 有 SHA256），配合自填 `base_url` 有投毒风险；`resp.bytes()` 整文件读内存（medium 档 132MB）；`mobile_ocr_plugin.rs:127` + `ocr.rs:390` 同步 command 阻塞 tokio runtime 工作线程（应用 `spawn_blocking`）。
- **OcrPage 移动端**：PDF 过滤选项仍在（必失败弹错）；`LlmConfigPage.tsx:118-141` embedding 面板移动端可下载永远用不了的 23MB 模型。
- **E2E**：`mobile-smoke.spec.ts` 未限定 mobile project，`__MOCK_PLATFORM__='android'` 会泄漏进桌面 chromium project；`:23` 选择器的 `nav` 兜底在桌面侧边栏也会命中，存在假性通过风险；`:31` `settingsLink` 死变量。
- **P1-07 埋点语义**：T1 实现为 LoginPage mount 即打点（计划定义是首个输入框 focus）；T2 从应用启动起算、HomePage mount 即打点（计划定义是从解锁成功起算、等首个对象渲染完成）——数值系统性偏离定义，复测对比前需先修正口径。
- **P3-03**：toast 不支持 onClick/action（备份提醒无法一键跳 `/settings/backup`）；`handleBiometricUnlock`/PIN 解锁不触发备份提醒检查；`initLlmNotificationListener` 启动即申请通知权限（存量行为，与计划相悖）。
- **同步线程模型**：`mobile.rs:266-276` 连接处理线程在 `stop()` 时 `abort()` 对 blocking 任务无效，进行中的会话「停止」后仍跑完。

### 低

- versionCode 规则文档（`release_process.md:282` 示例 20512）与实现（`tauri.properties` 2005012）不一致；`GITHUB_RUN_NUMBER` 在 workflow 重建后重新计数的回退风险；本地构建恒 base+0 与 CI 倒挂。
- AAB 未按 `SoloSoul_<version>_android.aab` 重命名（`build-android.yml:135` 上传原始 `app-release.aab`）。
- `build-android.yml:116` keystore 解码无前置校验（secret 未配时错误不直观）；`build.gradle.kts:37-43` 只判 PATH 非空，密码变量缺失时构建期失败而非回退。
- Android 指纹映射为 `touchId`，前端显示 "Touch ID"（`biometric.rs:99-101`、`BiometricSection.tsx:47-54`）文案错配。
- `global.css:238-244` 的 44px 基线会把弹层内 22px 小按钮（如 `SearchPopover.module.css:85-86 .clearBtn`）撑大，紧凑布局可能变形，建议抽查视觉。
- `Dialog.module.css:46-51` `align-self: flex-start` 与 auto margin 并存，冗余。
- `NsdPlugin.kt:46` `resolveListener` 单字段多次 resolve 时被覆盖；`resolveService` 重载 API 34 已 deprecated；插件无前后台 lifecycle 处理。
- `commands/sync.rs:6` 未使用导入、`discovery.rs` mobile `daemon` 参数未使用（警告）。
- 预存问题（非本次引入）：`build.gradle.kts:31-33` abiFilters 对所有 buildType 生效，x86 模拟器 debug 包缺原生库，与注释声称的「x86 仅用于模拟器调试」矛盾。

---

## 4. 可优化点

1. **命名统一**：`AppearanceSettingsPage.tsx:52`、`SettingsPage.tsx:43`、`OcrSettingsPage.tsx:21` 局部变量仍叫 `isMobile`（值是平台判定），与 P0-04 目标相违；`main.tsx:15` 与 `:28` 重复 `initPlatform()` 可删一处。
2. **hover 残留**：21 个未包裹文件按「移动端可达性」排序补包 `@media (hover: hover)`，优先 `LlmChatPage/index.tsx`、`OcrQuickScanPopover.tsx`；`index.tsx:252-254` 的 `sidebar-action-btn` 是死 CSS 可删。
3. **测试补齐**（新代码几乎零覆盖）：`solosoul-sync/session.rs`（新提取 402 行）、`mobile.rs`、`nsd_plugin.rs`、`NsdPlugin.kt`、biometric 移动端分支（现有测试为 `cfg(all(test, desktop))`）、`checkBackupReminder`/通知回退/useAutoLock 通知分支；E2E 按计划补对象列表/详情/新建对象三条用例 + `/plugins` 守卫断言。
4. **P2-07 改走 Tauri event**：Rust 侧缓存事件待前端 listen 就绪后重放；并修复 pending 消费顺序（先判登录态再删）。
5. **下载统一**：OCR 下载对齐 embedding 的流式 + checksum + 失败清理半成品；补 Range 续传或指数退避重试；`is_model_installed` 避免半安装误判。
6. **OCR 插件**：复用单个 recognizer 实例并管理生命周期；PDF 入口移动端隐藏或明确提示；`mobile_ocr_plugin.rs:127` 未注册的 `#[tauri::command]` 属性可去。
7. **CI 健壮性**：keystore 解码加 `test -s` 前置校验；release job 改「artifact 存在才附加」容错；versionCode 基础值用脚本从 `tauri.conf.json` version 自动算出，消除手工漂移。
8. **perf 日志平台过滤**：`LoginPage.tsx:75`、`HomePage.tsx:178` 的 `[perf]` 输出在桌面端也会打印，可用 `isMobilePlatformSync()` 包一层。
9. **归档缺失文档**：P4-02 选型结论（即使是一段「未做 ort spike 直接选 ML Kit 的理由」）与 iOS 工程定制点，补入 `docs/android/`。
10. **commit message/注释准确性**：P4-03「移除过宽 keep」与 `build.gradle.kts:73` 注释均与事实不符，建议修正以免误导。

---

## 5. 建议修复顺序

1. **修编译**：`mobile.rs` 的 `NoiseKeys` 改单一 `pub use`（lib.rs:40 直接从 `noise` re-export）、`running` 改 `Arc<AtomicBool>`；`map_bio_error`/`bio_err`/`fetch_registry` 去掉 desktop-only cfg；清理未使用导入；跑通 `cargo ndk -t aarch64-linux-android check` 与 iOS check。
2. **修 CI**：`ci_cd.yml:256` 的 job 归属重构（并入或 `workflow_run`），恢复整条流水线；趁机关联修复 AAB 重命名与 artifact 容错。
3. **补同步闭环**：`sync_enable` 后调用 `register_service`（先给 mobile SyncService 加 `listen_port()` getter 并透出命令）；`mdns_discover` 改「start → 轮询直到 timeout」；前端 SyncPage 接发现列表；NSD 权限申请 + `neverForLocation` + MulticastLock。
4. **生物识别安全**：迁移 Android Keystore（AES key 加密凭证 + `setInvalidatedByBiometricEnrollment(true)`），或先行在文档/设置页如实降级声明保护级别。
5. **验收缺口**：P1-06 接入 CI 并补用例、P1-07 真机实测落盘、P2-06 E2E 断言、P2-07 冷启动修复、P4-02 spike 文档归档。
6. 其余中级问题按 §3 排序消化。

---

## 6. 附：本次审查对文档的修改

- `docs/mobile/mobile-development-plan.md`：§7 头部加状态说明；**MOB-P5-02 / P5-03 / P5-04 标记「⏸️ 暂缓：暂无 iOS 发布计划」**（§1 总览表与 §8 验收清单同步标注）。任务内容保留，待重启 iOS 计划时沿用。
- 本报告归档于 `docs/mobile/mobile-review-2026-07-17.md`。
