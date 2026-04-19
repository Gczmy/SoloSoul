# SoloSoul 开发任务清单

> 全面重写：2026-04-18
> 项目状态：Flutter macOS 发布就绪，Rust Core 完整，云同步待开发

---

## 项目架构

```
SoloSoul/
├── flutter/                    # 主项目：Flutter 跨平台客户端
│   ├── lib/
│   │   ├── core/services/    # 17个核心服务
│   │   │   ├── native_crypto_service.dart    # Rust FFI 加密
│   │   │   ├── rust_vault_service.dart      # Rust Vault
│   │   │   ├── profile_storage_service.dart  # Profile 存储
│   │   │   ├── secure_storage_service.dart   # 安全存储
│   │   │   ├── keychain_service.dart        # Keychain 封装
│   │   │   ├── biometric_service.dart       # 生物识别
│   │   │   ├── security_service.dart        # 安全服务
│   │   │   ├── operation_logger.dart        # 操作日志
│   │   │   └── clipboard_monitor_service.dart
│   │   └── presentation/
│   │       ├── pages/        # 11个页面
│   │       │   ├── login_page.dart
│   │       │   ├── home_page.dart
│   │       │   ├── profile_page.dart
│   │       │   ├── travel_page.dart
│   │       │   ├── financial_page.dart
│   │       │   ├── professional_page.dart
│   │       │   ├── settings_page.dart
│   │       │   ├── security_settings_page.dart
│   │       │   ├── sensitivity_settings_page.dart
│   │       │   ├── operation_log_page.dart
│   │       │   ├── trash_page.dart
│   │       │   └── splash_page.dart
│   │       ├── providers/    # Riverpod providers
│   │       └── widgets/      # 共享组件
│   └── native/               # Rust 原生库 (FFI)
│       └── src/
│           ├── crypto/       # Argon2id + AES-256-GCM
│           ├── vault/        # 加密存储
│           ├── account/      # 账户管理
│           ├── sync/        # 同步引擎 (预留)
│           └── plugin/      # 插件沙盒 (预留)
├── cmd/                      # Go 后端服务
│   ├── solosould/           # HTTP API 服务器
│   └── solosoul/            # CLI 工具
└── docs/                    # 文档
```

---

## P0: 关键问题 (Critical)

### iOS Keychain Method Handler 🔴 P0
- **问题**: `ios/Runner/AppDelegate.swift` 缺少 Keychain method handler
- **影响**: iOS 设备无法安全存储密钥
- **依赖**: `flutter_secure_storage` 在 iOS release build 需要 native method handler
- **临时方案**: 使用 `SimpleSecureStorage` 文件存储

### macOS 分发配置 🟡
- [x] DMG 构建脚本 (`./build_dmg.sh`) ✅
- [x] DMG 产物 (`SoloSoul-v1.0.dmg`, 12MB) ✅
- [ ] Apple 公证 (Notarization) - 分发给用户前必须完成

### Bug修复 🔴
1. [ ] ID card 和 Address 的 history 有问题，修改后没有历史记录新增
2. [ ] 密码验证失败后所有数据回到新创建状态（锁定后重新登录数据恢复） - 需修复

---

## P1: 代码重构 (DRY)

### 抽象条目模板 🔴
- [ ] 抽象条目模板函数：
  - 右侧操作按钮（edit/delete/copy）
  - 条目的 history 按钮
  - Private data 字段的眼睛按钮/复制按钮
  - Restricted data 的密码验证弹窗
  - Copied 提示条统一复用
- [ ] 各条目只专注于独特内容：图标、字段前缀等客制化内容

### UI优化 🟡
- [ ] Operation log 每条记录增加 detail 按钮，查看条目细节信息
- [ ] Trash 点击 detail 对话框中：restore 按钮放到 purge 左边，close 保持不变

---

## P1: Flutter macOS 稳定性

### 代码质量优化
- [ ] 3个 section 类 (Contact/IdCard/Address) 代码复用 ~90% → 提取 base class
- [ ] `SensitivityLevel` 字符串 → enum 改造
- [ ] 密码框边框代码在多处重复 (8处) → 方案1: 创建静态工厂方法 `AppInputDecoration.errorBorder()` | 方案2: 在 `MaterialApp` 的 `theme` 中统一定义 `inputDecorationTheme`
- [ ] `getFieldLevel` 异常控制流 → `firstWhereOrNull`
- [ ] `getDeletedItems()` 列表重建 → 添加缓存
- [ ] 教育/就业页面 CollapsibleSectionCard 集成

