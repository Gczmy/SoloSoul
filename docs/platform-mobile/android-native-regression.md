# Android 原生回归设备入口

## FE2 生产首页提交工作量（2026-10-08）

完整备份驱动支持 `--scenario rendering`。只在专用测试进程中观察生产装饰 canvas 的 WebGL 提交，分别采至少 1 秒的前台静止、离开首页、返回与返回后静止；要求 0 / 0 / 正向绘制 / 0，并检查返回后的真实合成文字像素。未安装探针、后台文档、样本过短或没有正向绘制均失败；finally 恢复 WebGL 原方法。此测试不能证明 GPU 完成耗时、FPS 或功耗。

当前 API34 两模式各 1/1、0 跳过、17 阶段 / 12 PNG；实际提交 0 / 0 / 4 / 0，全部私有数据与系统设置恢复。[来源、范围与原文](../verification/fe2-rendering-work-2026-10-08/README.md)。当前同一主包的两模式非空预览也独立复验：[预览证据](../verification/fe2-current-preview-2026-10-08/README.md)。测试 APK 不同，不能混用来源或计数。

## FE2 实际系统文字缩放入口（2026-10-08）

完整备份驱动可在 `--scenario baseline` 上增加 `--font-scale 1.0`、`1.3` 或 `2.0`。只允许显式专用 SoloSoul AVD，先保存原系统 `font_scale` 精确值（包括缺失状态），结束后恢复并复读核对；恢复失败不能通过。现有数据、blur、通知权限恢复保持原要求。此入口只改专用模拟器系统设置，不调用 WebView.setTextZoom、不注入网页字号。

六个真实首页阶段读取 Android configuration.fontScale、WebView.textZoom、资料库标题与数量的生产文字 Range，以及实际裁切祖先。Range 可能高于 line-height，不能将逻辑行框越界直接称为裁切；只在真实裁切祖先越界时失败。通过两份同运行环境报告比较匹配文本的实际高度，防止仅系统设置改变、正文没有放大的伪通过。全部结果必须与具体主包 / 测试包、执行源码和恢复回执一起记录；此范围不能代替全部页面、真机或键盘下放大字号验收。

API34 / WebView113、同一生产包已接受 1.0 / 1.3 / 2.0 三组。最终 2.0 仪器还采集顶栏三个按钮的实际图标像素、命中与标题避让；通过真实滑动检查长对象菜单全部四个操作及真实备份提醒。仅字体场景在详情截帧之前取证菜单，以保留生产提醒的原始 8 秒时限，报告必须声明 `fontMenuBeforeDetail`，判定必须匹配该顺序。1.0 / 1.3 旧仪器不含后来新增的图标像素检查，不能升级为此项通过。[各阶段来源、失败记录及范围](../verification/fe2-native-typography-2026-10-08/README.md)。

RF-310 使用 `tauri/scripts/android-native-regression.mjs`，要求显式提供专用模拟器 serial 和 AVD 名称；不会自动选择首个设备或连接用户手机。只使用 Debug APK，不读取发布签名密钥。不覆盖正式 macOS 客户端或真实账户。

## 构建与执行

在 `tauri/` 中使用已配置的 Node、rustup、Android SDK/NDK、JDK。ARM64 示例：

```bash
npx tauri android build --debug --target aarch64 --split-per-abi --apk --ci
cd src-tauri/gen/android
./gradlew tasks --all
./gradlew :app:assembleArm64DebugAndroidTest -x rustBuildArm64Debug
cd ../../..
node scripts/android-native-regression.mjs \
  --adb "$ANDROID_HOME/platform-tools/adb" \
  --serial emulator-5586 --avd SoloSoul_RF201 --mode supported \
  --apk src-tauri/gen/android/app/build/outputs/apk/arm64/debug/app-arm64-debug.apk \
  --test-apk src-tauri/gen/android/app/build/outputs/apk/androidTest/arm64/debug/app-arm64-debug-androidTest.apk \
  --output /tmp/solosoul-native-supported
```

再以 `--mode fallback` 和新的输出目录执行一次。输出目录必须不存在，避免覆盖证据。最低 API 31，ABI 为 ARM64 或 x86_64；本次 CI 固定 API34 / Google APIs / x86_64，支持与回退分成两个矩阵任务。x86_64 对应 `--target x86_64`、`assembleX86_64DebugAndroidTest` 和 x86_64 APK 路径。

## 原生断言与失败判定

