# FE2 Android API31 旧 WebView 与预览主题验收

2026-10-08。专用 SoloSoul_FE2_API31 / emulator-5588 / ARM64，实际 WebView 91.0.4472.114。这里区分旧判据通过、目测发现问题和补充严格判据后的新执行；不把浏览器模拟当作旧 WebView 证据。

## 已确认的问题与修复

1. 实际旧 WebView 缺少 Object.hasOwn，生产启动失败。入口增加按能力检测的兼容层；缺少 crypto.randomUUID 时仍使用系统 getRandomValues 生成 UUIDv4，绝不使用 Math.random。
2. 默认构建目标把宽度媒体查询转换成旧引擎不支持的范围语法，编辑器的手机固定操作区没有生效。Vite 最低目标改为 Chrome91 / Safari15，实际产物保留传统媒体查询。
3. 实测 color-mix 与 dvh 不支持。视口高度使用 100vh 起点及支持时的 100dvh；Android 材质从同一主题快照交付 RGB 通道，背景 alpha 仍由 CSS 控制。WebGL / Kotlin 的颜色保持最终 hex。
4. 浅色预览顶栏与系统栏图标模式不一致。Android 预览进入 / 退出均使用当前应用主题，顶部和底部安全区使用对应内容表面。
5. 第五批截图发现浅色照片查看器标题和图标仍为白色。补齐 Android 顶栏标题、计数与按钮前景；新增各项实际对比与最终合成像素检查。系统栏 flags 单独正确不足以证明标题可读。

## 构建和执行分组

| 分组 | 来源 | 结果与证据边界 |
| --- | --- | --- |
| attempt-1-supported | first manifests | 启动 Object.hasOwn 失败；保留失败及恢复记录 |
| attempt-2-supported | second manifests | 启动及首页通过，编辑器操作不可见；没有整批通过 |
| attempt-3-supported | third manifests | 暗色预览通过；浅色路由已更新但对象卡片未挂载，立即点击失败；改为等待实际目标可见且可命中 |
| attempt-api34-fallback | third manifests | 实际 8 秒备份提醒在截图时已经消失，严格断言失败；未延长真实通知时限 |
| native-api31-supported / fallback | fourth manifests | 各 1 test / 29 records / 25 PNG，退出 0；此时未要求预览系统栏或顶栏前景证据，不能证明后续修复 |
| native-api31-bars-supported / fallback | fifth manifests | 系统栏及原内容判据通过，各 1 test / 29 records / 25 PNG；浅色照片标题仍有目测缺陷，不能称最终视觉验收完成 |
| native-api34-bars-fallback | fifth manifests | 原生成功及 242 项目录、模糊、通知权限恢复通过；同样不包含顶栏前景的新判据 |
| native-api31-header-supported / fallback | sixth manifests | 两组各 1 test / 29 records / 25 PNG，退出 0；系统栏、顶栏前景对比及实际像素、原内容门槛均通过 |
| attempt-api34-header-fallback | sixth manifests | 相册截图时真实提醒已到期，严格失败；242 项数据、模糊和通知权限完整恢复 |
| native-api34-header-bulk-fallback | seventh manifests | 1 test / 29 records / 25 PNG，退出 0；相同生产包，仅测试像素读取改为批量 getPixels，不修改提醒、截图等待或门槛 |

第五批所有浅色状态栏和导航栏 light flags 为 true，深色均为 false。浅色文本预览截图确认系统图标可读，但照片查看器的白色标题仍与浅色顶栏混合，故继续修复。

## 前端检查

- 兼容层 / 主题 / 返回相关单测 74 项及类型检查通过；预览主题修复相关单测 77 项通过。准确源码阶段与日志保留，不称最后 CSS 更改的全量重跑。
- 初次跨端浏览器 110 项通过、2 项硬编码服务器 URL 失败；只修正 URL 来源为配置 baseURL 后，2 项复验通过。
- 最新预览及 Windows 顶栏浏览器 36 项通过、4 项相册检测选择器漏掉嵌套标题失败；修正为检测全部实际标题与按钮后，4 项复验通过。对比标准没有降低。
- 原生证据拒绝检查目前 24 项通过：真实历史结果在原判据下有效，但不能通过缺失系统栏 / 前景证据的新判据。

## 保护及剩余范围

每次原生驱动备份整个应用私有目录，结束后核对全部内容、符号链接、权限与根权限，并恢复系统模糊状态；API34 同时恢复原通知 grant / flags。公开证据仅复制合成账户的 JSON / XML / 日志 / PNG，私有 tar 保留在 /tmp，不入库。未操作 emulator-5580。

Android 调试构建之后原 Cargo.toml、已有插件 registry 和 tauri.properties 均按原字节恢复。源码 / 产物 manifest 每批独立保留；本轮规范更新或后续编译不得覆写历史执行来源。

这里不是全产品兼容性证明，也不证明 iOS、实体设备功耗 / 连续动画表现、真实多窗口与缺口或其他 IME。macOS 完整客户端的原生视觉与恢复仍待验，FE2 总计继续 8 完成、6 待验证。未提交、推送或发布。

## 已接受的 API31 顶栏截图

浅色照片查看器底色 rgb(253,252,249)，标题及按钮 rgb(31,28,24)、对比 16.54；数量 rgb(122,114,101)、对比 4.63，实际像素分别 3168 / 140，按钮各 124～379 像素。状态栏和导航栏 light flags 均为 true。深色和两种 blur 模式按相同判据检查；浅色文本与照片截图人工确认可读。

API34 新增逐像素前景检查之后，在真实 8 秒提醒到期时拒绝通过。测试读取每个元素区域的像素改为一次 getPixels，再按原颜色误差统计，以减少 JNI 往返；并未重置提醒、关闭提醒或以更长时限替代生产行为。最终来源分开记录。

最终三组均通过完整新增判据与原内容检查，来源见 accepted-summary.json。API31 每组恢复全部 2 项私有内容与权限；API34 恢复全部 242 项及通知授权标记。API31 专用 AVD 已在最终恢复后关闭，身份核对及退出见 owned-emulator-stop.json；API34 测试过程中未操作 EntoKids 模拟器。

构建目标及入口兼容层的最后生产包启动 / Markdown 检查 1/1、退出 0（logs/production-startup.log）；该检查不代替移动端原生视觉或所有生产功能。当前 macOS 隔离包亦已重建，来源及无窗口校验另见 ../fe2-macos-compat-build-2026-10-08。