### 物理安全
- [ ] 防截屏 (FLAG_SECURE on Android, iOS snapshot blur)
- [ ] 多任务视图模糊 (AppLifecycleState blur overlay)

---

## P2: 跨平台构建

### macOS 🟢 基本完成
- [x] Release 构建 ✅
- [x] DMG 安装包 ✅
- [x] Keychain 集成 (macOS) ✅
- [ ] Touch ID / Face ID 集成

### iOS 🔴 待开发
- [ ] Rust 库编译 (arm64 + x86_64)
- [ ] Keychain method handler (P0 - 阻塞)
- [ ] Face ID / Touch ID
- [ ] iOS Simulator + 真机构建
- [ ] TestFlight / App Store 发布

### Android 🔴 待开发
- [ ] Android 项目初始化
- [ ] Rust 库编译 (arm64-v8a, armeabi-v7a, x86_64, x86)
- [ ] Android Keystore 集成
- [ ] BiometricPrompt 集成
- [ ] Play Store 发布

### Windows 🔴 待开发
- [ ] Windows 项目初始化
- [ ] Rust 库编译 (.dll)
- [ ] Windows Credential Manager
- [ ] Windows Hello 集成
- [ ] Microsoft Store 发布


---

## P3: 云同步开发

### 架构设计
- [ ] 云端存储格式设计
- [ ] 加密 blob 上传/下载
- [ ] 版本号机制 (冲突检测)
- [ ] WebSocket 实时同步通道
- [ ] 冲突解决 UI (三选项对话框)

### Flutter 端
- [ ] Online/Offline 标识逻辑修复
- [ ] 离线后台自动重连 (定时器 + 指数退避)
- [ ] 离线标识改为手动连接按钮

### Go 后端 (solosould)
- [ ] 云同步服务 API
- [ ] 设备注册与身份验证
- [ ] WebSocket 长连接
- [ ] 冲突解决 (最后写入优先)

### 法律文本
- [ ] 隐私政策更新 (数据上传云端)
- [ ] 服务条款 (云同步功能)
- [ ] 用户协议

---

## P4: 插件系统

### Wasm 沙盒架构
- [ ] Wasmtime/Wasmer 集成
- [ ] Host Functions 接口定义
- [ ] 插件权限 manifest.json 解析
- [ ] 用户交互授权弹窗
- [ ] 插件握手协议 (SHA-256 白名单)

### 安全机制
- [ ] mlock 内存锁定
- [ ] Zeroize 敏感数据清理
- [ ] JIT 即时解密
- [ ] 网络白名单策略
- [ ] Rate Limiting + Circuit Breaker

### 官方插件
- [ ] SlotGo (UK Visa 预约插件)

---

## P5: LLM 辅助功能

- [ ] LLM API 集成 (OpenAI/Claude)
- [ ] 脱敏后申请理由生成
- [ ] 非敏感逻辑处理 (润色、翻译)

---

## P6: 测试

### Flutter
- [ ] 组件测试
- [ ] 集成测试
- [ ] E2E 测试 (Playwright)

### Go 后端
- [x] 单元测试 (crypto, vault, schema, ocr, api) ✅

### 安全测试
- [ ] 渗透测试
- [ ] 模糊测试

---

## P7: 技术演进

- [ ] Riverpod 3.x 升级 (当前 2.6.1)
- [ ] 多语言支持 (i18n)
- [ ] 法律文本外部化 (从代码移到资源文件)

---

## 项目进度

| 模块 | 已完成 | 待完成 | 完成度 |
|------|--------|--------|--------|
| Flutter Core Crypto | 5 | 0 | 100% |
| Flutter UI Pages | 11 | 0 | 100% |
| Flutter Security | 4 | 3 | 57% |
| Rust Core | 5 | 0 | 100% |
| Go Backend | 6 | 2 | 75% |
| Cloud Sync | 0 | 8 | 0% |
| Cross-platform Build | 2 | 13 | 13% |
| Plugin System | 0 | 9 | 0% |
| LLM Features | 0 | 3 | 0% |
| Testing | 1 | 5 | 17% |
| **总计** | **34** | **43** | **44%** |

---

## 快速链接

- [USER_GUIDE](USER_GUIDE.md) - 用户指南
- [CLIENT_ROADMAP](CLIENT_ROADMAP.md) - 客户端路线图
- [CLAUDE.md](../CLAUDE.md) - Claude Code 开发指引
