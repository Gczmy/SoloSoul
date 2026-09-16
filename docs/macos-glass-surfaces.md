# macOS 应用内玻璃表面

实现入口：`tauri/src/styles/macos-glass.css`，由 `bootstrapApp.tsx` 加载。

## 范围与接入

材质只在 `html[data-desktop-platform='macos']` 生效。窗口背景继续使用现有原生材质；应用内浮层使用背景模糊、半透明底色、渐变高光和内外阴影模拟液态玻璃外观。

| 标记 | 用途 |
| --- | --- |
| `data-macos-glass="panel"` | 对象详情、指南、侧栏快捷卡片、对话框 |
| `data-macos-glass="popover"` | 更多操作、下拉菜单、尺标对象预览 |
| `data-macos-glass="sidebar-menu"` | 侧栏工具菜单，直接透出侧栏共用的原生玻璃 |
| `data-macos-glass="tooltip"` | 悬停说明、密码提示 |
| `data-macos-glass="notification"` | 通知卡片，保留状态色边框 |
| `data-macos-glass="control"` | 显式指定的玻璃按钮 |
| `data-macos-glass-backdrop` | 浮层遮罩，取消重复的全屏背景模糊 |
| `data-macos-glass-section` | 浮层内需要淡色分区的区域 |

AppBar 自动覆盖操作按钮，排除其弹出菜单内部的按钮。通用 `Dialog` 已接入；使用 `Card` 作为浮层外壳时设置 `surface="floating"`。正文卡片、字段输入区和全屏图片预览继续使用各自的材质。

这里只设置表面样式，不修改定位、尺寸、滚动或拖拽命中区。新增浮层时，把标记放在实际绘制卡片的 DOM 外壳上；二级固定浮层通过 Portal 或兄弟节点呈现，避免父卡片的滤镜改变定位参照。

浅色和深色分别调节透明度、高光与阴影；原生返回 `solid` / 高对比，或浏览器请求减少透明度 / 强制颜色时，改用实色表面。

侧栏工具菜单位于原生材质覆盖的导航区域内，确认 `liquid-glass` / `vibrancy` 后使用透明背景，不叠加网页模糊或乳白底色。菜单展开时，仅裁去主导航中被菜单覆盖的部分；主导航与添加页面按钮保留原有位置，关闭后恢复显示。

展开与折叠侧栏共用向上浮出的菜单及可用高度计算。折叠侧栏保留紧凑图文按钮，菜单限制在原生玻璃轨道内，仅高度不足时滚动；不再通过增减工具区高度挤动主导航和入口。

## 验证（2026-09-16）

- TypeScript、ESLint、Vite 生产构建通过。
- 相关组件单元测试 37 项通过。Node 26 需使用 `NODE_OPTIONS=--no-experimental-webstorage`，避免 Node 自带的实验性存储覆盖 JSDOM 存储。
- `e2e/macos-glass.spec.ts` 在 Chrome 与 WebKit 中通过：深浅色、AppBar 尺寸和跳转、侧栏卡片、对象详情及二级浮层、键盘操作和辅助功能回退。
- 现有 macOS 标题栏、工具栏、侧栏定位、Windows 标题栏和 Android 材质回归通过；侧栏定位测试同步使用现行 macOS 96px 折叠宽度。
- 使用系统 WKWebView 加载本次 CSS，按窗口截图对比条纹背景：玻璃卡片的背景模糊及边缘高光正常。Playwright WebKit 的页面截图未呈现该背景模糊，不能仅凭其截图判断原生合成结果；上游也有 [WebKit 截图忽略 CSS 滤镜的记录](https://github.com/microsoft/playwright/issues/28363)。

原生窗口恢复逻辑未变更，本轮未重跑 Dock / 台前调度唤醒的整机回归。
