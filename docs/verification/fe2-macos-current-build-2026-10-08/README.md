# FE2 macOS 当前源码隔离客户端重建

2026-10-08。此阶段仅证明最新源码构建及隔离入口可用，**没有启动窗口，没有新增视觉 / 恢复通过结论**。上一阶段浅色窗口截图仍属于其原二进制，不移用到本阶段。

## 构建与来源

以 Tauri CLI debug、macos-ui-regression feature、独立 macOS 配置及 app bundle 构建，退出 0。新二进制 SHA256 为 `1c1b28ff7af5fd6a1111548ddec70f06eb50253e324db8869c786fcf67ece961`，保存在本机独立 frozen 路径；bundle ID 为 com.solosoul.fe2.macos，版本保持 2.13.2，不是发布包。

本次包含当前共享偏好缓存、镜像 IPC 修复，以及最新前端。14 份隔离 / 配置 / 共享源码快照见 source-during-build.json，85 份工作区来源 hash 见 worktree-source-during-build.json，35 份实际前端构建资产 hash 见 frontend-assets.json。首次离线构建链接失败，ort-sys 明确指示 CARGO_NET_OFFLINE 会跳过预构建库查找，且未设置 ORT_LIB_PATH；不是内部单态化符号缓存损坏，未执行 cargo clean，也未改 Rust 依赖。第二轮显式指向上一成功构建已使用的本机 ARM64 libonnxruntime.a 缓存，仍离线，构建成功。缓存位置、大小和 SHA256 见 onnx-cache.json，原始失败与成功日志均保留。

Tauri CLI 按 macOS 配置临时启用 Cargo 的 macos-private-api；编译快照保留实际启用状态，构建后恢复原 manifest。原 Cargo.toml、Android registry 和 tauri.properties 的字节均恢复。第一次来源核对直接要求所有当前文件与编译快照相等，因此拒绝这一预期的 manifest 差异；后续仅接受精确的该 feature 变更，其余 13 份源码与 35 份资产核对通过，见 source-audit.json。没有将旧编译来源改写成当前工作区来源。

## 无窗口验收

新二进制实际 --fe2-macos-prepare 在新私有临时根创建一个公开合成账户，权限 0700，随后 --fe2-macos-preflight 退出 0。外部根、冲突 SOLOSOUL_DATA_DIR、无 bundle 绑定的 frozen 二进制三个负向检查均按预期退出 1。原文及路径见 build-result.json、prepare.log、preflight.log 和 preflight-negative.json。没有访问正式账户，没有启动 Tauri 窗口或操作系统认证 / 辅助功能设置，也没有执行新 bundle 的绑定命令；重建包需显式重新绑定，旧合成 Vault 未操作。

## 下一步

Mac 当前锁屏。手动解锁后，先确认并关闭原测试实例，再将新测试 bundle 显式绑定到本次合成根并标准启动，完成深浅主题、四种导航、完整顶栏 / 拖拽、浮层、登录方式 / 错误、矮窗口、预览 / 通知和 Dock / 台前调度恢复连续帧验收。系统 Touch ID / Keychain 和辅助功能仍有当前用户边界。此处 preflight 不证明上述能力，任务总数保持 8 完成、6 待验证，未提交推送或发布。
