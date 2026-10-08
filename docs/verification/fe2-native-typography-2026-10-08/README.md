# 实际 Android 系统字体缩放

2026-10-08，专用 API34 ARM64 / WebView 113.0.5672.136。生产包保持 `474846b7a031baa21c0ab27c013fa1981c9195c06e815e5da2f713c1a4a56c04`，产品前端 / MainActivity 没有修改。测试通过生产 UI 引导、主题、锁定解锁和对象详情返回，读取真实 Android configuration.fontScale、WebView.textZoom 及首页标题 / 数量 Range，不调用 setTextZoom、不注入网页字号。

## 已接受的范围

系统字体 1.0 与 1.3 两组各 **1/1、0 跳过、退出 0、12 阶段 / 11 PNG**，同一 `d3008615…` 仪器包。六个首页状态中的标题 / 数量均有几何与实际正文合成像素；正常 Range 高于逻辑 line-height 不等于裁切，检查真实 overflow 裁切祖先。

两份同运行环境报告逐一比较 12 组匹配文案：实际 WebView textZoom 为 **100 → 130**，文字高度倍率 **1.30000～1.30233**。仅系统设置值变为 1.3 而文字未放大的报告不能通过。精确范围见 `font-scale-pair.json`、两组原始报告与 `accepted-summary.json`。此结果不包括后来新增的顶栏图标像素门槛，不能将旧仪器结果升级为该门槛通过。

各组 242 项私有内容 / 链接 / 权限、根权限、blur、通知授权标记及原系统 font_scale（包括缺失状态）恢复。归档时实际比较前后 tar 清单一致；私有 tar 不进入本目录。

## 2倍字号最终结果

`native-double-final/` 同一主包、新测试包 `44fae416fbe9c8905f6b6385a05aca2d1a49ef8baee7ea3132c9d629ae575ecf`，系统模糊支持模式恰好 **1/1、0 跳过、退出 0、12 阶段 / 11 PNG**。真实 WebView textZoom 为 **200**，与正常字号的 12 组文字高度倍率均 **2.00000**；对照主包 hash 相同，仪器包和取证顺序分别保留来源。最终每个首页状态均核对三个顶栏按钮的范围 / 命中 / 非零 SVG 与实际合成前景，最低图标像素 **152**。

首页文案和长菜单通过真实触屏滑动进入阅读区域，没有通过缩小产品字体或放宽裁切门槛来通过。编辑器和菜单保留真实 8 秒备份提醒；菜单滚动后 Edit / History / Attachments / Delete 全部完整可见、无通知覆盖且可命中。仅字体场景将菜单放到详情截帧之前，普通 baseline 顺序未改；仪器报告明确记录 `fontMenuBeforeDetail`，外部判定要求对应顺序。系统返回分别关闭菜单 / 详情并保留 Travel 页面。

深色增强首页和带真实提醒的完整菜单最终帧已目测。全数据 / 权限与原字体设置恢复，前后本机 tar 再次核对一致。最终 Python 拒绝回归 **33/33**，新增检查拒绝缺图标像素 / 命中 / SVG、标题覆盖、错误顺序、缺真实提醒、菜单按钮缺失或越界。首次新增拒绝回归有 1 项失败：外部判定漏检菜单提醒，原生截图已经有真实提醒；补全外部判定后重新检查归档报告通过。`final-execution-source.json` / `final-artifacts.json` 记录实际设备执行来源，`final-checks-source.json` 记录随后判定脚本与拒绝单测的差异，没有冒称重新运行设备。

## 失败与来源

- `native-normal-initial/`：首个探针把 Range 高于 line-height 误称为实际裁切，失败。改为比较真正裁切祖先；失败没有计为通过。
- `native-double-initial/`：2倍文字的数量位于滚动视口下沿，未滚动探针失败。这不证明滚动后仍不可读；随后使用真实触屏滑动进入阅读区。
- `native-double-header-probe/`：三个正常顶栏按钮实际在视口内且可命中，另误采 visibility:hidden 的指南菜单按钮造成失败。不能根据这次失败截图宣称标题挤出正常按钮；后续排除隐藏菜单，并增加 SVG 几何和最终合成像素检查。
- `native-double-menu-before-scroll/` / `native-double-menu-reminder-expired/`：先要求长菜单全部初始可见，后真实滚动时晚期截图遇到提醒自然到期。保留失败，提前菜单取证，不延长生产提醒或削弱按钮门槛。
- `native-double-range-before-slop/`：手势部分移动被原生 touch slop 消耗；改为读取实际 Range，并给滑动距离留阅读余量。最终裁切容差不变，不能以增大裁切容差接受越界。
- `initial-source.json` / `initial-artifacts.json`、`source.json` / `artifacts.json`、`enlarged-execution-source.json` 分别记录执行阶段；新增 header 探针的来源另存，不替换已通过的仪器 hash。
- 最终拒绝回归与来源后续单独冻结；测试源变化不需要重建当前 macOS 生产包。

以上字体矩阵均为系统模糊支持模式。上述证据不证明全部页面、放大字号下键盘、多窗口、iOS、真机字体可访问性或实体性能；完整重构目标保持进行中。
