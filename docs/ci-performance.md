# CI 耗时优化与验证

其他重构和原生验收任务保持暂停。本轮只优化 CI 执行方式，保留现有测试与检查门槛。

## CI-001：修正 Cargo workspace 缓存

- 修复 `pr_check.yml` 的 Apple 检查和 `ci_cd.yml` 的 Linux 检查：缓存根目录由 `tauri/src-tauri` 改为 `tauri`，覆盖实际 `tauri/target` 和 workspace 的 `Cargo.lock`。
- 使用新缓存前缀 `v1-tauri-workspace`，避免沿用旧配置产生的缓存。
- PR Checks 在 main push 时仅执行原有 Apple Rust job，为后续 PR 预热相同 job key 的缓存。其余检查仍由原有 CI/CD 执行，不重复运行。
- 保留两个 iOS 目标顺序检查、macOS Clippy、future-keychain、完整 Rust 测试、ACL 和既有资源准备步骤。
- 本地验证：检查工作流语法、Cargo metadata 的 workspace/target 路径和改动范围；远端验证比较同一提交首次运行与重跑的缓存状态和步骤耗时。

## 基线

PR #18，run `39683652914`：

| 项目 | 耗时 |
| --- | --- |
| Apple Rust job | 44分35秒 |
| iOS 两个目标检查 | 10分18秒 |
| macOS Clippy | 4分35秒 |
| future-keychain check | 1分05秒 |
| Rust 测试步骤（含编译） | 26分01秒 |
| 其中 test profile 编译 | 23分44秒 |
| 前端 job | 24分42秒 |
| 生产浏览器测试：1,315 项、1 worker | 18.9分钟 |

旧日志缓存目录为 `tauri/src-tauri/target`，实际 rustc 输出目录为 `tauri/target/debug/deps`；该次缓存未命中。

## 比较方法

1. 首次运行确认新前缀没有可恢复缓存，作为冷缓存样本。
2. 确认成功保存实际 target 中的依赖产物后，重跑同一提交的同一 Rust job，作为热缓存样本。
3. 记录 runner、Rust 版本、cache key、命中状态、恢复/保存时间、编译时间和 job 总耗时。相同用例数及结果是比较前提。
4. 浏览器分片另行记录所有分片用例并集、重复项、测试结果及最慢分片耗时，不将机器排队时间当作测试耗时。

远端实测结果在运行完成后补入。
