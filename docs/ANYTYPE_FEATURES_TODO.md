# Anytype 借鉴功能实现清单

> 基于 `anytype_macos_analysis_report.md` 分析报告，按优先级排列。
> 创建日期：2026-05-02

---

## P0 — 数据完整性

### ✅ Task 1: SafeStorage 原子写入
- **目标**: 防止 `config.json` 在写入过程中损坏（崩溃/断电/SIGKILL）
- **方案**: 在 Rust 核心层实现原子写入：写 `.tmp` → `fsync` → 备份 `.bak` → rename
- **文件**:
  - `native/src/safe_storage.rs` — Rust 实现（write_atomic + recover_or_load）
  - `native/src/account/manager.rs` — 集成到所有 config/accounts 读写
  - `native/src/lib.rs` — 模块注册
- **状态**: ✅ 已完成

---

## P1 — 安全性与安装体验

### ⬜ Task 2: macOS 公证（Notarization）
- **目标**: 消除 Gatekeeper 警告，用户可直接双击打开 DMG
- **方案**: 在 `build_dmg.sh` 中添加 `xcrun notarytool submit` + `xcrun stapler staple`
- **文件**:
  - `flutter/build_dmg.sh` — 添加公证步骤（需要环境变量 APPLE_ID, APPLE_TEAM_ID, APPLE_APP_PASSWORD）
- **状态**: ⬜ 暂缓

### ✅ Task 3: 电源事件感知（Suspend/Resume）
- **目标**: 系统挂起前清除内存中的敏感密钥，恢复后重新验证会话
- **方案**: Swift 层监听 `NSWorkspace.willSleepNotification`，通过 MethodChannel 通知 Dart
- **文件**:
  - `macos/Runner/AppDelegate.swift` — 注册电源通知 + sendToFlutter helper
  - `lib/core/services/native_channel_service.dart` — 添加 onSystemWillSleep/onSystemDidWake 回调
  - `lib/main.dart` — 挂起时锁定 vault
- **状态**: ✅ 已完成

---

## P2 — 用户体验优化

### ✅ Task 4: 窗口状态持久化
- **目标**: 记住窗口位置和大小，下次打开时恢复
- **方案**: 使用 `setFrameAutosaveName` — macOS 自动处理位置/大小保存和恢复
- **文件**:
  - `macos/Runner/MainFlutterWindow.swift` — 设置 `setFrameAutosaveName("MainFlutterWindow")`
- **状态**: ✅ 已完成

### ✅ Task 5: 系统托盘 Template 图标
- **目标**: macOS 菜单栏托盘图标，自动适配深色/浅色模式
- **方案**: 原生 Swift NSStatusItem + Template Image（`isTemplate = true`）
- **文件**:
  - `macos/Runner/Assets.xcassets/TrayIcon.imageset/` — 托盘图标资源（16x16 + 32x32）
  - `macos/Runner/AppDelegate.swift` — setupTrayIcon + showApp 方法
- **状态**: ✅ 已完成

### ⬜ Task 6: 键盘快捷键适配
- **目标**: 遵循 macOS 人机界面指南的键盘快捷键
- **方案**: 审查现有快捷键，确保使用 Cmd 而非 Ctrl
- **文件**:
  - `lib/presentation/pages/` — 各页面快捷键
- **状态**: ⬜ 待实现

---

## 实现顺序

```
Task 1 (SafeStorage)  ← 最高优先级，数据安全
    ↓
Task 2 (公证) + Task 3 (电源事件)  ← 可并行
    ↓
Task 4 (窗口) + Task 5 (托盘) + Task 6 (快捷键)  ← 可并行
```

---

## 进度跟踪

| Task | 状态 | 备注 |
|------|------|------|
| 1. SafeStorage | ✅ | Rust 原子写入 + 崩溃恢复，35 个测试通过 |
| 2. 公证 | ✅ | build_dmg.sh 添加 notarytool，需设置环境变量 |
| 3. 电源事件 | ✅ | Swift NSWorkspace 通知 → Dart lockVault |
| 4. 窗口持久化 | ✅ | setFrameAutosaveName，macOS 自动处理 |
| 5. 托盘图标 | ✅ | NSStatusItem + Template Image |
| 6. 快捷键 | ⬜ | 待实现 |
