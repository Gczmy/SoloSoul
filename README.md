# SoloSoul (独灵) 🧩

**Your Local Digital Twin & Universal Identity Engine.**

「独奏生命数据，重塑数字原点」—— 一个去中心化、本地加密的个人超级档案与自动化执行引擎。

> **核心理念**：「Centralized Schema definition, decentralized data storage」

---

## 项目架构

```
SoloSoul/
├── tauri/                     # 主项目：Tauri + React 跨平台客户端
│   ├── src/                   # React 前端 (TypeScript, Zustand, CSS Modules)
│   │   ├── components/        # UI 组件
│   │   ├── pages/             # 页面
│   │   │   ├── ai/            # AI 对话
│   │   │   ├── auth/          # 启动/登录
│   │   │   ├── editor/        # 对象/模板编辑器
│   │   │   ├── scan/          # OCR/本地扫描
│   │   │   ├── settings/      # 设置页
│   │   │   ├── system/        # 关于/调试
│   │   │   ├── workspace/     # 对象工作区
│   │   │   └── ...
│   │   ├── stores/            # Zustand 状态管理
│   │   ├── lib/               # 工具库 (i18n, IPC, theme)
│   │   ├── hooks/             # React Hooks
│   │   ├── locales/           # 国际化 (en-US / zh-CN)
│   │   ├── types/             # TypeScript 类型
│   │   └── styles/            # 全局 CSS
│   └── src-tauri/             # Rust 后端 (Tauri)
│       ├── src/
│       │   ├── commands/      # IPC 命令 (30+)
│       │   ├── core/          # 核心逻辑 (SensitivityManager, etc.)
│       │   ├── db/            # SQLite 数据库 + 迁移
│       │   ├── ipc/           # IPC 通信
│       │   ├── services/      # 业务服务 (vault, llm_context, sync)
│       │   └── state/         # 应用状态
│       └── crates/            # Workspace crates
├── SoloSoul_plugin_market/    # 插件市场 (Git Submodule)
├── docs/                      # 文档 (构建脚本、用户指南、法律文件)
└── CHANGELOG.md               # 变更日志
```

---

## 已完成

### 核心系统 ✅

| 模块 | 状态 | 说明 |
|------|------|------|
| 对象系统 | ✅ | Create/Read/Update/Delete，扁平化管理，支持标签 |
| 模板引擎 | ✅ | 用户自定义对象模板，8+ 字段类型 |
| 历史快照 | ✅ | 每次修改自动保存快照，支持版本回滚 + diff 摘要 |
| 回收站 | ✅ | 软删除/恢复/永久删除，冲突检测，批量操作 |
| 搜索 | ✅ | 全文本搜索，支持分类/标签/类型筛选 |
| 操作日志 | ✅ | 完整的结构化审计日志，全字段国际化 |

### 安全 ✅

| 功能 | 说明 |
|------|------|
| Argon2id 密钥派生 | 默认 8MiB/2 iter (开发) / 64MiB/3 iter (生产) |
| AES-256-GCM 加密 | 所有数据本地加密存储 |
| Master Password 零存储 | 仅在内存中用于密钥派生 |
| 敏感度分级 | Public / Internal / Private / Sensitive / Restricted / Critical |
| Privacy Shield | 一键掩码敏感数据 |
| 生物识别 | Touch ID / Face ID 解锁 |

### AI 与工具 ✅

| 功能 | 说明 |
|------|------|
| AI 对话 | 多 Provider (OpenAI/Anthropic/Ollama/自定义)，流式响应 |
| 附件系统 | 上传/预览/下载/重命名/软删除/全部加密 |
| OCR 识别 | 本地图像 OCR + MRZ 护照/证件解析 |
| 本地扫描 | 文件系统扫描与索引 |
| 导出/导入 | 加密导出，支持标签筛选与附件 |

### 跨平台 ✅

| 平台 | 安装包 | 状态 |
|------|--------|------|
| macOS (Apple Silicon) | DMG | ✅ 可用 |
| Windows (x64) | NSIS (.exe) | ✅ 可用 |
| Linux | AppImage | ⏳ 待测试 |

### 国际化 i18n ✅

- en-US / zh-CN 双语言
- 覆盖所有页面：编辑器、认证、设置、布局、对象工作区、AI、OCR 等
- 字段标签、验证消息、操作日志、相对时间全部翻译

---

## 待完成

### P0

1. **签名与分发**
   - macOS 代码签名 + 公证 (Developer ID)
   - Windows Authenticode 签名

### P1

2. **物理安全**
   - 防截屏保护
   - 多任务视图模糊

### P2

3. **云同步**
   - 多设备同步引擎
   - 冲突解决

### P3

4. **插件生态系统**
   - 插件运行时 v2 适配
   - 插件市场集成

---

## 开发命令

```bash
cd tauri

# 开发模式
npm run dev

# 代码检查 (TypeScript + ESLint)
npm run check-all

# Release 构建
npm run tauri build
```

---

## 技术栈

| 组件 | 技术 |
|------|------|
| 前端框架 | React 19, TypeScript, Vite |
| 状态管理 | Zustand |
| 样式 | CSS Modules + 全局 CSS 变量 |
| 国际化 | i18next + react-i18next |
| 后端框架 | Tauri v2 (Rust) |
| 数据库 | SQLite (rusqlite) |
| 密码学 | Argon2id, AES-256-GCM |
| OCR | PP-OCRv6 (ONNX Runtime) |

---

## 安全特性

- **零知识架构** — Master Password 从不存储，仅在内存中派生密钥
- **本地优先** — 数据仅存储在 `~/.solosoul/`，绝不上传云端
- **全量加密** — AES-256-GCM 加密所有存储数据
- **敏感分级** — 三阶敏感度 (Public / Private / Restricted)
- **安全内存** — 敏感字段使用后销毁 (secure zeroing)
- **Session 过期** — 24 小时自动过期

---

## 构建与发布

详见 [`docs/release_process.md`](docs/release_process.md)。

发布流程：
1. **准备** — 统一版本号（3 个文件同步）
2. **构建** — macOS 产 DMG，Windows 产 NSIS `.exe`
3. **发布** — 上传至 GitHub Releases，更新 CHANGELOG

---

## 文档

| 文档 | 说明 |
|------|------|
| [Release 流程](docs/release_process.md) | 发布构建与 GitHub Release 流程 |
| [TODO](docs/TODO.md) | 开发任务清单 |
| [CHANGELOG](CHANGELOG.md) | 详细变更日志 |

---

## 许可证

SoloSoul 基于 **MIT License** 发布。详见 [LICENSE](LICENSE) 文件。

SoloSoul is released under the **MIT License**. See the [LICENSE](LICENSE) file for details.
