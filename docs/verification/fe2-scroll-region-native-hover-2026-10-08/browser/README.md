# 2026-10-08 滚动区域鼠标高亮：浏览器验收证据

> **资料精简说明（2026-10-08）：**本页保留历史验收说明，批量原图、录屏、日志和输入快照已移出当前目录；下文的结果、来源与未完成项保持原验收边界。完整 6297 份原件已逐文件备份并建立 [SHA-256 索引](../../fe2-evidence-index-2026-10-08.json)，恢复方法见[归档说明](../../README.md)。迁出文件的 Markdown 链接指向固定原提交；代码块中的原路径及依赖完整目录的复跑命令需先恢复原件。

本目录只收录本轮 CSS 原生 `:hover` 实现的浏览器验收与调试证据，没有合并旧轮报告。测试均访问独立的本地 Vite fixture 或模拟 Tauri IPC 的完整应用，没有控制真实原生客户端，也没有读取或修改个人 Vault 数据。

| 证据组 | 浏览器 | 用例结果 | 截图像素结果 | 范围 |
| --- | --- | --- | --- | --- |
| `fixture-32/` | Chromium、WebKit | 32/32 通过 | 144/144 通过 | 每浏览器 16 用例；浅色/深色各三轮 main、sidebar、tools、chrome 往返；逐帧采样 main 和 tools 滑块 |
| `integration-14/` | Chromium、WebKit | 14/14 通过 | 48/48 通过 | 完整 AppShell 的 macOS/Windows × 浅深主题共 8 例提供 48 个 main 滑块采样；另 2 例验证历史字段键盘保护，4 例验证 Android/iOS 320px 登录密码布局 |

## 方法与局限

测试断言浏览器原生悬停产生的每轴 CSS 自定义属性 `--scrollbar-thumb-x/y`，再以 PNG 中实际滑块的 RGB 像素验证最终绘制。fixture 的 window capture 在应用加载前拦截全部 pointer/mouse 移动与进出事件，随后只执行 `page.mouse.move()`；两种浏览器仍正确切换区域颜色，点击和滚轮计数均为 0。其余用例覆盖同轴/异轴/双轴嵌套、三种弹窗边界、静止指针下动态溢出、文本/input 更新、平台标记以及触摸平台门控。

这些结果证明测试浏览器中的 CSS 判断和截图绘制，不等同于真实 WKWebView 客户端验收。144 个 fixture 采样没有逐一覆盖所有动态用例的实际像素；48 个完整应用采样也只核对正文 main 滑块。原始 integration 像素 JSON 的 `scope` 文本复用了通用模板，具体范围以本 README、该 JSON 的 `samples` 和测试文件为准。

`getComputedStyle(el, '::-webkit-scrollbar-thumb')` 不能作为独立验收依据：Chromium 在嵌套区域仍匹配宿主 `:hover` 时会报告亮色，实际父区域滑块已绘制为灰色。`historical-debug/computed-style-false-positive/` 保留本轮误判、原始失败上下文与诊断截图。诊断中 tools 滑块像素为 `[17,17,17]`，sidebar/main 为 `[198,198,198]`，与最近同轴区域的规则一致。WebKit 也不提供可靠的该伪元素计算样式。

## 文件与来源

- `fixture-32/results/`：最终成功运行的原始截图、`paint-samples.json`、`.last-run.json`。
- `fixture-32/playwright-stdout-session-85904.log`：工具会话 85904 返回的成功结果行按原文归档，省略 Node/颜色环境警告；原执行没有单独重定向日志文件。
- `fixture-32/pixels.json`、`pixels-original.py`、`playwright-original.config.cjs`：从本轮 `/tmp` 结果字节一致复制；同时保存测试与 fixture 源码快照。
- `integration-14/`：主代理提供的本轮原始 Playwright log/JSON/config、像素报告/脚本与成功截图，均为字节一致复制。
- `historical-debug/fixture-selector-failure/`：本轮新增临时 div 后 `main > div:last` 命错目标导致的 30/32 调试运行；删除临时区域后修正。
- `historical-debug/computed-style-false-positive/`：本轮不可靠伪元素计算样式断言导致的 21/32 调试运行；用真实像素证实误判后移除该断言。
- 历史失败日志标为 `stdout-excerpt`，是工具返回内容摘录，不冒充完整原始日志；失败 `error-context.md`、截图与 `.last-run.json` 为原文件复制。
- `path-map.json`：原始 JSON 中 `/tmp` 截图绝对路径到归档相对路径的映射。原始 config/scripts 仍保留当次运行的本机路径；归档没有改写这些原证据。
- `verify-archived-pixels.py`：直接验证归档截图，不依赖原 `/tmp` 文件，也不写任何文件。
- `FILES.txt`：文件清单；`evidence-manifest.json` 记录各文件 SHA-256、大小及来源，排除其自身与 `SHA256SUMS`；`SHA256SUMS` 还覆盖 manifest，自身除外。

归档后可在本目录运行 `python3 verify-archived-pixels.py` 复核 192 个最终成功采样，并运行 `shasum -a 256 -c SHA256SUMS` 核对字节完整性。
