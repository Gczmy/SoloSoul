# 当前源码 iOS 模拟器包构建

2026-10-08，Xcode26.3 / macOS26.6 ARM64。按正式 Tauri 命令构建当前源码 Debug、无签名、仅归档的 iOS Simulator 包；生产二进制 SHA256 为 `8c062ef6c8cc05081d77dd9c56e61584d000d59417550beb2cd9ccad86118f56`，bundle ID `com.solosoul.app`。命令和包装脚本均退出0，完整 app 已冻结在本机 `/tmp/solosoul-fe2-ios-current-build-20261008/SoloSoul.app`。

`source.json` 冻结95份执行时源码。构建后恢复8组受保护输入的内容、链接和权限（包括两个 Cargo 清单、Apple 项目/Info.plist/资产目录和 Android 资产/属性），逐项恢复通过，95份源码没有构建引入的差异。私有备份和完整构建包不入库；这里只保留来源、原始日志和恢复回执。

构建成功只说明此模拟器目标可编译，不是设备签名、真机部署、系统栏材质或完整 UI 验收。此前 `b7c5ae…` 包的主题矩阵和欢迎页 UI 结果保持原包来源，不升级为当前源码通过。账户 UI 的后续实际执行见[账户验证](../fe2-ios-current-account-2026-10-08/README.md)。
