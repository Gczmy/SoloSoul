# FE2 macOS 当前共享前端最终隔离构建

> **资料精简说明（2026-10-08）：**本页保留历史验收说明，批量原图、录屏、日志和输入快照已移出当前目录；下文的结果、来源与未完成项保持原验收边界。完整 6297 份原件已逐文件备份并建立 [SHA-256 索引](../fe2-evidence-index-2026-10-08.json)，恢复方法见[归档说明](../README.md)。迁出文件的 Markdown 链接指向固定原提交；代码块中的原路径及依赖完整目录的复跑命令需先恢复原件。

2026-10-08。Android 通知横向安全区修复后，完整 macOS 隔离 Debug app 已重新构建，退出 0。bundle ID `com.solosoul.fe2.macos`，版本仍为 2.13.2；二进制 SHA256 `724833c7aea2360710172b35e93ab581a6a0c0694335ef7eed2565c953560eaf`，冻结副本与实际 bundle 一致。

95 份来源审计通过。构建前、中 hash 分别保留，只允许 Tauri CLI 临时追加 macos-private-api 的精确 Cargo 差异；原 Cargo.toml、已有 Android 插件 registry 和 tauri.properties 字节 / 权限已恢复。编译资产有独立 manifest，不借历史 Android 或 Mac 包的资产证明本包。

对上一轮既有、未运行窗口的私有合成根执行实际 --fe2-macos-preflight，退出 0，根权限 0700。本包未绑定、未启动窗口，没有读取正式账户。当前电脑控制工具报告 Mac 锁定，不能继续原生视觉操作；旧窗口截图仍属于旧二进制。

解锁后需确认旧测试实例，再显式绑定 / 标准启动当前 bundle，完成顶栏与四种导航、深浅登录和 PIN / Touch ID、菜单 / 预览 / 辅助功能及 Dock / 台前调度连续帧恢复。构建与 preflight 不证明这些原生要求完成。任务继续 8 完成、6 待验证，未提交、推送或发布。