- `supported`：实际系统 blur 必须可用；验证原生菜单选择、取消、暂停恢复、系统临时禁用 blur 时的可读性。
- `fallback`：设置系统 `disable_window_blurs=1`，实际能力必须不可用；验证插件返回 `unavailable`，不留下原生菜单，交由现有前端回退。此测试不把原生层拒绝打开误称为网页回退视觉验收。
- 两种环境均执行真实系统主题 light/dark 切换、显式 light/dark 偏好回归，以及资源安装和异步就绪回归。支持模式 13 项、回退模式 11 项，分别包含 1 项环境断言；不运行应跳过的另一模式用例。

每个方法独立 instrumentation 进程，并关闭框架自动清理 Activity；获得最终报告后由驱动 force-stop 测试应用，避免 WebView/Tauri 收尾使成功报告丢失。严格要求指定方法恰好 1 项通过；跳过、零测试、进程崩溃、设备不存在/离线、身份不符均 exit1。

`report.json` 记录设备 serial、AVD、API、ABI、APK SHA256、逐项耗时和结果；`junit.xml` 将设备初始化和收尾失败也表示为失败。保存 instrumentation 原文和失败截图，并拉取测试产生的玻璃/主题截图与 JSON。执行结束恢复原系统 blur 设置；主题用例自行恢复夜间模式和应用主题偏好。仅允许专用测试设备，测试中会修改该测试应用的资源和主题。

## CI 边界

