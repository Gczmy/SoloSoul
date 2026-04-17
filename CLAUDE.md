# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Project Overview

SoloSoul (独灵) is a **Local Digital Twin & Universal Identity Engine** - a decentralized, local-encrypted personal super-profile and automation execution engine.

**Core Philosophy**: "Centralized Schema definition, decentralized data storage"

## 项目架构

```
SoloSoul/
├── flutter/           # 主项目：跨平台客户端
│   ├── lib/
│   │   ├── core/services/    # 核心服务
│   │   └── presentation/     # UI 层
│   └── native/              # Rust 原生库
├── web/              # 遗留项目：Next.js Web UI (维护模式)
├── cmd/              # Go 后端服务
│   ├── solosould/    # HTTP API 服务器
│   └── solosoul/     # CLI 工具
└── docs/             # 文档
```

## 安全要求 (Zero-Knowledge)

- Master Password **从不存储** - 仅在内存中用于密钥派生
- Salt 存储在 `~/.solosoul/{account_id}/config.json` 用于密钥验证
- 敏感字段使用后**销毁** (secure zeroing)
- 外部插件访问需要用户**显式授权** (Consent Manager)
- Session tokens 24小时过期
- 数据仅本地存储于 `~/.solosoul/`，**绝不上传云端**
- 每个账户独立加密在 `~/.solosoul/acc_xxx123/`

## 开发命令

### Flutter

```bash
cd flutter

# 分析代码
dart analyze

# 运行
flutter run

# Release 构建
flutter build macos --release
```

### Go 后端

```bash
# 构建 (需要 Rust)
go build -tags "rust,cgo" ./...

# 构建 (纯 Go)
go build ./...

# 运行
./solosould
```

### Web

```bash
cd web
npm install
npm run dev
```

## 技术栈

| 组件 | 技术 |
|------|------|
| Flutter 客户端 | Dart, Riverpod, flutter_riverpod |
| Rust 核心 | Rust, Argon2id, AES-256-GCM |
| Go 后端 | Go, Gin |
| Web UI | Next.js 15, React, Zustand |

## Flutter 项目结构

```
flutter/lib/
├── core/
│   └── services/
│       ├── native_crypto_service.dart      # Rust FFI 加密
│       ├── profile_storage_service.dart    # Profile 存储
│       ├── secure_storage_service.dart     # 安全存储 (临时)
│       ├── rust_vault_service.dart         # Rust Vault
│       ├── biometric_service.dart          # 生物识别
│       ├── operation_logger.dart           # 操作日志
│       └── ...
├── presentation/
│   ├── pages/              # 页面
│   ├── providers/           # Riverpod providers
│   ├── widgets/            # UI 组件
│   └── theme/              # 主题
└── main.dart
```

### Flutter 页面路由

| 页面 | 路由 |
|------|------|
| LoginPage | /login |
| HomePage | /home |
| ProfilePage | /profile |
| TravelPage | /travel |
| FinancialPage | /financial |
| ProfessionalPage | /professional |
| SettingsPage | /settings |

## 关键约定

### 密码验证
- 使用共享组件 `lib/presentation/widgets/password_verification_dialog.dart`
- 不要在多个地方复制密码对话框代码

### 敏感数据
- `SensitivityLevel`: public / private / restricted
- `SensitiveValueWidget`: 自动掩码组件
- 验证后有 1 分钟缓存

## 项目状态

### Flutter 客户端 (主项目) ✅
- 核心加密 (Argon2id + AES-256-GCM) ✅
- Profile 页面 (Profile/Travel/Financial/Professional) ✅
- 数据敏感级别 ✅
- 操作记录与回收站 ✅
- macOS Release Build ✅

### Go 后端 ✅
- Vault/Profile/Plugin API ✅
- CLI 工具 ✅

### Web (遗留) ✅
- Next.js Web UI (维护模式)

### 待完成
- Flutter Keychain 恢复 (P0)
- 云同步开发
- iOS/Android/Windows 平台

## 文档

- [TODO](docs/TODO.md) - 开发任务清单
- [USER_GUIDE](docs/USER_GUIDE.md) - 用户指南
- [CLIENT_ROADMAP](docs/CLIENT_ROADMAP.md) - 客户端路线图
