# SoloSoul (独灵) 🧩

Your Local Digital Twin & Universal Identity Engine.

「独奏生命数据，重塑数字原点」—— 一个去中心化、本地加密的个人超级档案与自动化执行引擎。

---

## 项目架构

```
SoloSoul/
├── tauri/            # 主项目：Tauri + React 跨平台客户端 (macOS/Windows/Linux)
│   ├── src/          # React 前端源码
│   ├── src-tauri/    # Rust 后端 (Tauri)
│   │   ├── src/
│   │   │   ├── commands/   # IPC 命令
│   │   │   ├── core/       # 核心逻辑
│   │   │   ├── db/         # SQLite 数据库
│   │   │   ├── services/   # 业务服务
│   │   │   └── ...
│   │   └── crates/         # Workspace crates (crypto, vault, sync)
│   └── package.json
├── SoloSoul_plugin_market/  # 插件市场（Git Submodule）
│   ├── plugins/
│   ├── SDK/
│   └── ...
├── sdk/              # SDK 占位目录（未实现）
│   ├── js/
│   └── python/
└── docs/             # 文档
```

---

## 已完成

### Tauri 客户端 (主项目) ✅

**核心加密** ✅
- Rust Argon2id (64MB, 3 iterations)
- AES-256-GCM 加密/解密
- Secure memory 处理

**账户与安全** ✅
| 功能 | 状态 |
|------|------|
| 账户创建/解锁 | ✅ |
| 密码提示词 | ✅ |
| 账户列表折叠/展开 | ✅ |
| 账户删除 | ✅ |
| 数据敏感级别 | ✅ Public/Private/Restricted |
| Privacy Shield 掩码 | ✅ |
| Operation Log | ✅ |
| 回收站 (30天自动清理) | ✅ |

**Profile 页面** ✅
| 页面 | 功能 |
|------|------|
| 启动页 | ✅ |
| 登录页 | 账户选择 + 密码登录 |
| 主页 | 主页面 |
| 档案页 | Contact Info, Identity, Addresses |
| 旅行页 | Passports, Visas, Travel History |
| 财务页 | Bank Accounts, Cards, Tax IDs |
| 职业页 | Education, Employment, Skills |
| 设置页 | Account, Security, Sync, App Info |
| 操作记录 | 操作日志 |
| 敏感级别设置 | 敏感级别设置 |

**UI 组件** ✅
- SectionCard / CollapsibleSectionCard
- SensitiveValueWidget (分级掩码)
- 操作提示条 (Toast/Snackbar)
- PasswordVerificationDialog (共享密码验证)

**OCR 本地识别** ✅
- 护照 MRZ 扫描 → 自动创建 Travel Passport
- 通用文档/名片 OCR
- 完全本地处理，零网络依赖

**跨平台构建** ✅
- macOS Release
- Windows Release

---

## 待完成

### P0: 关键问题

1. **Tauri 平台适配**
   - macOS 代码签名与公证
   - Windows 签名

### P1: 安全

2. **物理安全**
   - 防截屏
   - 多任务视图模糊

### P2: 云同步

3. Online/Offline 标识修复
4. Offline 后台自动重连
5. Offline 标识改为手动连接按钮
6. 云服务器开发
7. 隐私政策/服务条款更新

### P3: 跨平台构建

| 平台 | 状态 |
|------|------|
| macOS | ✅ 可用 |
| Windows | ✅ 可用 |
| Linux | 待测试 |

---

## 开发命令

### Tauri 客户端

```bash
cd tauri

# 安装依赖
npm install

# 开发模式
npm run dev

# 代码检查
npm run check-all

# Release 构建
npm run tauri build
```

---

## 技术栈

| 组件 | 技术 |
|------|------|
| Tauri 客户端 | React 19, TypeScript, Vite, Zustand |
| Rust 核心 | Rust, Argon2id, AES-256-GCM, rusqlite, ONNX Runtime |
| 加密 | Argon2id (64MB, 3 iterations), AES-256-GCM |
| OCR | PP-OCRv4 (ONNX) / Apple Vision (iOS/macOS) |

---

## 安全特性

- **零知识架构**: Master Password 从不存储
- **本地加密**: 所有数据 AES-256-GCM 加密
- **敏感分级**: Public/Private/Restricted 三级保护
- **隐私盾**: Privacy Shield 一键掩码
- **阅后即焚**: Secure memory 处理
- **端到端加密**: 云同步时服务端不解密

---

## OCR 模型文件

OCR 功能依赖三个 ONNX 模型文件（共 ~15MB），已加入 `.gitignore`，通过 **GitHub Release Assets** 分发。

| 模型 | 大小 | 用途 | 获取方式 |
|------|------|------|---------|
| `ppocrv4_det.onnx` | ~4.5MB | 文本检测 | Release Assets |
| `ppocrv4_cls.onnx` | ~571KB | 方向分类 | Release Assets / paddle2onnx 转换 |
| `ppocrv4_rec.onnx` | ~10MB | 文本识别 | Release Assets |

首次构建前，将模型文件放置到 `tauri/src-tauri/resources/models/` 目录下。

详细下载脚本和转换指南见 [`docs/OCR_INTEGRATION_DESIGN.md`](docs/OCR_INTEGRATION_DESIGN.md)。

---

## 文档

| 文档 | 说明 |
|------|------|
| [TODO](docs/TODO.md) | 开发任务清单 |
| [USER_GUIDE](docs/USER_GUIDE.md) | 用户指南 |
| [CLIENT_ROADMAP](docs/CLIENT_ROADMAP.md) | 客户端路线图 |
| [CHANGELOG](docs/CHANGELOG.md) | 变更日志 |
| [OCR_INTEGRATION_DESIGN](docs/OCR_INTEGRATION_DESIGN.md) | OCR 集成技术设计文档 |

---

## 许可证

Private - All Rights Reserved
