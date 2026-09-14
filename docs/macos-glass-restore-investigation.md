# macOS 恢复窗口时玻璃背景闪黑：修复调研

调研日期：2026-09-14。目标：保留原生 Liquid Glass、实时桌面背景采样、正常 Dock / 台前调度行为，同时消除恢复窗口过程中的黑帧。

截至本次检索，已找到高度吻合的公开 issue，但未找到满足上述全部条件、经过验证的现成完整修复。后续本机实验确认：给原生窗口保留 alpha=0.001 的微量白色底色后，用户观察到不再闪黑且玻璃正常；现已接入 SoloSoul，集成验收进展见文末。具体系统内部故障点仍未确认。

## 本机证据

macOS 26.6；用户已重新编译、重启 SoloSoul，Dock 和台前调度两种恢复方式均能触发。独立 AppKit 对照程序不包含 Tauri、WebKit 或业务页面，焦点回调只记录事件：

| 材质 | 用户观察 |
| --- | --- |
| `NSGlassEffectView` | 玻璃 → 纯黑 → 玻璃 |
| `NSVisualEffectView`，Sidebar / BehindWindow / Active | 毛玻璃 → 灰黑 → 毛玻璃 |
| 真正不透明的 `NSWindow` | 不闪黑 |

对照源码：[macos-native-appearance.swift](/Users/zzc/PycharmProjects/SoloSoul/tauri/scripts/macos-native-appearance.swift)。这是人工观察结果，尚无逐帧统计。

因此，React、CSS、WKWebView 和材质随焦点改变都不是复现问题的必要条件。两种黑色深浅不同与材质合成结果有关，但不能仅凭颜色判断哪一层丢失了背景。原先的标题栏几何异常与瞬态黑帧也应分别验收：布局恢复正常不代表过渡帧正常。

## 已有 issue 与补丁

