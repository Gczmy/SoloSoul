<div align="center">

# SoloSoul · 独灵 🧩

**Your local digital twin & universal identity engine.**
**独奏生命数据，重塑数字原点。**

![macOS](https://img.shields.io/badge/macOS-Apple_Silicon-333333?logo=apple&logoColor=white)
![Windows](https://img.shields.io/badge/Windows-x64-0078D6?logo=windows&logoColor=white)
![Android](https://img.shields.io/badge/Android-APK-3DDC84?logo=android&logoColor=white)
![License](https://img.shields.io/badge/license-MIT-blue)
![Version](https://img.shields.io/badge/version-2.9.2-brightgreen)

> **本地优先 · 隐私优先 · 零知识** 的个人数字孪生与通用身份引擎。
>
> 「Centralized Schema definition, decentralized data storage.」
> 集中式 Schema 定义，去中心化数据存储。

**立即下载 / Quick Download**：[macOS](README.md#平台与安装) · [Windows](README.md#平台与安装) · [Android](README.md#平台与安装)

</div>

---

## 为什么是 SoloSoul？

你的数字生活有一个悖论：**你创造了海量的个人数据，但你几乎不拥有它们。**

- 通讯录锁在微信里，旅行记录散落在携程、飞猪、航旅纵横
- 银行流水在各家 App 里，没有一份统一的「你的财务状况」
- 护照、身份证、驾照躺在抽屉里，也躺在每个需要实名认证的网站上
- 职业履历在 LinkedIn、Boss 直聘、猎聘上——每一家都只有片段

这些数据被**割裂**（每个应用都有自己的数据模型）、被**锁定**（进去了就出不来）、被**监控**（每一次点击和搜索都被记录变现）。

SoloSoul 的信念是：**数据不应该被应用割裂。** 应该有一个统一的 Schema 定义「什么是护照」「什么是交易」「什么是联系人」——然后这些数据只存在于你的设备上，由你完全控制。

这不是又一个笔记应用，也不是又一个密码管理器。SoloSoul 管理的是**结构化的「你」**：你的身份、你的旅行、你的财务、你的专业履历——每个字段都有类型、有验证规则、有敏感度分级。

---

## 核心特性

### 🧠 你的数字孪生

以对象为单位的个人档案系统，告别散落各处的信息碎片。

| 能力 | 说明 |
|------|------|
| **对象系统** | 身份、旅行、财务、履历……一切皆对象，扁平化管理，支持标签 |
| **模板引擎** | 自定义对象模板，8+ 字段类型，一次定义处处复用 |
| **历史快照** | 每次修改自动保存快照，随时回滚 + diff 摘要 |
| **回收站** | 软删除 30 天保护期，批量恢复 / 永久删除，冲突检测 |

### 🔐 安全默认

安全不是可选项，是默认状态。所有数据在离开你的手指之前加密。

- **零知识架构** — 主密码从不离开设备，只在内存中派生密钥（Argon2id），用完即毁
- **AES-256-GCM 全量加密** — 所有存储数据，包括附件
- **敏感度四级分级** — Public → Internal → Sensitive → Critical，自动掩码 / 遮盖 / 重新验证
- **生物识别** — Touch ID / Face ID 快速解锁，底层始终有主密码兜底
- **自动锁定** — 应用切后台超时自动锁定，密钥即刻擦除

### 🤖 本地 AI 与工具

AI 在本地或你指定的端点运行——绝不上传原始数据。

- **AI 对话** — 多 Provider（OpenAI / Anthropic / Ollama / 自定义），流式响应
- **OCR 识别** — 本地图像文字识别 + MRZ 护照 / 证件解析（PP-OCRv6 / Apple Vision）
- **附件系统** — 上传、预览、下载、重命名，全部加密存储
- **加密导出 / 导入** — `.solosoul` 开放格式，随时完整迁移，导出必须加密

### 🔄 设备同步

多设备之间的数据同步，不经过任何第三方服务器。

- **端到端加密通道** — Noise 协议握手，双向确认 + PIN 验证码配对
- **mDNS 局域网发现** — 设备自动发现，无需配置服务器
- **冲突可视化** — 字段级差异对比，保留本地 / 远程 / 忽略，一目了然
- **自动同步** — 切回前台、本地变更、周期三种触发方式，可持久化开关

### 🔍 掌控一切

- **全局搜索** — 本地索引毫秒响应，支持分类 / 标签 / 类型筛选
- **审计日志** — 每次 CRUD 的结构化操作记录，全字段国际化
- **i18n 双语** — en-US / zh-CN，覆盖所有页面与文案

---

## 平台与安装

| 平台 | 安装包 | 下载 |
|------|--------|------|
| macOS (Apple Silicon) | `.dmg` | [⬇ 下载 macOS 版](https://github.com/Gczmy/SoloSoul/releases/latest/download/SoloSoul_macOS.dmg) |
| Windows (x64) | `.exe` (NSIS) | [⬇ 下载 Windows 版](https://github.com/Gczmy/SoloSoul/releases/latest/download/SoloSoul_Windows.exe) |
| Android | `.apk` (universal) | [⬇ 下载 Android 版](https://github.com/Gczmy/SoloSoul/releases/latest/download/SoloSoul_Android.apk) |

点击上方链接即可直接下载对应平台的最新安装包，无需在 Release 页面挑选文件。链接使用**无版本号的稳定文件名**（如 `SoloSoul_macOS.dmg`），发布新版本时会同步更新同名资产，链接始终指向最新版。Release 页面中其余文件（`latest.json`、`.sha256`、`.minisig` 等）为自动更新与校验元数据，普通用户无需下载。安装后创建本地账户（无需邮箱），设置主密码即可开始。

> [!IMPORTANT]
> **macOS 安装提醒**：macOS 版本暂未通过 Apple 开发者认证与公证（暂无开发者账号），首次打开会被 Gatekeeper 拦截。请任选以下一种方式解除：
>
> **方式一：系统设置手动允许（推荐）**
>
> 1. 双击打开 `.dmg` 并拖入「应用程序」；
> 2. 首次双击 `SoloSoul.app` 被拦截后，打开 **系统设置 → 隐私与安全性**；
> 3. 在「安全性」区域找到 *「“SoloSoul”已被阻止使用，因为来自身份不明的开发者」* 提示；
> 4. 点击 **「仍要打开」**，输入管理员密码确认即可。
>
> **方式二：终端移除隔离标记**
>
> 在终端中执行（可先输入 `xattr -rd com.apple.quarantine ` 再拖入 `.app` 自动补全路径）：
>
> ```bash
> xattr -rd com.apple.quarantine /Applications/SoloSoul.app
> ```
>
> 随后正常双击即可打开。

### 终端用户：SoloSoul CLI

配套的终端 TUI 客户端 `solosoul`，支持账户管理、对象 CRUD、搜索、历史回滚、审计日志、加密导入导出等 30+ 命令。详见 [CLI 用户指南](docs/solosoul_cli/USER_GUIDE.md)。

---

## 我们对你的承诺

SoloSoul 与用户之间不是「服务商与客户」，而是「**工具提供者与技术使用者**」。我们写下了 [用户契约](docs/manifesto/03_用户契约.md)，白纸黑字：

- **你的数据，只存在于你的设备上** — 没有云端备份，没有后台分析
- **我们无法读取你的数据** — 零知识架构，即使开发者想看也看不到
- **不追踪，不分析，不画像** — 零遥测，连「未读小红点」都不需要联网
- **删除就是删除** — 没有你不知情的隐藏副本
- **没有强制，没有锁定** — 不需要邮箱注册，可以永远使用当前版本，随时完整导出离开

**如果你丢失主密码，我们无法帮你恢复。** 这不是冷漠，这是零知识架构的必然结果——请定期导出加密备份。

---

## 文档

| 文档 | 说明 |
|------|------|
| [用户契约](docs/manifesto/03_用户契约.md) | 我们对用户做出的八项承诺 |
| [产品使命](docs/manifesto/01_产品使命.md) | SoloSoul 为什么存在 |
| [设计哲学](docs/manifesto/02_设计哲学.md) | 我们做产品决策的依据 |
| [CLI 用户指南](docs/solosoul_cli/USER_GUIDE.md) | 终端客户端使用说明 |
| [CHANGELOG](CHANGELOG.md) | 详细变更日志 |
| [开发者文档](docs/DEVELOPMENT.md) | 项目结构、技术栈、构建与发布 |

---

## 开源协议

SoloSoul 基于 **MIT License** 发布——你可以自由使用、修改、分发，甚至分叉。

SoloSoul is released under the **MIT License**. See the [LICENSE](LICENSE) file for details.