`.github/workflows/build-android.yml` 提供 `android-native` 独立矩阵任务及手动入口，结果无论成功失败都上传。镜像由 [android-emulator-runner 官方配置](https://github.com/ReactiveCircus/android-emulator-runner) 启动，保留动画，Ubuntu 使用 KVM。完整工作流目前被维护者禁用；增加入口不会自动启用或触发远端任务。本地设备结果和远端 CI 结果分别记录，不将配置存在当作实际远端通过。

## RF-121 Card 原生补证（2026-10-02）

两组各新增独立 Debug WebView Card 检查：生产 Card/CardGrid、Android token 和平台 CSS，真实帧后检查浅深色溢出并在原生截图核对内部背景像素。专用 Activity 不初始化账户、禁用网络；Debug Gradle 自动生成内联资源，Release manifest 与任务图排除该入口。API34 ARM64 支持11/回退9均通过，10项Kotlin单元检查通过。[完整证据](../verification/rf121-android-native-checkpoint-2026-10-02.json)。默认对比度与正常动画不代替辅助功能、完整文字像素或多版本验收。

## FE2 原生玻璃绘制入口（2026-10-07）

两组均增加 `sharedThemeLiquidRendersAndRecoversWithoutContinuousDrawing`。独立 Debug 夹具直接载入生产 `androidLiquidRenderer`、主题派生与 Android CSS，不加载 AndroidHome 的业务数据、账户 Store 或正式初始化。文字固定为公开合成值；它只证明渲染器与当前样式在 Android WebView 的兼容，不能替代完整首页、账户生命周期或实体 GPU 性能验收。

检查最终 GPU uniforms 与应用主题数值一致、GL 实际像素不透明且无错误、装饰边界不覆盖文字、原生截图文字区域保持实色、静止 / 减少动效不持续重绘，以及真实 `WEBGL_lose_context` 丢失 / 恢复时 CSS fallback 和生产渲染器重建。缺失绘制、WebGL 不可用、缺失 context-loss 能力、uniform 错误或超时均失败，不接受跳过。独立类型校验使用已有 `native-regression/card-surfaces/tsconfig.json`，来源白名单只额外允许无账户依赖的生产渲染器。

本次执行结果记录于[FE2 检查点](../verification/fe2-frontend-checkpoint-2026-10-07.json)，历史 RF-121 的 11/9 数量保持原样。`fe2-liquid-*.png` 与 `fe2-android-liquid.json` 由驱动和其余证据一起采集；支持和回退运行需新的证据目录。

## FE2 保存主题的冷启动回归（2026-10-07）

两组增加 `savedSchemesAndCustomAccentSurviveColdStartup`，使用真实 MainActivity、生产前端和 `ui_get_preferences`，核对 `clean-slate` / `forest-night` 及自定义 `#112233` 的背景、表面、文字、强调色、派生材质文本和原生系统栏图标。系统按 light → dark → light 切换；没有创建账户或替换 IPC。用于防止 Rust 返回结构丢弃已保存底色方案的回归。

每个主题用例均明确写入待测的浅 / 深底色方案；默认用例写入共享暖石，避免继承上一例的非默认启动缓存。就绪必须同时满足完整颜色、挂载、启动层消失、系统媒体查询及原生栏图标，不能因 `data-theme` 相同就接受旧缓存帧；颜色不匹配仍在有界等待结束后失败。原 UI 偏好文件字节及系统夜间模式由 finally 恢复；WebView 启动缓存属于专用模拟器产生的测试状态，没有声称恢复全部沙箱内容。历史 12/10 渲染器补证仍是当时结果，当前两组 13/11 的退出码与构建来源见 FE2 检查点。

## FE2 真实首页账户入口（2026-10-07）

`AndroidHomeInstrumentedTest#realHomePreservesGlassAndCopyAcrossLockUnlock` 是独立补充方法，不增加上述通用矩阵的 13/11 计数。它通过生产 UI 创建公开临时账户，切换局部 / 增强和浅 / 深外观，锁定后通过主密码 UI 解锁，并用真实 IPC 核对账户外观偏好。仅支持已有、明确指定且没有账户的专用模拟器。

必须使用完整备份驱动，不能直接运行方法。根目录示例：

```bash
python3 tauri/scripts/android-home-native-regression.py \
  --adb "$ANDROID_HOME/platform-tools/adb" \
  --serial emulator-5586 --avd SoloSoul_RF201 --mode supported \
  --apk tauri/src-tauri/gen/android/app/build/outputs/apk/arm64/debug/app-arm64-debug.apk \
  --test-apk tauri/src-tauri/gen/android/app/build/outputs/apk/androidTest/arm64/debug/app-arm64-debug-androidTest.apk \
  --output /tmp/solosoul-home-supported-new
```

以新的目录再执行 `--mode fallback`。驱动在安装和启动前备份全部私有目录，拒绝非空账户清单 / 孤立账户目录；finally 停止进程，使用完整落盘 tar 恢复后比较全部文件内容、链接和权限。备份不得上传或入库。私有目录恢复失败也独立尝试恢复系统 blur；任一恢复失败不能输出通过。

首页 DOM、画布就绪和文字存在均不是截图已绘制的充分条件。截图先等待生产 WebView 视觉提交，再检查资料库正文区域的文字像素，保存最终系统合成帧。引导初始化和解锁只对明确的维护忙错误允许有限 UI 重试，保留次数和提示；错误类型不符、持续繁忙、解锁后没有首页、缺失保存偏好或跳过均失败。当前两组各 1/1 通过且完整恢复，均发生一次解锁维护忙后第二次成功，不能作为首次解锁无错误的证据。

这些结果覆盖真实空账户首页与基本锁定解锁，不覆盖有数据的对象详情、账户切换或实体设备性能。[公开帧与报告](https://github.com/Gczmy/SoloSoul/blob/75f1c1594504dd6e09af57ff6debd25cccce03fa/docs/verification/fe2-native-home-2026-10-07/fifth-supported/device-files/report.json)、[全部来源与失败历史](../verification/fe2-frontend-checkpoint-2026-10-07.json)。

## FE2 有数据的对象与系统返回（2026-10-07）

上述首页方法现扩展为 12 个阶段：原 5 阶段之后，通过真实新建菜单选择 Travel / Visa，填入公开对象名称和公开 Country 字段并保存，检查列表、详情、操作菜单和实际 Android 系统返回，最后检查数量为 1 的增强浅 / 深首页。两组各 1/1、退出 0、跳过 0；各 11 张最终合成帧。驱动命令保持不变，结果必须用新的输出目录。

生产 MainActivity 现在桥接系统返回到前端 Router 历史；原有浮层守卫仍负责分层关闭。当前本地 SPA 的 `WebView.canGoBack()` 不能作为可返回判断。验收只覆盖对象详情及操作菜单返回保留当前分类；根页面、键盘和完整导航矩阵单独补验。

原生通知权限弹窗只按已识别的 SoloSoul 通知消息和拒绝按钮操作；原 grant 与用户权限标记先记录，finally 独立恢复并核对。私有根模式核对和远端归档收尾完成后才设置恢复成功，任何恢复错误均拒绝通过。真实截图要求 WebView 提交当前视觉状态；底部弹层等待入场动画结束再核对边界，不放宽几何要求。

当前截图也确认备份 Toast 遮挡菜单下方操作与编辑页保存区，该布局问题仍需修正。12 阶段通过不代表通知遮挡、账户切换、实体设备性能或其他平台已通过。[当前公开结果](https://github.com/Gczmy/SoloSoul/blob/75f1c1594504dd6e09af57ff6debd25cccce03fa/docs/verification/fe2-native-populated-2026-10-07/seventh-fallback/report.json)、[系统返回后的列表帧](https://github.com/Gczmy/SoloSoul/blob/75f1c1594504dd6e09af57ff6debd25cccce03fa/docs/verification/fe2-native-populated-2026-10-07/seventh-fallback/device-files/object-list-after-system-back.png)、[来源与失败历史](../verification/fe2-frontend-checkpoint-2026-10-07.json)。

### FE2 通知占位的原生补验

当前 AndroidHome 流程的编辑与对象菜单阶段要求真实备份 Toast 仍存在且仅有一份；核对通知使用 static 正常流、菜单通知属于非 inert 菜单，并逐项检查 Save / Edit / History / Attachments / Delete 的视口边界、通知矩形不相交与中心点命中。测试不得通过关闭提醒或等待过期来满足遮挡断言。系统通知权限树可有 3 秒有界就绪等待，仅完整识别 SoloSoul 通知请求后拒绝；未知或持续不完整的权限界面失败。

当前新 APK 的首轮因权限树识别失败，未得到上述阶段通过证据；数据 242 项、模糊及通知授权 / 标记均恢复。重建后第二次通过权限阶段，但编辑采样点没有 Toast，被提醒存在断言拒绝；两次均恢复数据、blur 和通知授权 / 标记。实际提醒的触发 / 采样时序仍待核对，不能拿无提醒帧证明不遮挡。结果单列本轮通知检查点，不覆盖此前旧 APK 的原生通过历史。

## 2026-10-07 备份 Toast 存在期间的原生补证

先前两次未取得有效提醒帧的失败仍保留。只读 DOM 观察记录确认提醒在 `/editor` 等待期间出现；当前方法等待实际 `Back Up Now` 按钮和通知正常流后采样，截图后确认提醒未消失。没有缩短 / 延长提醒时长或通过关闭绕过检查。

专用 API34 ARM64 `SoloSoul_RF201`，同一当前主包及新测试包：系统 blur 支持 **1/1**、关闭 **1/1**，退出 0、跳过 0，各 12 阶段 / 11 PNG。真实提醒存在时 Save、Edit、History、Attachments、Delete 均可见、可命中且无重叠；菜单通知在 sheet 内。两组截图已目测。242 项私有目录内容 / 链接 / 权限、根权限、blur、通知授权及用户标记恢复。

公开证据：[支持模式](https://github.com/Gczmy/SoloSoul/blob/75f1c1594504dd6e09af57ff6debd25cccce03fa/docs/verification/fe2-toast-layout-2026-10-07/third-native-supported/report.json)、[关闭模式](https://github.com/Gczmy/SoloSoul/blob/75f1c1594504dd6e09af57ff6debd25cccce03fa/docs/verification/fe2-toast-layout-2026-10-07/fourth-native-fallback/report.json)，APK / 源码 SHA256 见[检查点](../verification/fe2-frontend-checkpoint-2026-10-07.json)。旧 APK 结果保持原来源；私有 tar 不入库。附件 / 历史 / 预览、账户切换、键盘和实体设备性能未由这两条通知路径证明，首次维护忙重试仍存在。

## 2026-10-07 浮层通知扩展后的新包基线

新 ARM64 Debug 主包 `65933cf9e289c0e788e36c4a1a1095a20b109d1585015e4b2326e77365c05121` 包含附件 / 历史 / 预览通知扩展。使用同一 instrumentation，在专用 API34 ARM64 支持 / 系统 blur 关闭两组各 1/1、退出 0，12 阶段 / 11 PNG；242 项目录、权限、blur、通知授权 / 标记恢复。真实备份提醒存在期间编辑与对象菜单操作可见命中且未重叠，已有首页 / 返回未回归。

当前方法仍没有进入附件 / 历史 / 文件 / 相册 / 查看器的新增通知场景，这部分仅有两个浏览器项目各 35/35 的证据，原生需要继续扩展验证。两个阶段分别冻结 APK 与源码，不能把上一包结果称为本包或新增场景通过。[本包支持报告](https://github.com/Gczmy/SoloSoul/blob/75f1c1594504dd6e09af57ff6debd25cccce03fa/docs/verification/fe2-overlay-toast-2026-10-07/native-supported/report.json)、[关闭报告](https://github.com/Gczmy/SoloSoul/blob/75f1c1594504dd6e09af57ff6debd25cccce03fa/docs/verification/fe2-overlay-toast-2026-10-07/native-fallback/report.json)。

## 2026-10-07 附件与历史嵌套通知场景

完整备份驱动增加 `--scenario overlays`，使用同一真实首页方法，显式选择附件 / 历史的通知验收范围；默认 `--scenario baseline` 保留原编辑 / 对象菜单的提醒断言。两种场景必须使用不同的新输出目录。

新场景的 15 阶段包含：在对象详情打开附件 → 系统返回仅关附件 → 详情更多菜单进入历史 → 系统返回仅关历史 → 详情系统返回保留 Travel 分类。附件和历史在真实备份提醒存在时检查唯一容器、static 正常流、当前面板包含关系、实际 `5100` 层级、视口内与中心命中；附件 Upload / Close、历史 Close 不与提醒相交且可命中。截图后仍必须有真实备份动作，禁止改写提醒计时或注入合成通知。对象菜单随后不再重复要求同一提醒，该路径的通知证据取自独立 baseline，不混淆范围。

专用 API34 ARM64 支持 / 系统 blur 设置关闭两组各 **1/1**、退出 0、跳过 0，各 **15 阶段 / 13 PNG**，四张附件 / 历史最终帧已目测。主包仍为 `65933cf9e289c0e788e36c4a1a1095a20b109d1585015e4b2326e77365c05121`，新测试包 `67c7f116c34079b14d73a790fe9beefc83f40ba420b67a7896f6098dc918e468`。Python 驱动拒绝原 12 阶段报告或通知留在低层的报告，6 项检查通过。242 项完整目录、权限、blur 和通知授权 / 用户标记均恢复；私有 tar 不入库。

这些结果证明空附件列表与有真实快照的历史面板，不证明上传、文件 / 相册 / 查看器、键盘或账户切换。初始化和解锁仍出现一次维护忙后的 UI 重试。[支持报告](https://github.com/Gczmy/SoloSoul/blob/75f1c1594504dd6e09af57ff6debd25cccce03fa/docs/verification/fe2-nested-overlay-toast-2026-10-07/native-supported/report.json)、[关闭报告](https://github.com/Gczmy/SoloSoul/blob/75f1c1594504dd6e09af57ff6debd25cccce03fa/docs/verification/fe2-nested-overlay-toast-2026-10-07/native-fallback/report.json)，当前范围与源码来源见 FE2 检查点。

## 2026-10-07 真实 IME、提醒与键盘返回

同一完整备份驱动增加 `--scenario keyboard`，输出目录仍必须全新。依次真实触屏聚焦对象名称 → 检查原生 IME 高度 → 在真实备份提醒尚存在时采输入框 / Save → 通过 Android 键盘事件输入公开文本 → 实际系统返回 → 保留编辑草稿与路由且恢复 WebView 高度 → 保存并完成对象 / 首页流程。不可直接运行 instrumentation 绕过数据备份。

原缺陷为 IME 高 872px 而 WebView 仍高 1920px，网页不能正确获取键盘避让高度。MainActivity 按实际 IME 覆盖调整 WebView margin，并保留未消费的 insets；进入多窗口的系统边界适配参数同步纠正。测试检查控件完整底沿与真实键盘顶沿、中心和四条边内部命中、提醒唯一无覆盖、最终合成帧；禁止合成提醒、延长计时或仅接受晚期无提醒帧。

最终主包 `171aad74b2d85f9b18f9460d89615ebe461c2b9d23bb6b59f9de8ed6f4f3963f`、测试包 `e5fc3096308dac9d7d4c1752ffb3aea4ec48101fa596b3394ca4f0027f455efc`，API34 ARM64 两组各 **1/1**、退出 0、跳过 0、15 阶段 / 14 PNG。键盘显示时 WebView 为 1048px，关闭后恢复 1920px；实际输入后的草稿与 `/editor?section=travel` / idx=14 保留。两组 242 项数据、根权限、blur 和通知授权 / 标记完整恢复，Python 驱动 8 项检查通过。

提醒布局先于录入采样，录入和返回阶段独立核对真实草稿；没有削弱同屏提醒的原要求。新场景较晚的对象菜单不重复要求同一 8 秒提醒，菜单通知证明仍来自独立 baseline。前置菜单失败、圆角探针错误与提醒过期失败均独立保留。[最终支持结果](https://github.com/Gczmy/SoloSoul/blob/75f1c1594504dd6e09af57ff6debd25cccce03fa/docs/verification/fe2-keyboard-2026-10-07/final-native-supported/report.json)、[最终关闭结果](https://github.com/Gczmy/SoloSoul/blob/75f1c1594504dd6e09af57ff6debd25cccce03fa/docs/verification/fe2-keyboard-2026-10-07/final-native-fallback/report.json)、[来源说明](../verification/fe2-keyboard-2026-10-07/README.md)。旧系统、实际多窗口、键盘动画、登录键盘、其他 IME、账户切换与实体设备性能尚未验证。

## 2026-10-07 两账户外观隔离与无填色操作对比

同一备份驱动新增 `--scenario accounts`，需全新输出目录。实际创建 A / B、锁定、密码选择并反复解锁；A 森林深色 / 增强玻璃 / 深强调色 / 1 对象，B 冷灰浅色 / 局部玻璃 / 黄强调色 / 0 对象。只读 IPC 检查当前账户保存值，原生 WindowCompat 检查系统栏模式；截图必须有当前正文与操作文字像素。首次仅验证正文的两组通过因截图发现低对比操作文字，不算完整视觉验收，旧报告原样保留。

修正无填色操作文字后，新主包 `4d186038fe6d4eaab748f093fe5f544bfce0b5187ecda3da40a3f268d6cf158e` / 测试包 `54ad522c3c7e327813e75836e636d435093350cfc36b92a514949b18e761c4ea`，支持 / 关闭系统 blur 两组各 1/1、退出 0、跳过 0、18 阶段 / 16 PNG。实际 “View all” 对比 5.26 / 4.53、文字像素 999 / 969、可见命中；账户偏好与对象数量未串用。两组全部 242 项数据和权限 / blur / 通知授权恢复；最终 Python 11 项通过，基于真实通过报告验证拒绝低对比等伪通过条件。

初始目录和后续解锁仍有 maintenance 提示后的有界 UI 重试。此次系统栏只验证颜色模式，状态栏安全区需单独几何检查。冷重启 / 系统主题、预览、登录 IME、多窗口与设备性能不在本场景内；旧键盘结果仍对应自己的 APK，不冒充本包键盘验收。[完整来源与截图](../verification/fe2-accounts-2026-10-07/README.md)。

## 2026-10-08 系统安全区、登录 IME 与新文档

完整备份驱动新增 `--scenario insets`，沿用全新输出目录与专用 SoloSoul 模拟器约束；不得绕过备份直接运行 instrumentation。场景在原 12 阶段基础上增加首页竖横竖、真实锁定登录、触屏 Gboard、系统返回关闭键盘及实际 WebView.reload，共 19 阶段 / 18 PNG。排除业务明确隐藏或 inert 项后，可见品牌 / AppBar / 四个导航及登录输入 / 提交按钮必须完整位于原生 systemBars | displayCutout / IME 安全边界中，并中心可命中。外部驱动独立重算物理边界，拒绝旧 baseline、越界、缺 IME、丢历史和重载高度未恢复的报告；Python 13 项通过。

最终 API34 ARM64 系统模糊支持 / 关闭两组各 1/1、退出 0、跳过 0，同一当前主 / 测试 APK。键盘下解锁按钮底沿约 969.73px，键盘顶沿 1048px；返回恢复 1920px 与 Router idx=19，重载后的登录安全区正确。全部 242 项私有内容、链接、权限和系统设置恢复。初始化 / 解锁仍有维护忙重试；旧 WebView、不为零的真实侧边缺口、多窗口和其他 IME 没有被本场景证明。完整来源、失败、检查日志及截图见[安全区证据](../verification/fe2-safe-area-2026-10-07/README.md)。

## 2026-10-08 账户外观与 Activity/WebView 重建

驱动新增 `--scenario lifecycle`，复用真实创建 / 保存 / 切换两个公开账户的流程。A 森林深色 / 增强玻璃 / 深强调色 / 1 对象，B 冷灰浅色 / 局部玻璃 / 黄强调色 / 0 对象。系统与显式偏好相反时继续采用账户偏好；每个账户执行实际 `ActivityScenario.recreate()`，断言 Activity 和 WebView 均更换实例，重建后需要时通过密码 UI 解锁。检查真实保存偏好、身份 / 数量、原生 Configuration 和系统栏模式、画布分区、最终正文及操作文字像素，不注入主题或认证 Store。

同一生产主 APK `f82370125a99ce46260a2cecea7fed7396fc60d55a30be498ccc052b75341c13`，新测试 APK `cb25578653e57668daeaa6d21b814df95928a079cc43a7888adf63b70cf4a343`。API34 ARM64 支持 / 关闭系统 blur 两组各 1/1、退出 0、0 跳过、22 阶段 / 20 PNG。操作对比 A 5.259 / B 4.529，像素 999 / 969；两次原生重建后的配色、数据与材质正确。每组完整恢复 242 项私有数据、根权限、blur、通知授权 / 标记及原系统夜间模式；私有 tar 不入库。最终 Python 驱动 16/16，基于实际报告拒绝缺重建、错原生主题、账户串用或无文字像素；收紧后的判定重新核对两组，未冒称重跑设备流程。

这里只重建 instrumentation，未重新构建生产前端 / JNI。实际执行与后续判定来源分别冻结；这不是 force-stop 后的进程冷启动，也不代表所有系统模式、真实多窗口或实体性能。初始化与解锁仍有维护忙的有界 UI 重试，未宣称修复。公开截图、日志、来源及未验范围见[生命周期阶段证据](../verification/fe2-lifecycle-2026-10-08/README.md)。

## 2026-10-08 真实进程冷启动

完整备份驱动新增 `--scenario cold`。先以真实 UI 完成 18 阶段双账户准备，并只读等待 A 的五个外观键已保存到登录前镜像；结束原进程后执行第二个 instrumentation 方法，断言新 PID、两份原账户身份和真实锁定登录，不注入认证或主题。再通过密码 UI 验证 A 在系统浅 / 深色下的显式深色增强玻璃，以及 B 在系统深色下的显式浅色局部玻璃，共增加 5 阶段 / 4 PNG。

```bash
python3 tauri/scripts/android-home-native-regression.py \
  --adb "$ANDROID_HOME/platform-tools/adb" \
  --serial emulator-5586 --avd SoloSoul_RF201 --mode supported --scenario cold \
  --apk tauri/src-tauri/gen/android/app/build/outputs/apk/arm64/debug/app-arm64-debug.apk \
  --test-apk tauri/src-tauri/gen/android/app/build/outputs/apk/androidTest/arm64/debug/app-arm64-debug-androidTest.apk \
  --output /tmp/solosoul-home-cold-supported-new
```

`--mode fallback` 使用另一个全新目录。原进程在 instrumentation 后已正常退出可以接受；未知 / 不同 / 多 PID 拒绝操作。两个方法必须各精确 1 项成功、无跳过；缺冷启动阶段、错主题 / 密码区 / 账户数、无文字像素、偏好串用或任何数据 / 系统恢复失败均拒绝通过。备份 tar 只留本机，不入库。

最终当前主包 `5158c440cec5a7168278a414298cce15cda80d7104c1354949fa3aff5ea00b6e`、测试包 `f5545eea9a1a4168a1406dd84e196a437df3b5d0f28505946fa2c2c43baca335`，API34 ARM64 两模式各 2/2、退出 0、0 跳过、23 阶段 / 20 PNG；新旧 PID 分别为 10205→10662、11016→11450。每组 242 项私有数据与权限 / blur / 通知授权标记 / 系统模式恢复。完整来源及三次失败见[冷启动证据](../verification/fe2-cold-2026-10-08/README.md)。

生产修复仅恢复已有私有账户根、避免路径别名误删 UI 配置、更新账户启动缓存，并同步偏好读写锁。测试等待真实保存完成，证明稳定帧恢复，不代表写入时崩溃或连续首帧无闪烁；旧 WebView、多窗口、非空附件预览、维护忙首次交互、实体性能仍待验。

## 2026-10-08 减少动态效果跨账户与进程验证

在 `--scenario cold` 上追加 `--motion-preferences`，沿用专用模拟器、全目录备份和新输出目录。A / B 必须通过实际复选框显式保存 true / false，连同原五个外观键共六个偏好；第一阶段等待实际镜像持久化，再停止原进程。第二方法须新 PID、锁定登录 true、密码解锁后 A true / B false，文档标记与保存值一致；A 的导航计算过渡时间不超过 1ms。色板、画布可见性、文字像素、命中与完整恢复门槛不变。旧报告不包含此扩展，不能当作动效通过。

最终当前 API34 ARM64 支持 / 关闭 blur 两组各 2/2、0 跳过、退出 0、23 阶段 / 20 PNG，242 项数据与系统恢复。首轮采样不足导致画布不可见的失败保留，测试现等待实际可见再采帧；20 项驱动拒绝检查通过。执行源码与后续仅新增测试的源码分别冻结，不重新标记旧 APK。详见[公开动效报告](../verification/fe2-motion-2026-10-08/README.md)。这是偏好及 CSS 交付 / 稳定帧证据，不是连续动效、全控件或实体性能验收。

## 2026-10-08 非空附件与实际预览

驱动增加 `--scenario previews`，沿用明确指定的专用 AVD、完整私有数据备份和全新输出目录。真实 UI 创建公开账户 / 对象，再用生产 FileProvider 与真实附件 IPC 导入缓存中的 TXT / PNG，核对两份 SOLC 加密附件；这不是系统文件选择器或 Upload UI 验收。深浅主题分别打开附件、文本 / 图片预览、照片集和照片查看器，实际属性编辑保存产生 Saved 通知；深色初始附件 / 文件 / 相册使用真实限时备份提醒。检查最终文字像素、原图四象限、缩略图比例、缩放工具、通知占位与命中、系统安全区及返回后对象详情 / 状态栏。文件预览用界面返回；查看器→相册→附件→详情用 Android 返回键。

同一 API34 ARM64、同一主 / 测试 APK，系统模糊开启 / 关闭各 **1/1、0 跳过、退出 0、29 阶段 / 25 PNG**。每组 242 项数据与根权限 / blur / 通知授权标记完整恢复。七次失败按实际终态保留；最终只调整测试可见图像选择、稳定几何与截帧时机，未改生产材质或通知计时。执行来源与后续严格判定分开冻结；Python **22/22**，两组原始报告经收紧后的判定通过，没有冒称重跑设备或生产构建。

截图、日志、命令与未验范围见[预览阶段证据](../verification/fe2-preview-2026-10-08/README.md)。多图翻页、PDF / 外部阅读器、选择器、大附件、连续动画、旧 WebView、真实多窗口 / 缺口 / 其他 IME、维护忙首次 UX、iOS 和实体性能仍待验。

## 2026-10-08 API31 旧引擎与预览栏前景检查

原生驱动允许另一个明确拥有的 SoloSoul_FE2_API31 / emulator-5588，实际 Android12 / WebView91.0.4472.114 / ARM64；不能使用未指定或非 SoloSoul 的模拟器。沿用整个私有目录 / 根权限 / 系统状态备份恢复及全新输出目录。

`--scenario previews` 当前额外要求所有非附件预览的顶栏标题、数量与按钮对比至少 4.5:1，最终帧对应元素至少有 6 个目标色像素；原生浅色系统状态栏和导航栏 flags 为 true、深色均为 false。原图 / 文本、真实限时提醒、48px 命中、安全区和逐层返回判据均保留。`verify_evidence` 的旧判据默认仅用于重核历史记录，实际新执行强制启用系统栏及前景两项扩展，不能把历史成功重新标成新增范围通过。

新的公开记录见[API31 与预览栏证据](../verification/fe2-api31-2026-10-08/README.md)，包含每批 APK / JNI / 源码 hash、实际 WebView 版本与能力、原始失败以及后续截图审查发现的白色照片标题问题。这里只覆盖该次生产 UI 路径；未覆盖能力仍按执行台账保留。

## 2026-10-08 API31 横屏通知安全区

旧 WebView91 的导航、登录 / IME、旋转与重载已在系统模糊开启 / 关闭两组验证；实际横屏右侧系统栏 126px，通知槽补横向安全区。规则限定已登录 Android 壳，认证壳与平板轨道已占用的安全区不再次叠加。原生 insets 现强制实际限时备份提醒及操作 / 关闭按钮安全、可点击；没有延长生产提醒。

最终两组各 1/1、19 阶段 / 18 PNG、0 跳过、退出 0，数据 / 权限 / blur 完整恢复。浏览器定向复验 4/4，Python 拒绝检查 26/26；历史范围缺少通知证据不能升级为新范围通过。新增 `verify_evidence(..., require_safe_notifications=True)` 仅用于要求新范围，当前设备驱动对 insets 强制开启；旧报告的原结论保留。浮点相邻边界误判与浏览器夹具失败、最终包 / 源码来源均见 [证据目录](../verification/fe2-api31-insets-2026-10-08/README.md)。不代表旧引擎全产品、实体性能、iOS 或 macOS 原生已完成。
