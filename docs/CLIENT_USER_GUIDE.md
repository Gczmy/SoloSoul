# SoloSoul 客户端用户指南

> 本指南面向最终用户，详细说明 SoloSoul Flutter 客户端的安装、使用和安全功能。
>
> 最后更新: 2026-04-18

---

## 目录

1. [简介](#1-简介)
2. [安装与启动](#2-安装与启动)
3. [安全架构](#3-安全架构)
4. [页面导航](#4-页面导航)
5. [档案管理](#5-档案管理)
6. [数据敏感性](#6-数据敏感性)
7. [回收站](#7-回收站)
8. [故障排除](#8-故障排除)

---

## 1. 简介

SoloSoul (独灵) 是一个**本地数字孪生**引擎，通过本地加密确保您的个人身份信息始终处于您的控制之下。

### 核心特性

- **🔒 零知识安全** - Master Password 从不存储，仅在内存中使用
- **🏠 本地优先** - 所有数据加密存储在本地 `~/.solosoul/`
- **🍎 macOS 原生** - Flutter 构建，原生 macOS 体验
- **🔐 双重加密** - 应用层 AES-256-GCM + 存储层 SQLCipher

### 客户端 vs Web/后端

| 组件 | 技术 | 存储位置 |
|------|------|----------|
| **Flutter 客户端** (本指南) | Flutter + Rust Core | 本地 `~/.solosoul/` |
| **Go 后端** | Go + Gin | 可选云同步 |
| **Web UI** | Next.js | 维护模式 |

---

## 2. 安装与启动

### 系统要求

- **操作系统**: macOS 10.15+ (Catalina 或更高)
- **内存**: 建议 4GB+
- **磁盘**: 至少 200MB 可用空间

### 安装步骤

#### 方式一：Release 构建 (推荐)

```bash
# 下载最新 release
curl -L -o SoloSoul.dmg https://github.com/solosoul/solosoul/releases/latest/download/SoloSoul.dmg

# 挂载并安装
open SoloSoul.dmg
# 将 SoloSoul.app 拖入应用程序文件夹
```

#### 方式二：从源码构建

```bash
cd flutter

# 安装依赖
flutter pub get

# 运行开发版本
flutter run -d macos

# 构建 release
flutter build macos --release
```

### 首次启动

1. 打开 **SoloSoul.app**
2. 进入 **Setup** 页面
3. 设置您的 **Master Password**（至少 8 位）
4. 确认密码
5. 点击 **Create Account**

> **重要**: Master Password 无法找回！请务必妥善保管，建议使用密码管理器备份。

---

## 3. 安全架构

### 3.1 密钥派生

```
Master Password ──[Argon2id]──▶ Master Key (256-bit)
                                 │
                                 ├─── Session Key (派生用于当前会话)
                                 └─── Data Key (加密实际数据)
```

**Argon2id 参数**:
- Memory: 64 MB
- Iterations: 3
- Parallelism: 4

### 3.2 加密流程

```
用户输入密码
      │
      ▼
┌─────────────────┐
│ Argon2id 派生   │ ◀── 从不存储的 Master Password
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│   Master Key    │ ◀── 仅存在于内存
└────────┬────────┘
         │
         ▼
┌─────────────────┐     ┌─────────────────┐
│   加密 Profile  │ ──▶ │  ~/.solosoul/   │
│   (AES-256-GCM)│     │  加密存储       │
└─────────────────┘     └─────────────────┘
```

### 3.3 双重加密

| 层级 | 加密方式 | 防护目标 |
|------|----------|----------|
| 应用层 | AES-256-GCM | 防止云端泄露、传输被抓包 |
| 存储层 | SQLCipher | 防止磁盘镜像被提取 |

### 3.4 敏感数据处理

敏感字段（证件号、密码等）使用后会自动清零：

```dart
// 敏感数据使用后自动销毁
sensitiveField = null;  // 触发 secure zeroing
```

---

## 4. 页面导航

### 4.1 路由概览

```
/login          ──▶ 登录页
/home           ──▶ 首页仪表盘
/profile        ──▶ 身份档案
/travel         ──▶ 旅行档案
/financial      ──▶ 财务档案
/professional   ──▶ 职业档案
/settings       ──▶ 设置页
```

### 4.2 登录页 (LoginPage)

**路由**: `/login`

**功能**:
- **Unlock**: 使用 Master Password 解锁 vault
- **Setup**: 首次使用时创建密码

**安全特性**:
- 密码错误次数过多会触发锁定
- Master Password 从不存储

### 4.3 首页仪表盘 (HomePage)

**路由**: `/home`

**显示内容**:
- **Profile Complete**: 档案完整度百分比
- **Last Unlock**: 上次解锁时间
- **Accounts**: 已创建账户数量

### 4.4 身份档案 (ProfilePage)

**路由**: `/profile`

**字段**:
| 字段 | 敏感性 |
|------|--------|
| Full Name | Private |
| Given Name | Private |
| Family Name | Private |
| Date of Birth | Restricted |
| Gender | Private |
| Email | Private |
| Phone | Restricted |
| Address | Private |

### 4.5 旅行档案 (TravelPage)

**路由**: `/travel`

**字段**:
| 字段 | 敏感性 |
|------|--------|
| Passport Number | **Restricted** |
| Country | Private |
| Nationality | Private |
| Expiry Date | Restricted |
| Visa Number | **Restricted** |

### 4.6 财务档案 (FinancialPage)

**路由**: `/financial`

**字段**:
| 字段 | 敏感性 |
|------|--------|
| Bank Name | Private |
| Account Number | **Restricted** |
| Card Number | **Restricted** |
| Expiry Date | Restricted |
| CVV | **Restricted** |
| Tax ID | **Restricted** |

### 4.7 职业档案 (ProfessionalPage)

**路由**: `/professional`

**字段**:
| 字段 | 敏感性 |
|------|--------|
| Education | Private |
| Employer | Private |
| Job Title | Private |
| Income | **Restricted** |

### 4.8 设置页 (SettingsPage)

**路由**: `/settings`

**功能**:
- **Lock**: 立即锁定 vault
- **Change Master Password**: 修改主密码
- **Export Data**: 导出加密备份
- **Clear Trash**: 清空回收站

---

## 5. 档案管理

### 5.1 编辑档案

1. 进入对应档案页面 (Profile/Travel/Financial/Professional)
2. 点击 **Edit** 按钮
3. 修改字段
4. 点击 **Save** 保存

**注意**: 修改后会触发加密写入，可能需要几秒钟。

### 5.2 创建新档案

1. 进入 **Settings** 页面
2. 点击 **Create New Account**
3. 填写账户名称
4. 点击 **Create**
5. 新账户创建完成，自动切换到新账户

### 5.3 切换账户

1. 进入 **Settings** 页面
2. 选择 **Switch Account**
3. 选择目标账户
4. 输入该账户的 Master Password

---

## 6. 数据敏感性

SoloSoul 将数据分为四个敏感性级别：

| 级别 | 标识 | 说明 | 示例 |
|------|------|------|------|
| **Public** | 绿色 | 可自由查看 | Country, Gender |
| **Private** | 蓝色 | 需要验证后查看 | Name, Email, Phone |
| **Restricted** | 橙色 | 敏感信息，严格保护 | Passport No., Card No., Tax ID |
| **Internal** | 红色 | 仅内部使用 | Salt, Derived Keys |

### 6.1 查看受限字段

受限字段默认显示为 `••••••••`，需要验证 Master Password：

1. 点击字段旁边的 **Reveal** 按钮
2. 输入 Master Password
3. 字段值显示 1 分钟
4. 时间结束后自动重新隐藏

### 6.2 敏感性视觉提示

在编辑和查看页面，字段按颜色编码：
- 🔵 蓝色边框 - Private
- 🟠 橙色边框 - Restricted
- 🟢 绿色边框 - Public

---

## 7. 回收站

删除档案时，数据不会立即销毁，而是移入回收站。

### 7.1 查看回收站

1. 进入 **Settings** 页面
2. 点击 **Trash**
3. 查看所有已删除的档案

### 7.2 恢复档案

1. 进入 **Trash** 页面
2. 找到要恢复的档案
3. 点击 **Restore**
4. 档案恢复到正常状态

### 7.3 永久删除

**警告**: 此操作不可撤销！

1. 进入 **Trash** 页面
2. 点击 **Delete Permanently**
3. 确认删除
4. 数据永久销毁

### 7.4 清空回收站

1. 进入 **Settings** 页面
2. 点击 **Clear Trash**
3. 确认清空
4. 所有回收站数据永久销毁

---

## 8. 故障排除

### 8.1 忘记 Master Password

**无法恢复！** Master Password 不存储在任何地方。

**建议**:
- 使用 macOS Keychain 或密码管理器备份密码
- 如果确实忘记，只能删除 `~/.solosoul/` 重新开始

```bash
# 备份重要数据（如有）
cp -r ~/.solosoul ~/solosoul_backup

# 删除 vault
rm -rf ~/.solosoul

# 重新启动 SoloSoul
open -a SoloSoul
```

### 8.2 客户端无响应

```bash
# 强制退出
Cmd + Option + Escape
# 选择 SoloSoul 并强制退出

# 重启客户端
open -a SoloSoul
```

### 8.3 解锁失败

**可能原因**:
1. 密码错误
2. vault 文件损坏
3. 磁盘空间不足

**解决方案**:
1. 确认密码输入正确
2. 检查 `~/.solosoul/` 目录权限
3. 确保有足够磁盘空间

### 8.4 数据不同步

客户端数据存储在本地，不涉及云同步。如需备份：

```bash
# 备份整个 vault
cp -r ~/.solosoul ~/solosoul_backup_$(date +%Y%m%d)
```

### 8.5 安全性警告

如果看到安全性警告，可能的原因：

1. **检测到异常访问**: 检查是否有未知设备登录
2. **密码强度不足**: 建议使用更强壮的密码
3. **数据完整性异常**: 可能是磁盘故障

---

## 附录

### A. 快捷键

| 快捷键 | 功能 |
|--------|------|
| Cmd + L | 锁定 vault |
| Cmd + , | 打开设置 |

### B. 存储位置

```
~/.solosoul/
├── config.json           # 配置 (版本、账户列表)
├── vault.db             # SQLCipher 加密数据库
├── index.db            # 加密索引
└── accounts/           # 各账户数据
    └── {account_id}/
        └── profile.enc  # 加密的档案数据
```

### C. 联系我们

- **GitHub Issues**: https://github.com/solosoul/solosoul/issues
- **文档**: https://solosoul.example/docs

---

**SoloSoul - Be the only master of your digital self.**
