# RF-312：原生手指事件与窗口绘制就绪

本轮确认系统图片选择器可经真实坐标手指事件返回应用，固定 100/5000 对象完整 12 阶段诊断及外层验收各一次实际退出 0。探针没有可访问性点击回退、URI 注入或后端捷径。产品代码、依赖与主 APK 不变；RF-312 仍待重复样本及中位/尾部指标、全进程内存、冷启动、硬件和其他平台。

旧简化 `MotionEvent.obtain(...x,y...)` 设置 `SOURCE_TOUCHSCREEN` 后，设备实测 `getToolType(0)` 仍为 0。相同对照 APK 中，明确未知工具类型的事件成功注入但选择器没有返回、没有 OCR 结果；改为 `TOOL_TYPE_FINGER=1` 后，同样坐标、source、pointer id/count、pressure、size、device id 和 DOWN/UP 动作完成选择并得到已知 OCR 正文。事件对之间不写报告文件，避免 I/O 拉长按压；两种模式均执行一次约 100 ms 的坐标按压，不重试点击。

该对照支持本探针的工具类型问题，不将它扩大为所有历史失败的唯一原因。[Android 官方 API](https://developer.android.com/reference/android/view/MotionEvent.html)区分输入来源与指针工具类型；[PointerProperties](https://developer.android.com/reference/android/view/MotionEvent.PointerProperties)可显式设置工具类型，[当前官方 AndroidX 源码](https://raw.githubusercontent.com/androidx/androidx/androidx-main/recyclerview/recyclerview-selection/src/main/java/androidx/recyclerview/selection/MotionEvents.java)也以工具类型识别手指。源码仅作机制参考，未声称已反编译或精确匹配设备上选择器的库版本。

手指对照组在 OCR、搜索和媒体路径继续执行，随后在关闭扫描 PDF 后的旧绘制断言失败，实际退出 1并保留。旧断言联合检查附着/显示/焦点，没有分别记录哪一项失败，因此不能唯一归因为焦点。最终探针等待精确 Main WebView 的 `RESUMED`、附着、显示和窗口焦点同时满足，再使用真实 compositor callback 和两帧绘制；准备与绘制共享原 10 秒及外层 45 秒截止，不增加超时、不放松条件。

| 对象数 | 完整原生/外层结果 | 首次 OCR 到结果 DOM | 窗口正文前景像素 | 绘制检查 | 最长准备及提交耗时 |
|---|---|---:|---:|---:|---:|
| 100 | 12 阶段通过 / 退出 0 | 3151.2322 ms | 2235 | 202 次全部满足 | 1343.9073 ms |
| 5000 | 12 阶段通过 / 退出 0 | 3197.6953 ms | 2235 | 199 次全部满足 | 1028.1867 ms |

两组文件事件均记录 `source=4098`、`toolType=1`、单指 id 0、真实 DOWN/UP 和注入结果。首次 OCR 到已知正文的口径包括选择器返回、IPC、解码、进程首次 ML Kit 初始化及轮询；正文经过原生滚动、实际窗口像素和人工复核，完整显示在固定导航栏上方。ML Kit 的 100% 显示仍是已有置信度占位，本轮不把它视为准确率。搜索、四类媒体/两页切换、关闭返回、锁定及两次真实密码解锁均通过。

附加证据校验接受两组全部绘制记录，拒绝缺少提交/焦点、错误 Activity、超出原截止、未知工具类型和布尔工具类型等 6 个负例。两条控制失败和两条最终成功均保留；四对私有数据恢复记录各 216 条目，内容/链接/权限独立一致，8 个私有 TAR 已删除。原包/UID/blur及唯一测试图片和对应 MediaStore 索引已恢复或清理，APK、签名密钥、数据库与私有备份不入库。

[结构化结果](rf312-android-finger-2026-10-12.json)及[164 成员原件索引](rf312-android-finger-2026-10-12/index.json)保留实际源码、构建/签名/运行退出码、全部失败、图像和恢复核验；[原件包](rf312-android-finger-2026-10-12/raw.tar)逐成员长度/SHA-256 核对通过。每组最终仅一次；阶段时间含仪器、输入、轮询和绘制检查开销，PSS 包含 runner、不含 renderer，不据此计算总体中位/尾部或制造产品优化结论。

下一步沿用该完整手指事件行程重复采样，再补齐 renderer/全进程内存和采样开销归因；搜索截图在 IME 关闭后尚需绘制稳定核验。当前没有关闭 RF-312，没有增加关闭任务数，不自动推送。
