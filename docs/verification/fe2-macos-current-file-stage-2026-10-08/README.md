# 当前 macOS 包与短暂恢复的原生会话

> **资料精简说明（2026-10-08）：**本页保留历史验收说明，批量原图、录屏、日志和输入快照已移出当前目录；下文的结果、来源与未完成项保持原验收边界。完整 6297 份原件已逐文件备份并建立 [SHA-256 索引](../fe2-evidence-index-2026-10-08.json)，恢复方法见[归档说明](../README.md)。迁出文件的 Markdown 链接指向固定原提交；代码块中的原路径及依赖完整目录的复跑命令需先恢复原件。

2026-10-08。当前106份源码构建独立Debug完整客户端，二进制6410ed9994f3c84ddbe71077817f6315007be2de59f04758c943e608db1cec3d；构建/保护驱动退出0，9组平台输入恢复，源无差异。旧PID99069已退出，未终止其他应用。

新0700合成根prepare/preflight退出0，新0600绑定仅写冻结bundle；CUA按完整.app路径标准启动。没有继承外部sandbox，隔离来自FE2显式路径校验；完整app/保护备份/根绑定/合成Vault仅在/tmp。

[CUA观察](https://github.com/Gczmy/SoloSoul/blob/75f1c1594504dd6e09af57ff6debd25cccce03fa/docs/verification/fe2-macos-current-file-stage-2026-10-08/native-cua-observation.json)：标识查询两次超时不当锁屏；随后Finder实际可读，新包实际显示合成登录页。真实1200x800登录图像在会话工具输出，深圆角底板、浅面板及正文/控件/恢复入口可见，未导出PNG，不声称像素序列或图像哈希。真实密码输入和回车解锁后AX路由到首页、出现侧栏开关和合成账户欢迎；再取首页截图时系统明确锁屏，没有首页像素。

[只读进程证据](https://github.com/Gczmy/SoloSoul/blob/75f1c1594504dd6e09af57ff6debd25cccce03fa/docs/verification/fe2-macos-current-file-stage-2026-10-08/current-process-observation.json)规范化/private/tmp路径后核对当前冻结包PID49150、二进制哈希及独立Vault/日志/插件打开路径；未观察到正式路径，不等于全部访问保证。首个未规范化/tmp探针没有匹配，未据此重启。当前进程留给解锁续验。

仅浅色登录画面与真实解锁动作；整条交通灯/顶栏、四导航/浮层、辅助功能、预览、窄矮窗及Dock/台前调度连续恢复帧待验。系统未改，目标14项、8完成、6待验证，未提交推送发布。
