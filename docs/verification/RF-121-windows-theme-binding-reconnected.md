# RF-121 会话恢复后的 Windows 窗口验证

2026-10-10。第五轮三个独立公开测试库的完整原生行程和主题/窗口/像素绑定均通过。实际查看全部三张截图：浅色首页可读，侧栏、顶栏、内容区和卡片层次可辨，没有空白或整页黑屏。这里只验收该固定诊断入口与已观察场景，RF-121 完整材质矩阵保持未完成。

运行前记录工具所在会话 1 为 Active，输入桌面可用；物理控制台仍为会话 2。[WTSGetActiveConsoleSessionId](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-wtsgetactiveconsolesessionid) 返回物理控制台会话，不能据此要求当前工具迁移到会话 2。此前 Disconnected、访问错误 5 和四轮失败原件保留；恢复后捕获成功，没有证明历史无帧的唯一根因。

本轮复用未改动的 GUI、明确读取能力的第三版窗口工具、1830 项源码/配置和公开 100 对象 v1 夹具。三个样本均为 `light`、DOM 材质 `mica`、展开侧栏、96 DPI、前台窗口，辅助功能标记均为 false。三次六阶段行程、六条认证流程、维护准入合同、固定请求回执和实际 BGRA 字节 SHA 均通过独立校验；捕获、独立验证与图像转换退出码均为 0，原预算保持。

截图前后绑定同一 run/PID/browser、frame/loader/timeOrigin，以及记录的主题、色板、视口、表面几何、辅助功能与焦点。原始像素为 1202×832 的 BGRA8-premultiplied，PNG 经明确的去预乘转换生成。第二、三张可见 OCR 安装完成横幅，第一张没有；因此通过的是固定记录表面的状态绑定，不能扩展为整页像素不变、无闪烁或恢复行为通过。三项未文档化 DWM 属性仍为 null，`nativeAppearanceVerified=false`；DOM 的 `mica` 不等于实际 backdrop 属性或透明度效果验收。

清理前 fresh CIM 核验 27 个记录身份退出，固定 GUI 与捕获工具均无存活进程，清理工具未结束进程。只删除本轮 24 个隔离缓存、解除 3 个联接；111 份 Vault/根级原件与应用日志逐字保留。141 份归档原件全部回读 SHA 一致，不含物理 Vault、可执行文件、模型或缓存。

见[结构化结果](rf121-windows-theme-binding-reconnected-2026-10-10.json)、[原件索引](rf121-windows-theme-binding-reconnected-2026-10-10/index.json)及三张实际截图：[样本 1](rf121-windows-theme-binding-reconnected-2026-10-10/surface-sample-001.png)、[样本 2](rf121-windows-theme-binding-reconnected-2026-10-10/surface-sample-002.png)、[样本 3](rf121-windows-theme-binding-reconnected-2026-10-10/surface-sample-003.png)。[四轮失败历史](RF-121-windows-theme-binding.md)保留。

本轮没有改动业务代码；已通过且输入未变的 Rust/Node/构建检查沿用原收据，不重复运行。计时仍是带诊断入口的记录，`performanceMetrics=null`。后续需要独立、隔离且合同明确的深色与折叠/缩放/恢复场景，再补辅助功能、长内容和其他平台；Windows 捕获能力目前已恢复，不再作为环境阻塞。RF-112、RF-121、RF-312 及 RF-122～127 前置保持未完成。
