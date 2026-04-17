# SoloSoul 用户指南

> 本指南详细说明 SoloSoul 的安装、配置和使用方法。
>
> 最后更新: 2026-04-08

---

## 目录

1. [简介](#简介)
2. [安装](#安装)
3. [快速开始](#快速开始)
4. [Web 界面](#web-界面)
5. [命令行工具](#命令行工具)
6. [档案管理](#档案管理)
7. [OCR 扫描](#ocr-扫描)
8. [Plugin 管理](#plugin-管理)
9. [安全说明](#安全说明)
10. [故障排除](#故障排除)

---

## 简介

SoloSoul (独灵) 是一个**本地数字孪生**引擎，用于安全存储和管理您的个人身份信息。

### 核心特性

- **🔒 零知识安全** - 所有数据在本地加密，仅您能访问
- **📄 OCR 自动填充** - 扫描护照或身份证自动提取信息
- **🔌 Plugin 生态** - 安全授权第三方工具访问您的数据
- **⚡ 高性能** - Go 语言编写，支持高速 API

### 工作原理

```
┌─────────────────────────────────────────────────────────────┐
│                     SoloSoul 工作流程                         │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌──────────┐    ┌──────────┐    ┌──────────────────┐  │
│  │  扫描    │───►│  OCR     │───►│  自动填充         │  │
│  │  文档    │    │  提取    │    │  档案            │  │
│  └──────────┘    └──────────┘    └──────────────────┘  │
│                                              │              │
│                                              ▼              │
│  ┌──────────┐    ┌──────────┐    ┌──────────────────┐  │
│  │  Plugin  │◄───│  Consent │◄───│  授权访问        │  │
│  │  请求    │    │  管理    │    │  自动化工具      │  │
│  └──────────┘    └──────────┘    └──────────────────┘  │
│                                                              │
│  ┌──────────────────────────────────────────────────┐   │
│  │              ~/.solosoul/ (本地加密存储)           │   │
│  └──────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

---

## 安装

### 系统要求

- **操作系统**: macOS 10.15+ / Linux (需要 systemd)
- **内存**: 建议 4GB+ (Argon2 加密需要)
- **磁盘**: 至少 100MB 可用空间

### 下载与编译

```bash
# 克隆仓库
git clone https://github.com/solosoul/solosoul.git
cd solosoul

# 编译后端
cd solosoul
go build -o solosould ./cmd/solosould

# 编译 CLI (可选)
go build -o solosoul ./cmd/solosoul

# 安装到 PATH
sudo mv solosould /usr/local/bin/
sudo mv solosoul /usr/local/bin/
```

### Web 界面安装

```bash
cd web
npm install
npm run dev
```

---

## 快速开始

### 1. 初始化 Vault

首次使用时，需要创建加密存储库：

```bash
# 使用 CLI
solosoul init

# 或使用 Web 界面访问 http://localhost:3000
```

**重要**: 设置一个强密码并妥善保管，**无法找回**！

### 2. 启动服务器

```bash
# 后端 API 服务器
solosould --addr :8080

# 前端 Web (另一个终端)
cd web && npm run dev
```

### 3. 访问 Web 界面

打开浏览器访问: **http://localhost:3000**

### 4. 创建档案

1. 登录后进入 **Dashboard**
2. 点击 **Edit Profile** 或 **Profile** 标签
3. 填写您的信息

---

## Web 界面

### 登录页面

访问 http://localhost:3000/login

- 输入您的 **Master Password**
- 点击 **Unlock** 解锁 vault
- 首次使用需先通过 **Setup** 创建密码

### 仪表盘 (Dashboard)

显示:
- **Profile Complete**: 档案完整度百分比
- **Profiles**: 档案数量
- **Documents**: 已上传文档数量
- **Active Sessions**: 当前活跃的 plugin 会话

**快速操作**:
- Edit Profile - 编辑档案
- Upload Document - 上传文档
- Manage Plugins - 管理插件
- Settings - 安全设置

### 档案编辑器 (Profile Editor)

5 个标签页:

| 标签 | 内容 |
|------|------|
| **Identity** | 姓名、出生日期、联系方式、地址 |
| **Travel** | 护照信息、签证 |
| **Financial** | 银行账户、卡片信息 |
| **Professional** | 教育背景、工作经历 |
| **Preferences** | 餐食偏好、座位偏好 |

**保存**: 编辑完成后点击 **Save Changes**

### 文档库 (Vault)

管理已扫描的文档:
- **筛选**: All / Passports / IDs / Visas / Photos
- **上传**: 点击按钮或拖拽上传
- **查看**: 点击文档卡片查看详情

### OCR 扫描 (Scan)

自动从证件提取信息:

1. 选择文档类型 (Passport / National ID / Visa / Driver License)
2. 上传或拖拽图片
3. 点击 **Scan Document**
4. 检查提取的字段
5. 确认保存

### Plugin 管理 (Plugins)

管理可访问您数据的第三方工具:

- **Pending**: 待批准的插件
- **Approved**: 已授权的插件
- **Revoke**: 撤销访问权限

### 设置 (Settings)

- **Change Master Password**: 修改主密码
- **Lock Vault**: 锁定 vault
- **Export Data**: 导出加密备份

---

## 命令行工具

### solosould - API 服务器

```bash
# 启动服务器
solosould --addr :8080

# 指定 vault 路径
SOLOSOUL_VAULT_PATH=/path/to/vault solosould --addr :8080

# Unix socket 模式 (生产环境推荐)
solosould --unix /tmp/solosoul.sock
```

### solosoul - CLI 工具

```bash
# 查看 vault 状态
solosoul status

# 初始化新 vault
solosoul init

# 解锁 vault
solosoul unlock

# 锁定 vault
solosoul lock

# 档案操作
solosoul profile list          # 列出所有档案
solosoul profile create myid  # 创建档案
solosoul profile get myid     # 获取档案详情
```

---

## 档案管理

### 档案结构

```json
{
  "profile_id": "my-passport",
  "version": "1.0",
  "identity": {
    "full_name": {
      "full_name": "张三",
      "given_name": "三",
      "family_name": "张"
    },
    "date_of_birth": {
      "year": 1990,
      "month": 1,
      "day": 15
    },
    "gender": "M",
    "contact": {
      "email": "zhang@example.com",
      "phone": "+86 138-0000-0000"
    },
    "primary_address": {
      "street": "朝阳区建国路88号",
      "city": "北京",
      "state": "北京",
      "postal_code": "100022",
      "country": "中国"
    }
  },
  "travel": {
    "primary_passport": {
      "number": "E12345678",
      "country": "CN",
      "nationality": "中国",
      "expiry_date": {
        "year": 2030,
        "month": 1,
        "day": 14
      }
    }
  }
}
```

### 字段路径

档案中的字段使用点分隔路径:

| 路径 | 说明 |
|------|------|
| `identity.full_name.full_name` | 完整姓名 |
| `identity.contact.email` | 电子邮箱 |
| `identity.primary_address.country` | 国家 |
| `travel.primary_passport.number` | 护照号 |
| `preferences.meal_preference` | 餐食偏好 |

---

## OCR 扫描

### 支持的文档类型

| 类型 | 格式 | 可提取字段 |
|------|------|-----------|
| **Passport** | MRZ TD3 | 姓名、护照号、国籍、出生日期、有效期 |
| **National ID** | MRZ TD1 | 姓名、身份证号、地址 |
| **Visa** | VIZ | 签证号、类型、有效期 |
| **Driver License** | VIZ | 驾照号、准驾类型 |

### MRZ 格式

护照使用机器可读区 (MRZ)，例如:

```
P<CNZHANG<<SAN<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<
E12345678<CNM19900115M2501015<<<<<<<<<<<<06
```

### OCR 工作流程

```
┌─────────────────────────────────────────────────────────────┐
│                     OCR 处理流程                              │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  1. [上传图片] ──► 2. [预处理] ──► 3. [MRZ 解析]         │
│                                    │                         │
│                                    ▼                         │
│  4. [字段提取] ◄── 3. [文本识别] ◄── 2. [PaddleOCR]      │
│                                    │                         │
│                                    ▼                         │
│  5. [用户确认] ──► 6. [保存档案] ──► 完成                  │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### 预处理选项

- **旋转**: 自动检测方向
- **灰度**: 转为黑白提高识别率
- **对比度**: 增强文字清晰度
- **降噪**: 去除图像噪点

---

## Plugin 管理

### 什么是 Plugin?

Plugin 是第三方应用，经您授权后可安全访问档案中的特定字段。

### 授权流程

```
┌─────────────────────────────────────────────────────────────┐
│                   Plugin 授权流程                            │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  1. [Plugin 请求访问] ──► 2. [显示请求字段]              │
│                                    │                         │
│                                    ▼                         │
│  3. [用户审批] ──► 4. [创建会话 (24h)]                     │
│                                    │                         │
│                                    ▼                         │
│  5. [Plugin 访问授权字段] ──► 6. [会话过期/撤销]          │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### 示例: SlotGo

SlotGo 是官方的自动化抢票插件:

```json
{
  "id": "com.solosoul.slotgo",
  "name": "SlotGo",
  "version": "1.0.0",
  "required_fields": [
    "identity.full_name.full_name",
    "identity.contact.email",
    "identity.contact.phone"
  ]
}
```

### 管理已授权的 Plugin

1. 进入 **Plugins** 页面
2. 查看 **Approved** 列表
3. 如需撤销，点击 Plugin 卡片中的 **Revoke Access**

---

## 安全说明

### 零知识架构

SoloSoul 严格遵守零知识原则:

| 安全措施 | 说明 |
|----------|------|
| **Master Password 不存储** | 仅内存使用，无法恢复 |
| **Argon2id 密钥派生** | 防暴力破解 (64MB 内存) |
| **AES-256-GCM 加密** | 军事级加密 |
| **阅后即焚** | 敏感数据使用后自动清零 |

### 数据存储位置

```
~/.solosoul/
├── config.json      # 配置 (盐值、版本)
├── index.db         # 加密索引
├── profiles/       # 档案数据 (加密)
│   └── {profile_id}/
│       └── *.enc   # 加密的字段数据
└── plugins/       # Plugin 配置
```

### 重要安全提示

1. **牢记密码** - 无法重置，忘记即丢失
2. **不要分享密码** - SoloSoul 团队永远不会询问
3. **定期备份** - 导出加密备份
4. **撤销不需要的 Plugin** - 定期检查已授权列表

---

## 故障排除

### 常见问题

#### 1. 忘记 Master Password

**无法恢复！** 如果忘记密码，vault 中的数据将永久无法访问。

**建议**: 在安全的密码管理器中备份密码。

#### 2. 服务器无法启动

```bash
# 检查端口占用
lsof -i :8080

# 使用其他端口
solosould --addr :8081
```

#### 3. Web 界面无法连接后端

```bash
# 确认后端运行
curl http://localhost:8080/health

# 检查 CORS 设置 (如果跨域)
```

#### 4. OCR 识别不准确

- 使用清晰、无反光的图片
- 确保文字完整可见
- 尝试不同的预处理选项

#### 5. Plugin 无法访问数据

- 确认已 **批准** Plugin
- 检查会话是否 **过期** (24小时后自动失效)
- 确认请求的字段在档案中存在

### 日志位置

```bash
# 服务器日志 (stdout)
solosould --addr :8080

# Vault 路径
echo $SOLOSOUL_VAULT_PATH  # 默认 ~/.solosoul/
```

### 获取帮助

- **GitHub Issues**: https://github.com/solosoul/solosoul/issues
- **文档**: https://solosoul.example/docs

---

## 附录

### 环境变量

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `SOLOSOUL_VAULT_PATH` | `~/.solosoul/` | Vault 存储路径 |
| `SOLOSOUL_API_URL` | `http://localhost:8080` | API 服务器地址 |

### API 端点

| 端点 | 方法 | 说明 |
|------|------|------|
| `/health` | GET | 健康检查 |
| `/api/auth/status` | GET | 获取 vault 状态 |
| `/api/auth/setup` | POST | 初始化 vault |
| `/api/auth/unlock` | POST | 解锁 vault |
| `/api/auth/lock` | POST | 锁定 vault |
| `/api/profile` | GET | 列出档案 |
| `/api/profile/{id}` | GET | 获取档案 |
| `/api/profile` | PUT | 更新档案 |
| `/api/plugins` | GET | 列出 plugins |
| `/api/plugins/{id}/consent/request` | POST | 请求 consent |

### 许可协议

MIT License

Copyright (c) 2026 SoloSoul

---

**SoloSoul - Be the only master of your digital self.**
