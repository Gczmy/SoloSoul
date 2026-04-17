# SoloSoul (独灵) 🧩

Your Local Digital Twin & Universal Identity Engine.

「独奏生命数据，重塑数字原点」—— 一个去中心化、本地加密的个人超级档案与自动化执行引擎。

它是 SlotGo 等自动化抢票/预约工具的"数字大脑"和"安全弹药库"。

---

## 项目架构

```
SoloSoul/
├── flutter/           # 主项目：跨平台客户端 (macOS/iOS/Android/Windows)
│   ├── lib/
│   │   ├── core/
│   │   │   └── services/   # 加密、存储、日志服务
│   │   └── presentation/
│   │       ├── pages/       # 页面
│   │       ├── providers/    # Riverpod 状态管理
│   │       ├── widgets/     # UI 组件
│   │       └── theme/       # 主题
│   ├── native/              # Rust 原生库
│   └── macos/              # macOS Runner
├── web/              # 遗留项目：Next.js Web UI (维护模式)
├── cmd/              # Go 后端服务
│   ├── solosould/   # HTTP API 服务器
│   └── solosoul/    # CLI 工具
└── docs/            # 文档
```

---

## 已完成

### Flutter 客户端 (主项目) ✅

**核心加密** ✅
- Rust Argon2id FFI (64MB, 3 iterations) - Apple Silicon SIMD 加速
- Dart FFI 绑定
- AES-256-GCM 加密/解密
- Secure memory 处理

**账户与安全** ✅
| 功能 | 状态 |
|------|------|
| 账户创建/解锁 | ✅ 绕过 Keychain 秒进 |
| 密码提示词 | ✅ |
| 账户列表折叠/展开 | ✅ |
| 账户删除 | ✅ |
| 数据敏感级别 | ✅ Public/Private/Restricted |
| Privacy Shield 掩码 | ✅ |
| Operation Log | ✅ |
| 回收站 (30天自动清理) | ✅ |

**Profile 页面** ✅
| 页面 | 路由 | 功能 |
|------|------|------|
| SplashPage | / | 启动页 |
| LoginPage | /login | 账户选择 + 密码登录 |
| HomePage | /home | 主页面 |
| ProfilePage | /profile | Contact Info, Identity, Addresses |
| TravelPage | /travel | Passports, Visas, Travel History |
| FinancialPage | /financial | Bank Accounts, Cards, Tax IDs |
| ProfessionalPage | /professional | Education, Employment, Skills |
| SettingsPage | /settings | Account, Security, Sync, App Info |
| OperationLogPage | /operation-log | 操作记录 |
| SensitivitySettingsPage | /sensitivity-settings | 敏感级别设置 |

**UI 组件** ✅
- SectionCard / CollapsibleSectionCard
- SensitiveValueWidget (分级掩码)
- 操作提示条 (Toast/Snackbar)
- PasswordVerificationDialog (共享密码验证)

**跨平台构建** ✅
- macOS Release: 43.7MB

### Go 后端 (solosould) ✅

**API 服务** ✅
- Vault 服务 (Unlock/Lock/ChangePassword)
- Profile 服务 (Get/Update/Validate/List/Delete)
- Field 服务 (GetFields/SetFields)
- Plugin 管理系统
- OCR API 端点
- Session Token 管理

**CLI 工具** ✅
- `init`, `unlock`, `lock`, `status`, `profile`

### Web UI (遗留，维护模式) ✅

- Next.js 15 + App Router
- 登录与设置、仪表盘、档案编辑器
- Plugin 管理、OCR 扫描页

---

## 待完成

### P0: 关键问题

1. **Flutter Keychain Entitlements**
   - macOS/iOS Keychain 权限配置
   - Release 前恢复 `flutter_secure_storage` 正常使用

### P1: 安全

2. **物理安全**
   - 防截屏 (Android FLAG_SECURE, iOS snapshot blur)
   - 多任务视图模糊

### P2: 云同步

3. Online/Offline 标识修复
4. Offline 后台自动重连
5. Offline 标识改为手动连接按钮
6. 云服务器开发
7. 隐私政策/服务条款更新

### P3: 跨平台构建

| 平台 | Touch ID | Keychain | App Store |
|------|----------|----------|-----------|
| macOS | 待做 | 待恢复 | - |
| iOS | 待做 | 待配置 | 待发布 |
| Android | 待做 | 待配置 | 待发布 |
| Windows | 待做 | Credential Manager | 待发布 |

### P4: Web 迁移/移除

- Web UI 功能考虑迁移到 Flutter 或移除

---

## 开发命令

### Flutter

```bash
cd flutter

# 安装依赖
flutter pub get

# 开发模式
flutter run

# Release 构建 (macOS)
flutter build macos --release

# 代码分析
dart analyze
```

### Go 后端

```bash
# 构建 (需要 Rust)
go build -tags "rust,cgo" ./...

# 构建 (纯 Go)
go build ./...

# 运行 API 服务器
./solosould

# CLI
./solosoul status
```

### Web

```bash
cd web
npm install
npm run dev
```

---

## 技术栈

| 组件 | 技术 |
|------|------|
| Flutter 客户端 | Dart, Riverpod, flutter_riverpod |
| Rust 核心 | Rust, Argon2id, AES-256-GCM, rusqlite |
| Go 后端 | Go, Gin |
| Web UI | Next.js 15, React, Zustand |
| 加密 | Argon2id (64MB, 3 iterations), AES-256-GCM |

---

## 安全特性

- **零知识架构**: Master Password 从不存储
- **本地加密**: 所有数据 AES-256-GCM 加密
- **敏感分级**: Public/Private/Restricted 三级保护
- **隐私盾**: Privacy Shield 一键掩码
- **阅后即焚**: Secure memory 处理
- **端到端加密**: 云同步时服务端不解密

---

## 文档

| 文档 | 说明 |
|------|------|
| [TODO](docs/TODO.md) | 开发任务清单 |
| [USER_GUIDE](docs/USER_GUIDE.md) | 用户指南 |
| [CLIENT_ROADMAP](docs/CLIENT_ROADMAP.md) | 客户端路线图 |
| [CHANGELOG](docs/CHANGELOG.md) | 变更日志 |

---

## 许可证

Private - All Rights Reserved
