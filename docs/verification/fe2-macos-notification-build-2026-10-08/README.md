# FE2 macOS 通知安全区初次构建

2026-10-08。完整隔离 Debug app 构建退出 0，二进制 `5ba956e660919fa73948c04f6bf80e1ce8392c9917da3878ce12d5dfdd6c8557` 已在本机单独冻结。构建前 / 中源码和资产分别记录，CLI 唯一允许差异为精确的 macos-private-api feature；三份受保护文件已恢复。

后续将通知安全区规则限定到已登录 Android 壳，避免认证壳和平板导航轨道重复留白。本初次包没有启动窗口、绑定或原生视觉验收，已经由 [最终构建](../fe2-macos-notification-final-build-2026-10-08/README.md) 取代，不能视为后续源码通过。
