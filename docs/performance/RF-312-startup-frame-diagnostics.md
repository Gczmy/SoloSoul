# RF-312：Android 启动帧诊断拒绝记录

累计七次原生候选均未通过，两次提交帧仍未验收。每次计划 100→5000，实际均在 100 档拒绝后停止，5000 档未启动；没有用重复运行替换既有失败，也不加入默认六对统计。

## 观察与修正

| 轮次 | 实际边界 | 结果 |
| --- | --- | --- |
| 1 | 登录文档就绪 | 0 次提交；完整失败 JSON 未及时收回，保留终态输出和缺口。 |
| 2 | 原生视图检查 | 0 次提交；首次层级显示 WebView 为 0×0，粗粒度错误不足以唯一确定失败谓词。 |
| 3 | 登录文档就绪 | 可见、启用、本地登录路由及两项命中均成立，文档焦点为 false。 |
| 4 | 两次原生提交 | 登录就绪已通过，0 次提交；仍为拒绝，未改成动画回调或强制重绘。 |
| 5 | 栅栏与绘制进度 | 栅栏完成、监听已安装；期限内未观察到后续有效 `OnDraw`，0 次提交。 |
| 6 | 预绘制注册 | 修正参考 API31 回调提取顺序，仍要求实际 `OnDraw` 与提交；本轮仍为 0 次提交。 |
| 7 | 可选早期窗口追踪 | 32 次绘制、31 次指标回调；栅栏后无新 Java 绘制，只有一次指标投递，原两帧仍拒绝。 |

隔离候选把首次布局完成前的视图保留为原期限内的待就绪状态；绑定之后仍要求同一 Activity／WebView、有效几何、硬件渲染及原生窗口焦点。文档焦点与页面可见性分别记录；源码还要求成功确认前再次检查系统前台窗口及原进程身份，该成功分支尚未走到。Android 的窗口焦点与视图输入焦点不同，DOM 的 `hasFocus()` 也有独立定义。[Android View](https://developer.android.com/reference/android/view/View#hasWindowFocus())、[HTML 焦点定义](https://html.spec.whatwg.org/multipage/interaction.html#dom-document-hasfocus)。

第五轮仅增加五个固定进度代码及同代拒绝读取支持，移除新增赋值可逐字还原原采集器。完整终态停在 `native-draw-listener-armed`，证明视觉栅栏回调完成且监听已安装；没有后续有效 `OnDraw` 或提交记录。尚不能唯一归因于页面静止、观察挂接时机或绘制路径，第六轮将注册改到预绘制，但仍没有后续有效绘制；原因继续待查，保留原两帧条件。

`VisualStateCallback` 表示内容已可供后续绘制；提交回调表示帧进入交换链，也不证明显示器已呈现。两个 API 的语义均不能替代本次缺少的两次实际回调。[WebView VisualStateCallback](https://developer.android.com/reference/android/webkit/WebView.VisualStateCallback)、[ViewTreeObserver](https://developer.android.com/reference/android/view/ViewTreeObserver#registerFrameCommitCallback(java.lang.Runnable))。

## 第七轮被动追踪

默认关闭的 `--window-frame-trace` 已通过真实运行。它在已有 Activity 窗口创建完成后挂接，记录到 32 次预绘制、32 次绘制、31 次窗口指标回调；WebView 与 Decor 属于同一视图树。栅栏后预绘制／绘制均为零，有一次指标回调投递；原生报告丢失为零，前缀未截断，监听及 worker 在原三秒内拆除。API 仍可能丢失未投递报告，不能声称全量覆盖。[窗口指标 API](https://developer.android.com/reference/android/view/Window.OnFrameMetricsAvailableListener)。

投递时间不是该帧绘制时间；没有将 VSYNC 原始时钟与栅栏换算比较。此单个带观察开销的模拟器诊断不用于判断产品性能瓶颈。早期绘制存在，但当前要求的后续两次提交没有出现。下一步核对原 RF-312 的启动测量终点与当前诊断前提；旧两帧条件、拒绝、默认六对和已通过的 IPC 口径保持。

## 注册顺序修正

Android 12 参考源码在派发 `OnDraw` 前提取本帧提交回调。候选在 `OnPreDraw` 提前注册，但仍必须由实际 `OnDraw` 记录绘制时间、实际提交回调完成预约；一次只保留一个预约，前一帧提交后才允许下一帧。此顺序缺陷与本轮没有后续绘制不是已证明的同一原因。纯 Java 回归仅验证事件顺序与时限，不能当作真实帧。[AOSP 参考](https://android.googlesource.com/platform/frameworks/base/+/android12-release/core/java/android/view/ViewRootImpl.java)。

## 实际验证与保全

- 最新 52 项 Python、11 项纯 Java 顺序回归和 9 项就绪检查通过，零跳过。公开 19 份源码与原生测试来源逐字一致，实际十二步构建产物与第七轮原生 APK 相同，绑定 115 份运行及 123 份构建来源；正常 CLI 两项、优化模式八项拒绝及异地路径帮助两项通过。
- 原六轮独立审计保持不变；第七轮另以同代来源复核 15 份原生状态、49 份完整控制回复、22 份只读 UI 回复及两张相同 PNG。七轮合计 96 份状态、473 份完整控制回复、305／306 份完成／尝试 UI 回复、十二份 PNG、六份完整拒绝；首轮缺口与一个拒绝尾部保留。
- 七个全新环境分别实际查看三片 PNG、完整窗口／lastanr／DropBox 后放行。第七轮的 39 条 DropBox 和终态无 ANR 已检查；两张登录截图实际查看一致。七轮分别恢复 134／107／90／97／75／60／78 项私有内容、链接及权限，原 APK／UID／系统设置恢复，七份原期限内严格宿主归零回执通过。
- 第一轮公开 OCR 夹具的独立清理没有通过，未写成全清理成功；其失败输出和隔离 AVD 保留。另一个旧 PowerShell 路径启动前失败未启动 SDK，随后使用存在且 SHA 相同的实际路径，旧失败不回写。

第五轮宿主等待在原期限的非正余量处拒绝，完整错误原样保留；失败后才在原清理期限内收回哈希绑定终态，未转为成功。

原 45 秒阶段、3 秒只读 JS、5 秒状态 SDK、240 秒／512 请求观察器、240 秒启动、600 秒健康审查及 45 秒宿主退出期限保持。原始系统回复、截图、APK、私有备份及本机标识只留本地；[公开显式诊断入口](RF-312-frame-runtime.md)已验证 CLI、来源复制及生产者；完整两档正向原生入口尚未验收。

见[去标识结果](rf312-startup-frame-diagnostics-2026-10-18.json)。应用本体时延、启动两帧、完整观察开销、全部归属进程全程／峰值内存及多端仍待；RF-312、368/377、九项未完成及 goal 保持。
