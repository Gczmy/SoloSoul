# L9 构建工具与开发环境

> **层级定位**：Flutter 主项目的构建系统、CI/CD 流水线、开发工具链。这些属于支撑层，不直接参与运行时功能，但对开发效率和发布质量至关重要。
>
> **本文档目的**：记录现有构建与发布流程，便于 Tauri 迁移时对照，确保新的构建体系覆盖同等能力。

---

## 目录

- [L9.1 构建系统](#l91-构建系统)
- [L9.2 CI/CD 流水线](#l92-cicd-流水线)
- [L9.3 开发工具链](#l93-开发工具链)
- [L9.4 废弃组件说明](#l94-废弃组件说明)
- [Tauri 迁移对照](#tauri-迁移对照)

---

## L9.1 构建系统

### Flutter 构建

**macOS Release 构建**：
```bash
flutter build macos --release --obfuscate --split-debug-info=./debug_info/macos
```

**Android / iOS / Windows**：待适配，当前以 macOS 为主。

**Rust Native 库构建**：
```bash
cd flutter/native
cargo build --release
```

输出：
- macOS: `native/target/release/libnative.a` / `.dylib`
- iOS: `libnative.a`（静态库）
- Android: `libnative.so`

### DMG 打包

```bash
./build_dmg.sh
# 输出: flutter/build/macos/SoloSoul-v1.0.dmg
```

---

## L9.2 CI/CD 流水线

### ci_cd.yml（Push 到 master/main 或 PR）

1. **rust-test**（ubuntu-latest）— `flutter/native` 的 `cargo test`
2. **dart-unit-test**（ubuntu-latest）— `flutter test test/unit/`
3. **widget-test**（ubuntu-latest）— `flutter test test/widget/`
4. **integration-test**（macos-latest）— 构建 Rust native lib 后运行集成测试
5. **release**（macos-latest，仅 master push）— Release 构建、DMG 打包、Draft Pre-release 发布

### pr_check.yml（PR 快速反馈）

1. **rust-check** — `cargo fmt --check` + `cargo clippy -- -D warnings`
2. **dart-check** — `dart analyze --fatal-infos --fatal-warnings`
3. **test** — Rust 测试 + Dart 单元测试 + Widget 测试

---

## L9.3 开发工具链

| 工具 | 版本 | 用途 |
|------|------|------|
| Flutter | 3.41.6 | UI 框架 |
| Dart | 3.6 | 编程语言 |
| Rust | stable | 原生核心 |
| Xcode | 16 | macOS/iOS 构建 |
| CocoaPods | latest | iOS 依赖管理 |

---

## L9.4 废弃组件说明

以下组件已废弃并从代码库中移除，不再参与 Tauri 迁移：

| 组件 | 原路径 | 说明 |
|------|--------|------|
| Go HTTP API 服务器 | `cmd/solosould/` | 独立 HTTP 服务端，与 Flutter 客户端无直接耦合 |
| Go CLI 客户端 | `cmd/solosoul/` | 命令行工具，操作本地 Vault |
| Go 业务核心库 | `core/` | HTTP Server、AccountManager、PluginManager 等 |
| Web UI | `web/` | Next.js 15 + React 19 前端，调用 Go 后端 |
| crypto-argon2 | `crypto-argon2/` | 供 Go 后端 CGO 调用的 Rust FFI 库 |

> **注意**：Flutter 主项目的架构始终是 **Flutter（Dart UI）+ Rust（原生核心，通过 flutter_rust_bridge FFI）**，Go 后端和 Web UI 是独立的遗留交付物，从未被 Flutter 客户端直接依赖。

---

## Tauri 迁移对照

| 当前 Flutter | Tauri 重构 | 说明 |
|-------------|-----------|------|
| `flutter build macos` | `npm run tauri build` | Tauri 构建命令 |
| `flutter/native/` Rust | `src-tauri/src/` + `crates/` | Rust 核心迁移 |
| flutter_rust_bridge FFI | `tauri::command` IPC | 通信方式替换 |
| `cargo test` (native) | `cargo test --workspace` | Rust 测试保留增强 |
| `flutter test` | Vitest + Playwright | 前端测试迁移 |
| DMG 打包 | Tauri 内置 bundler | `src-tauri/target/release/bundle/dmg/` |
| `.github/workflows/ci_cd.yml` | `.github/workflows/ci.yml` | CI 流水线重构 |

---

*文档版本：v2.0（已移除 Go/Web 相关内容）*  
*修改日期：2026-06-04*
