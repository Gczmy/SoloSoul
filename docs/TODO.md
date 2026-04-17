# SoloSoul 开发任务清单

> 统一 TODO：以 Flutter 为主项目，Web 为遗留项目（考虑后续移除）
> 创建: 2026-04-14
> 更新: 2026-04-17

---

## 项目架构

```
SoloSoul/
├── flutter/          # 主项目：跨平台客户端 (macOS/iOS/Android/Windows)
│   └── lib/
│       ├── core/          # 核心加密、Rust FFI
│       ├── presentation/  # UI 页面和组件
│       └── ...
├── web/             # 遗留项目：Next.js Web UI (考虑移除)
└── cmd/             # Go 后端服务
    ├── solosould/   # HTTP API 服务器
    └── solosoul/    # CLI 工具
```

---

## P0: 关键问题 (Critical Issues)

### Flutter: Keychain 恢复
- **macOS**: ✅ 已完成 - `macos/Runner/AppDelegate.swift` 实现了 Keychain method handler
- **iOS**: ❌ 未完成 - `ios/Runner/AppDelegate.swift` 缺少 Keychain method handler
- **临时方案**: 使用 `SimpleSecureStorage` 文件存储 (ProfileStorageService)

**iOS Release 前必须完成**:
1. 在 `ios/Runner/AppDelegate.swift` 实现 Keychain method handler
2. 配置 iOS Entitlements 添加 Keychain 权限
3. 验证 `flutter_secure_storage` 在 iOS release build 下正常工作
4. 删除临时方案，切换到原生 Keychain

### Flutter: open -a 黑屏问题
- **状态**: 仅开发环境存在，DMG 正式发布不受影响
- **原因**: Launch Services 数据库中多版本路径混淆
- **临时 workaround**: 将 app 拖入 Applications 文件夹后再打开

---

## P1: 安全 (Security)

### Keychain 恢复 (Flutter)
| 平台 | 存储方案 | 状态 |
|------|---------|------|
| macOS | Keychain | ✅ 已完成 |
| iOS | Keychain | ❌ 需实现 method handler |
| Android | Keystore | 🔄 待开发 |
| 通用 | 文件加密 | ✅ 账户数据已加密 (SimpleSecureStorage) |

### 物理安全 (Flutter)
- [ ] 防截屏 (FLAG_SECURE on Android, iOS snapshot blur)
- [ ] 多任务视图模糊 (AppLifecycleState blur overlay)

---

## P2: 核心功能完成 (Flutter)

### 加密与存储 ✅
- ✅ Rust Argon2id FFI (64MB, 3 iterations)
- ✅ Dart FFI 绑定 (`native_crypto_service.dart`)
- ✅ AES-256-GCM 加密/解密
- ✅ Profile 数据结构 (`ProfileStorageService`)

### 账户与登录 ✅
- ✅ 账户创建/解锁 (绕过 Keychain 秒进)
- ✅ 密码提示词
- ✅ 账户列表折叠/展开
- ✅ 账户删除功能
- ✅ 主页面显示当前账号名

### Profile 页面 ✅
- ✅ ProfilePage (Contact Info、Identity Documents、Addresses)
- ✅ TravelPage (Passports、Visas、Travel History)
- ✅ FinancialPage (Bank Accounts、Cards、Tax IDs)
- ✅ ProfessionalPage (Education、Employment、Skills)
- ✅ SettingsPage (Account、Security、Sync、App Info)

### 数据敏感级别 ✅
- ✅ SensitivityLevel enum (public, private, restricted)
- ✅ SensitiveValueWidget (分级掩码组件)
- ✅ Privacy Shield 开关
- ✅ Operation Log 与 Sensitivity Settings 共享门禁

### 操作记录 ✅
- ✅ OperationLogPage
- ✅ Travel/Financial/Professional 页面操作日志记录
- ✅ 撤销 (Undo) 功能
- ✅ 回收站 (Trash) 功能
- ✅ 30天自动清理 + 手动清空

### UI/UX ✅
- ✅ 提示条系统 (Toast/Snackbar)
- ✅ CollapsibleSectionCard 组件
- ✅ SectionCard 共用组件
- ✅ 表单验证和错误处理

---

## P3: 云同步开发 (Flutter)

### 1. Online/Offline 标识修复
- **问题**: 目前 online/offline 标识逻辑不清晰
- **期望**: online = 云同步开启 AND 成功连接服务器；offline = 否则
- **涉及文件**: Settings 页面、Sync 相关状态管理

### 2. Offline 后台自动重连
- **问题**: 离线时不会自动尝试连接云服务器
- **期望**: 当云同步功能开启时，offline 状态下后台自动尝试连接云服务器
- **实现**: 定时器 + 指数退避重连策略

### 3. Offline 标识改为手动连接按钮
- **问题**: 当前 offline 标识只是状态显示
- **期望**: 将 offline 标识改为按钮，用户可手动点击立即尝试连接云服务器
- **UI**: 按钮显示 "Connect" 或重连图标

### 4. 云服务器开发
- **功能**:
  - 接收客户端加密信息
  - 收到其他设备同步请求后，将数据同步到各个设备
  - 设备注册与身份验证
  - 冲突解决（最后写入优先）
- **架构**:
  - Go 后端服务（与现有 solosould 分离或集成）
  - WebSocket 或 gRPC 长连接
  - 端到端加密（服务端不解密用户数据）
- **数据流**:
  1. 设备A 加密数据 → 云服务器
  2. 设备B 请求同步 → 云服务器推送加密数据 → 设备B
  3. 各设备本地解密