| 来源 | 核对状态 | 与本问题的关系 |
| --- | --- | --- |
| [Recrest #85](https://github.com/SoftVentures/Recrest/issues/85) | Open；2026-06-19 创建；当前 0 条回复 | 高度吻合：恢复时透明区域短暂变黑，随后恢复模糊；两种原生材质及 Active 状态均失败。作者目前接受闪烁，没有完整补丁。 |
| [Tauri #8255](https://github.com/tauri-apps/tauri/issues/8255) | Open；2023 年起 | Sonoma 上焦点切换后的透明窗口异常；主要是边框、阴影和透明状态问题，不能直接等同 Tahoe 瞬态黑帧。 |
| [window-vibrancy #112](https://github.com/tauri-apps/window-vibrancy/issues/112) | Closed | 边框与阴影缩成角落小矩形。关闭原因是转移到 Tauri，不是修复成功。见[关闭说明](https://github.com/tauri-apps/window-vibrancy/issues/112#issuecomment-1817684600)。 |
| [window-vibrancy #88](https://github.com/tauri-apps/window-vibrancy/issues/88) | Closed / not planned | 设置 Active 解决的是失焦后的材质外观变化；本机已用 Active 复现闪黑，不适合作为这次完整修复。 |
| [Kimbo Terminal PR #5](https://github.com/lucatescari/kimbo-terminal/pull/5) | 2026-04-29 合并 | 聚焦后重设透明、重建材质、切换阴影、微调尺寸以恢复卡住的透明状态。验证没有证明恢复全过程零黑帧，且采用传统毛玻璃。 |
| [cmux #5860](https://github.com/manaflow-ai/cmux/issues/5860) | Open | macOS 27 上玻璃 tint 导致持续黑色，移除 tint 的候选修复针对不同版本与症状；不能套用于本机 26.6 的短暂闪黑。 |

Recrest 把问题归类为系统限制，这是项目作者通过排除法得出的判断，不是 Apple 对“无法修复”的官方确认。它引用的 Apple 论坛 [800118](https://developer.apple.com/forums/thread/800118) 和 [810314](https://developer.apple.com/forums/thread/810314) 在本次访问时进入人机验证页面，未能核实正文及所谓 Apple 建议。

Apple 的 [macOS 26 发布说明](https://developer.apple.com/documentation/macos-release-notes/macos-26-release-notes) 提到启用“降低透明度”后后台窗口或 Dock 闪烁（152060485），但触发条件不同，也不是本问题已修复的证据。

## 最值得验证的机制：窗口图层自动扁平化

Oskar Groth 的[实现研究](https://oskargroth.com/blog/reverse-engineering-nsvisualeffectview)描述了自定义背景模糊在窗口闲置约一秒后失效、切换空间时出现异常的情况。其 MaterialView 通过私有窗口配置维持背景图层，提供了一条机制接近的实验路线。它使用自定义 `CABackdropLayer`，不能直接视为原生 `NSGlassEffectView` 的成功案例。

核对的 [MaterialView 固定版本源码](https://github.com/OskarGroth/MaterialView/blob/b3a04ce56eca69d388c48e9621ce61778fe976c7/Sources/MaterialView/NSMaterialView.swift#L1088)包括：

- 关闭 `shouldAutoFlattenLayerTree`；源码注释记录约 1.05 秒的延迟。这一时间来自该实现的观察，不是 Apple 保证，也未在本机测得。
- 切换 `canHostLayersInWindowServer` 以重建托管图层，源码明确提示操作昂贵。
- 给窗口背景极小的非零 alpha（0.001），其解释是维持系统圆角遮罩；不等于已证明能消除黑帧。
- 用 `CGSSetWindowTags` 干预空间切换时的扁平化。

这一机制能解释约一秒后发生变化的可能性，但时间相似不足以确认因果。尤其不能把一组私有参数同时加入 SoloSoul 后，仅凭不再出现持久黑块就宣布完整修复。

私有标志也存在版本差异：MaterialView 当前代码使用第二个 32 位字的 `1 << 16`，并注明 Big Sur；其参考实现 [BackdropView](https://github.com/avaidyam/BackdropView) 使用 `1 << 23`，注释仍有问号。本次未验证这些位在 macOS 26.6 的含义，不应直接复制数字。

## 建议的验证顺序

以下为待执行实验，不是已经验证的修复。

1. **继续使用独立 AppKit 程序，保留同一个原生玻璃视图。** 各实验独立启动，避免上一次模式的窗口内部状态干扰结果。先分别测试极小非零背景 alpha、关闭窗口阴影；每次只改变一个变量。前者对应圆角遮罩线索，后者有[修复轮廓残影的用户报告](https://github.com/tauri-apps/tauri/issues/8255#issuecomment-2908619653)，均未证明能消除本机黑帧。
2. **测试初始化时关闭图层自动扁平化。** 在首次显示前配置，保留原生 `NSGlassEffectView`。私有 selector 必须先探测是否可用；不支持时退出该实验。若初始配置无效，再单独加入一次图层重新托管，比较结果，不在每次聚焦时循环重建。
3. **若只剩空间切换时闪黑，再研究对应系统版本的窗口标志。** 先确认含义和生命周期，避免把旧系统位值作为永久产品代码。
4. **原生程序通过后再移植到 Tauri。** 保持窗口、材质、WebView 身份及布局稳定，检查启动、主题切换、前后台事件是否触发不必要的透明度或材质重置。

第 1 步成本低；第 2 步是目前最值得验证的机制线索。若只能通过自定义 MaterialView 成功，则只能算“保留实时毛玻璃”，仍需说明不再是相同的原生液态玻璃效果。

另可把台前调度关闭作为诊断对照：通过 Dock 点击也可能处于台前调度管理之下，不能排除其参与。社区的 [Accessory 绕过方式](https://github.com/tauri-apps/tauri/issues/8255#issuecomment-2198004346)会让应用[不出现在 Dock](https://github.com/tauri-apps/tauri/issues/8255#issuecomment-2224060542)，因此不满足正常 Dock 行为这一完整目标。

## 验收条件

建议以可重复的操作矩阵验收，而非只检查最终截图或布局断言：

- Dock、台前调度、Cmd-Tab、隐藏后恢复、最小化后恢复分别重复至少 30 次；覆盖短暂失焦和后台停留超过 2 秒。
- 逐帧观察透明区、顶部边缘与交通灯区域；同时保留人工观察，因为录屏可能改变合成行为。
- 确认玻璃仍实时采样移动的背景，避免“冻结上一帧”被误认为恢复正常。
- 检查调整尺寸、全屏、切换空间、浅深主题与睡眠唤醒；显示器条件允许时覆盖内外屏。
- 若采用私有窗口配置，对比空闲和切换时的 WindowServer CPU / GPU 占用，并保留版本检测与回退路径。

焦点后临时覆盖颜色、延迟恢复材质、每次重建玻璃或微调窗口尺寸，都只能作为缓解候选；没有逐帧证据时不能承诺彻底不闪。有限次测试通过也只证明已测环境，不保证所有 macOS 版本。

调研阶段未把私有 API 候选方案写入生产实现。已存在的不透明兼容模式是本机有效的退路，但不满足本次“保留玻璃”的目标。

## 修复实验进展（2026-09-14）

已在独立 AppKit 对照程序中加入六种配置。每次更换实验或材质都创建新的 NSWindow；焦点事件不更改材质、尺寸、透明度或图层配置。所有实验仍可选原生 Liquid Glass。

| 实验 | 改变的配置 | 本机配置检查 | 切回前台目测 |
| --- | --- | --- | --- |
| A | 原始配置 | 通过 | 此前已确认闪黑 |
| B | 背景 alpha = 0.001 | 通过 | 用户确认：已不闪黑，玻璃正常 |
| C | 关闭原生阴影 | 通过 | 备选，未采用 |
| D | 首次显示前禁止自动扁平化 | 读回 false | 备选，未采用 |
| E | D + 首次显示前重新托管图层 | 扁平化 false、托管 true | 备选，未采用 |
| F | E + 背景 alpha = 0.001，保留阴影 | 通过 | 备选，未采用 |

本机运行时检测到方法名为 `_setShouldAutoFlattenLayerTree:` / `_shouldAutoFlattenLayerTree`，而 `canHostLayersInWindowServer` 的访问方法不带下划线。实验程序检查 selector 和 BOOL 方法签名后调用，不依赖 KVC 猜测，也不写入未经核实的 CGS 标志位。

构建与配置验证：

```bash
xcrun swiftc -module-cache-path /private/tmp/solosoul-swift-module-cache \
  -framework AppKit tauri/scripts/macos-native-appearance.swift \
  -o /private/tmp/solosoul-native-appearance
/private/tmp/solosoul-native-appearance --probe-window-api
/private/tmp/solosoul-native-appearance --check-configurations
```

配置检查需要可连接 WindowServer 的图形会话；它不显示窗口，也不能证明消除了过渡帧。手动运行可用 `--variant=alpha` 等参数，或通过窗口下拉框选择 A–F。瞬态黑帧由人工验收；B 已有效，无需继续采用 C–F 的额外配置。

### 接入 SoloSoul

根据 B 方案的人工结果，生产实现只采用公开的 `NSColor.white.withAlphaComponent(0.001)` / `NSWindow.backgroundColor`。首次挂载材质之前设置底色，重复主题同步保持该值；切换不透明模式仍使用 alpha=1 的主题色。原生玻璃、窗口阴影、Dock 行为和 WebView 层级保留；未引入诊断程序中的私有图层接口。

已完成 `cargo build`、`cargo clippy -- -D warnings` 与 `cargo fmt --check`。使用生产代码和 WKWebView 的图形回归程序也已通过：标题栏完整覆盖、WebView 留在标题栏下方、视图身份稳定、主题同步、隐藏/显示、调整尺寸，以及玻璃/不透明往返切换后微量底色与原生阴影保持。

完整客户端已通过 `npm run tauri -- build --debug --bundles app -- --offline` 构建，包含前端 TypeScript 检查与生产资源打包。产物为 `tauri/target/debug/bundle/macos/SoloSoul.app`，已通过图形工具确认进入实际登录页，兼容模式未勾选（玻璃开启）。

图形工具恢复后，验收窗口静态截图显示顶部为完整圆角；随后自动执行完整客户端 Dock 切换时再次超时。2026-09-14，用户完成修复验证并确认通过，随后授权提交与推送。人工验收补足了自动检查和静态截图无法确认过渡帧的部分；未据此宣称覆盖所有系统版本或完成前述全部扩展测试矩阵。