### 5. 条款更新
- **涉及**:
  - 隐私政策（数据上传云端说明）
  - 服务条款（云同步功能条款）
  - 用户协议
- **内容**:
  - 云同步的数据处理方式
  - 端到端加密说明（服务端不解密）
  - 数据存储期限
  - 多设备同步机制

---

## P4: 跨平台构建 (Flutter)

### macOS
- [x] 基础版本完成
- [ ] Touch ID / Face ID 集成
- [x] Keychain 密钥存储
- [ ] 菜单栏应用 (Menu Bar App)
- [ ] Universal Binary 编译 (arm64 + x86_64)

### iOS
- [x] 创建 iOS 项目
- [ ] Rust 库编译 (arm64 + x86_64)
- [ ] Keychain method handler (P0 - 未完成)
- [ ] Face ID / Touch ID
- [ ] 构建与测试
- [ ] TestFlight / App Store 发布

### Android
- [ ] 创建 Android 项目
- [ ] Rust 库编译 (arm64-v8a, armeabi-v7a, x86_64, x86)
- [ ] Keystore 存储
- [ ] BiometricPrompt 集成
- [ ] 构建与测试
- [ ] Play Store 发布

### Windows
- [ ] 创建 Windows 项目
- [ ] Rust 库编译
- [ ] Windows Credential Manager
- [ ] Windows Hello 集成
- [ ] 构建与测试
- [ ] Microsoft Store 发布

### 分发
- [ ] macOS: DMG 安装包 + Homebrew
- [ ] Android: APKs + Google Play
- [ ] iOS: TestFlight + App Store
- [ ] Windows: MSI / MSIX

---

## P5: Web 遗留项目 (考虑移除)

> Web 项目为早期实现，后续考虑迁移到 Flutter 或移除

### 当前状态: 功能完成，维护模式
- [x] Next.js 15 + App Router
- [x] 登录与设置
- [x] 仪表盘
- [x] 档案编辑器 (5个标签页)
- [x] Plugin 管理
- [x] OCR 扫描页

### 待迁移/移除
- [ ] LLM 辅助功能 (计划迁移到 Flutter)
- [ ] 多设备同步 (计划迁移到 Flutter)
- [ ] Web UI 特定功能 (考虑移除)

---

## P6: Go 后端 (solosould)

### 当前状态: 核心功能完成
- [x] Vault 服务 (Unlock/Lock/ChangePassword)
- [x] Profile 服务 (Get/Update/Validate/List/Delete)
- [x] Field 服务 (GetFields/SetFields)
- [x] Plugin 管理系统
- [x] OCR API 端点
- [x] Session Token 管理

### 待完成
- [ ] Unix Domain Socket 通信
- [ ] 云同步服务 (与 Flutter 客户端集成)
- [ ] Plugin: SlotGo (UK Visa Plugin)

---

## 待办事项 (TODO)

### Flutter UI 优化

1. **Education/Employment 页面** - CollapsibleSectionCard 集成

2. **设置页 Version 信息动态化**

3. **Riverpod 3.x 升级**
   - 当前版本: `flutter_riverpod: ^2.6.1`
   - 目标版本: 3.x (最新稳定版)
   - 涉及文件: 所有 providers (profile_provider.dart, auth_provider.dart, sensitivity_provider.dart 等)
   - 主要变更: 代码生成方式、注解语法、Provider 继承方式
   - 参考: https://riverpod.dev/docs/migration/from_v2_to_v3

4. **代码质量优化** (简化)
   - 调用版本检测 API 或配置文件
   - 显示：当前版本、最新版本、是否有更新

3. **代码质量优化** (简化)
   - [x] `_showPasswordDialog` 重复 → 提取共享组件
   - [ ] 3个 section 类 (Contact/IdCard/Address) ~90%相同代码 → 提取 base class
   - [ ] `getDeletedItems()` 每次重建列表 → 添加缓存
   - [ ] `SensitivityLevel` 字符串状态 → 改用 enum
   - [ ] `getFieldLevel` 用异常做控制流 → 用 `firstWhereOrNull`

4. **法律文本外部化**
   - 隐私政策、服务条款从代码移到资源文件

### LLM 辅助 (未来)
- [ ] LLM API 集成 (OpenAI/Claude)
- [ ] 脱敏后的申请理由生成
- [ ] 非敏感逻辑处理 (润色、翻译)

### 测试
- [x] Go 单元测试 (crypto, vault, schema, ocr, api)
- [ ] Flutter 组件测试
- [ ] E2E 测试 (Playwright)
- [ ] 安全测试

---

## 已知问题 (Known Issues)

| Issue | Severity | Status |
|-------|----------|--------|
| iOS Keychain method handler | P0 | 需在 AppDelegate.swift 实现 |
| macOS Keychain | ✅ | 已完成 |
| PaddleOCR stub | Medium | 需 Python 依赖 |
| Profile 切换页面数据消失 | High | 调查中 |

---

## 进度统计

| Category | Done | In Progress | To Do |
|----------|------|-------------|-------|
| Flutter Core Crypto | 5 | 0 | 0 |
| Flutter UI Pages | 8 | 0 | 2 |
| Flutter Security | 4 | 1 | 1 |
| Cloud Sync | 0 | 1 | 4 |
| Cross-platform Build | 1 | 1 | 11 |
| Go Backend | 6 | 1 | 2 |
| Web (Legacy) | 9 | 0 | 3 |
| **Total** | **33** | **4** | **23** |

**完成度**: ~55% (33/60 tasks)
