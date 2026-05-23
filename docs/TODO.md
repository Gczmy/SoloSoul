# SoloSoul 开发任务清单

> 全面重写：2026-04-27
> 项目状态：Flutter macOS Release 已发布，Unified Object Model 已完成，云同步待开发

---

## 项目架构

```
SoloSoul/
├── flutter/                    # 主项目：Flutter 跨平台客户端
│   ├── lib/
│   │   ├── core/
│   │   │   ├── services/       # 核心服务（18个）
│   │   │   │   ├── native_crypto_service.dart    # Rust FFI 加密
│   │   │   │   ├── rust_vault_service.dart       # Rust Vault
│   │   │   │   ├── profile_storage_service.dart  # Profile 存储
│   │   │   │   ├── unified_object_service.dart   # UnifiedObject CRUD
│   │   │   │   ├── secure_storage_service.dart   # 安全存储
│   │   │   │   ├── keychain_service.dart         # Keychain 封装
│   │   │   │   ├── biometric_service.dart        # 生物识别
│   │   │   │   ├── security_service.dart         # 安全服务
│   │   │   │   ├── operation_logger.dart         # 操作日志
│   │   │   │   └── clipboard_monitor_service.dart
│   │   │   ├── models/         # 基础模型
│   │   │   │   ├── unified_object_model.dart     # UnifiedObject + PropertyValue
│   │   │   │   └── base_models.dart              # 旧模型（兼容）
│   │   │   └── router/         # GoRouter 配置
│   │   └── presentation/
│   │       ├── pages/          # 13个页面
│   │       │   ├── login_page.dart
│   │       │   ├── home_page.dart
│   │       │   ├── object_workspace_page.dart    # 对象工作区（新）
│   │       │   ├── object_editor_page.dart       # 对象编辑器（新）
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
│   │       ├── providers/      # Riverpod providers
│   │       │   ├── unified_object_provider.dart  # 对象状态管理（新）
│   │       │   └── ...
│   │       └── widgets/        # 共享组件
│   │           ├── app_sidebar.dart              # 常驻侧边栏（新）
│   │           ├── scaffold_with_sidebar.dart    # 侧边栏布局（新）
│   │           ├── icon_picker_sheet.dart        # 图标选择器（新）
│   │           ├── lock_vault_dialog.dart        # 锁定确认对话框（新）
│   │           ├── object_tile.dart              # 对象列表项（新）
│   │           └── property_editor_factory.dart  # 属性编辑器（新）
│   └── native/                 # Rust 原生库 (FFI)
│       └── src/
│           ├── crypto/         # Argon2id + AES-256-GCM
│           ├── vault/          # 加密存储
│           ├── account/        # 账户管理
│           ├── sync/           # 同步引擎 (预留)
│           └── plugin/         # 插件沙盒 (预留)
├── cmd/                        # Go 后端服务
│   ├── solosould/              # HTTP API 服务器
│   └── solosoul/               # CLI 工具
└── docs/                       # 文档
```

---

## 已完成大功能

### ✅ Unified Object Model（2026-04-27）
- 核心模型：`UnifiedObject` + `UnifiedObjectData` + `PropertyValue` 体系
- 树形结构：`parentId` / `childrenIds` 作为层级唯一真相来源
- 内置类型：`page`, `collection`, `note`, `task`, `contact`（`ObjectTypeRegistry`）
-  schemaVersion 已升级到 v3
- 加密持久化：通过 `ProfileStorageService` → `RustVaultService` (AES-256-GCM)
- 侧边栏树形展示：自定义 page 支持折叠/展开多级子页面
- 对象工作区：page 类型显示卡片式 children，非 page 类型显示列表
- 对象编辑器：通用创建/编辑页面，支持图标选择、parent 选择（仅限 page）

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
1. [x] ID card 和 Address 的 history 有问题，修改后没有历史记录新增
2. [x] 密码验证失败后所有数据回到新创建状态（锁定后重新登录数据恢复） - 已修复 (ref.listen guard in profile_provider.dart:2052)

---

## P1: 账户配置系统 (Account Settings Sync)

### 核心问题：敏感度等级可配置与同步性矛盾

**现状**：
- 敏感度等级（public/private/restricted）硬编码在代码中
- 每账户单独维护的配置无法跨设备同步
- 配置数据缺乏与主数据同级的安全地位

**解决思路**：Metadata-in-Vault — 配置数据"随身化"

---

### 架构设计

#### 1. 存储架构：从"应用配置"转向"账户配置"

**现状问题**：
- SharedPreferences 或本地 JSON 文件存储配置 → 无法随加密数据库同步

**解决方案**：在 Rust Vault 层增加 `SETTING_{accountId}` key

```
vault/
├── PROFILE_{accountId}    # 已有：账户主数据
├── HIST_{accountId}      # 已有：字段历史
└── SETTING_{accountId}   # 新增：账户加密配置
```

**原理**：配置数据写入磁盘前，使用该账户的 Master Key 加密。另一设备同步数据库后，解密主数据的同时自动解密账户配置。

---

#### 2. 敏感度枚举扩展

```dart
enum SensitivityLevel {
  public,    // 明文显示，无验证
  internal,  // 明文显示，编辑需验证
  sensitive, // 遮罩显示，复制/查看需验证
  critical;  // 深度遮罩，解锁特定时长内可见
}
```

**与现有 `SensitivityLevel` 的映射**：
- `public` → 现有 `public`
- `private` → 现有 `private`
- `restricted` → 拆分为 `sensitive` + `critical`

---

#### 3. 配置映射表（Map-based Mapping）

在账户配置中存储：

```dart
// 全局默认值（内置代码）
final defaultSensitivity = <String, SensitivityLevel>{
  'password': SensitivityLevel.critical,
  'bank_account': SensitivityLevel.sensitive,
  'address': SensitivityLevel.internal,
  'name': SensitivityLevel.public,
};

// 账户覆盖层（用户修改后）
final accountOverrides = <String, SensitivityLevel>{
  'work_email': SensitivityLevel.sensitive, // 覆盖默认 public
};

// 最终计算：accountOverrides 优先，无覆盖则用 defaultSensitivity
```

**同步优势**：同步时只需同步几行"规则字符串"，而非成百上千个条目状态。

---

#### 4. 标签继承（Tag-based Inheritance）

允许用户为条目打标签（`#Finance`, `#Work`），配置一次，自动应用到所有带该标签的条目：

```dart
final tagRules = <String, SensitivityLevel>{
  '#Finance': SensitivityLevel.critical,
  '#Work': SensitivityLevel.sensitive,
  '#Personal': SensitivityLevel.internal,
};
```

---

### 实现步骤

#### Step 1: Rust 层 — Vault 存储接口

**文件**：`native/src/vault/store.rs`

**新增接口**：
```rust
// 保存账户配置（加密）
pub fn save_account_settings(account_id: &str, settings_json: &str) -> Result<(), VaultError>

// 加载账户配置（解密）
pub fn load_account_settings(account_id: &str) -> Result<String, VaultError>

// 列出所有账户配置key（用于同步扫描）
pub fn list_setting_keys() -> Result<Vec<String>, VaultError>
```

**存储路径**：
```
~/.solosoul/acc_{accountId}/
├── config.json       # 现有：盐和验证
├── vault.db          # 现有：SQLCipher 数据库
└── settings.json     # 新增：账户风格配置（可选，不存在时走默认值）
```

---

#### Step 2: Flutter 层 — RustVaultService 封装

**文件**：`lib/core/services/rust_vault_service.dart`

**新增方法**：
```dart
Future<void> saveAccountSettings(String accountId, Map<String, dynamic> settings)
Future<Map<String, dynamic>?> loadAccountSettings(String accountId)
Future<List<String>> listSettingKeys()
```

---

#### Step 3: Provider 层 — AccountStyleProvider

**文件**：`lib/presentation/providers/account_style_provider.dart`（新建）

**职责**：
- 账户解锁后立即加载加密配置
- 监听用户修改，延迟写入 Vault
- 提供 `getLevel(fieldId, tags)` 方法，查找顺序：accountOverrides → tagRules → defaultSensitivity

**状态**：
```dart
final accountStyleProvider = StateNotifierProvider<AccountStyleNotifier, AccountStyleState>

class AccountStyleState {
  final Map<String, SensitivityLevel> overrides;      // 字段级覆盖
  final Map<String, SensitivityLevel> tagRules;        // 标签规则
  final bool isLoaded;
}
```

---

#### Step 4: UI 层 — 动态响应

**文件**：`lib/presentation/widgets/universal_entry_card.dart`

**修改**：在渲染槽位时，根据配置决定显示方式

```dart
final level = ref.watch(accountStyleProvider).getLevel(fieldId, entryTags);

switch (level) {
  case SensitivityLevel.public:
    return Text(item.value); // 明文
  case SensitivityLevel.internal:
    return _buildInternalDisplay(item); // 明文+编辑验证
  case SensitivityLevel.sensitive:
    return BlurredText(text: item.value, onTap: () => _requestUnlock()); // 模糊+点击解锁
  case SensitivityLevel.critical:
    return LockedText(icon: Icons.lock, onLongPress: () => _requestUnlock()); // 深度锁定
}
```

---

#### Step 5: 设置界面

**文件**：`lib/presentation/pages/sensitivity_settings_page.dart`（扩建）

**功能**：
- 全局默认敏感度配置
- 账户覆盖层编辑器
- 标签规则编辑器
- 实时预览不同配置下的显示效果

---

#### Step 6: 一键切换虚假账户（Durez/Panic Mode）

**扩展目标**：基于账户配置，实现"假身份模式"

```
~/.solosoul/acc_{fakeId}/
├── config.json
├── vault.db
└── settings.json  # 包含 fake_profile_data
```

用户触发 Panic Mode 时：
1. 锁定当前真实账户
2. 用虚假账户数据快速替换内存状态
3. 展示预设的"干净"身份

---

### 同步流程

```
设备A:
  账户解锁 → 加载 SETTING_{accountId} → 合并 tagRules + overrides + defaults → 渲染 UI
  用户修改配置 → 延迟500ms写入 Vault → 标记 dirty

同步时:
  dirty SETTING_{accountId} → 加密上传云端
  或 dirty SETTING_{accountId} → 写入 NAS/Local Backup

设备B:
  账户解锁 → 下载 SETTING_{accountId} → 解密 → 合并配置 → 渲染 UI
```

---

### 依赖关系

```
Step 1 (Rust)  → Step 2 (Flutter封装)  → Step 3 (Provider)  → Step 4 (UI)  → Step 5 (Settings)
     ↓                  ↓                      ↓                   ↓
  P0-阻塞          可并行测试              核心逻辑              体验优化
```

---

## P1

### 本地搜索与 Vault 自动导入 🟡
> 详细设计文档：[LOCAL_SEARCH_IMPORT_DESIGN.md](LOCAL_SEARCH_IMPORT_DESIGN.md)

**目标**：让用户指定搜索路径，扫描本地文件中的个人信息，预览确认后自动创建/填充 Vault 条目。

**Phase 1：基础扫描能力（预计 3 天）**
- [x] 创建 `ScanResult` / `ScanSection` / `ScanField` 数据模型（Freezed）
- [x] 实现 `LocalSearchService`（macOS `mdfind` + 跨平台 `find` / `dir` 回退）
- [x] 扩展名 + 文件名关键词过滤
- [x] 内容指纹正则匹配（身份证、手机号、邮箱、护照号、银行卡号）
- [x] 基础 `ContentParserService`（txt / md / json / csv / pdf）
- [x] 单元测试

**Phase 2：预览与导入管线（预计 4 天）**
- [x] 实现 `ScanImportService`（映射 → 冲突检测 → 批量导入）
- [x] `LocalSearchConfigPage` + `LocalSearchProgressPage`
- [x] `ScanPreviewPage`（字段级确认、冲突高亮、敏感度标记）
- [x] `ScanImportResultPage`
- [x] 集成 `SensitiveValueWidget` + `PasswordVerificationDialog`
- [x] Widget 测试

**Phase 3：Office 格式与优化（预计 3 天）**
- [x] `.docx` / `.xlsx` 内容解析
- [x] 增量扫描缓存（mtime + size 记录到应用文档目录）
- [x] Windows Everything SDK 集成
- [x] 集成测试（端到端扫描 → 预览 → 导入 → Vault 验证）

---

### UI优化 🟡
- [ ] 优化travel history条目， 有两个重复的到达时间
- [ ] 现在所有条目在点击添加按钮后，都是使用footer，输入框都在最上方，很不方便，应该出现在对应条目的位置，比如条目在第三个，name输入框就出现在第三个的位置。
- [ ] 历史记录按钮加上。
- [ ] trash页面每个条目如果有历史记录的话，也直接在条目上加入history按钮。
- [ ] trash页面历史记录页，直接显示历史记录，现在还需要再点一下按钮，删除里面的这个按钮。


## PN: 安全

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

### Phase 1 — 插件市场基础设施 ✅
- [x] Rust SDK Host Functions 绑定 (`SDK/rust/src/lib.rs`)
- [x] `hello_world` 示例插件
- [x] `registry.json` 格式定义 + JSON Schema
- [x] 插件市场 CI/CD workflow

### Phase 2 — Rust Host 核心 ✅
- [x] Wasmtime 集成 (`wasmtime` + `wasmtime-wasi` preview1)
- [x] `PluginManifest` 扩展解析（version/api_version/compatibility/publisher/signature/network_policy）
- [x] `PluginStore` — `~/.solosoul/plugins/` 目录管理（0700 权限 + manifest/wasm 校验）
- [x] `SoloHostFunctions` — 4 个 Host Functions（`request_field` / `post_data` / `log` / `get_timestamp`）
- [x] `RateLimiter` — 10次/分钟/字段限流
- [x] `ConsentChannel` — oneshot 跨语言授权弹窗通道（60s 超时）
- [x] `AuditEntry` — 操作审计日志
- [x] `WasmSandbox` — Fuel 限制（Debug 100M / Release 10M）+ Trap 捕获
- [x] `PluginSessionManager` — 活跃 Session 跟踪 + TTL 自动清理
- [x] `FIELD_MAP` — 运行时字段敏感度映射
- [x] 8 个 FRB API 函数（install / execute / consent_response / revoke_session / force_unload / list_active_sessions / load_manifest / get_base_dir）

### Phase 3 — Flutter Service 层 ✅
- [x] `PluginRegistry` / `PluginRegistryEntry` / `PluginUpdateInfo` / `InstalledPluginInfo` 模型
- [x] `PluginRegistryService` — 远程 registry + 24h 缓存 + 离线回退
- [x] `PluginInstallerService` — 安装/更新/卸载，10MB wasm 限制，`installed.json` reconcile
- [x] `PluginService` — 加载已安装插件、运行插件（调用 FRB）

### Phase 4 — Flutter UI 层 ✅
- [x] `PluginConsentDialog` — 敏感度色标 + i18n
- [x] `PluginDashboardPage` — Tab 切换 + PluginCard + 搜索筛选 + 离线空状态
- [x] `settings_page_plugin_section` — 设置页插件管理入口
- [x] `AppRoutes.pluginDashboard` + GoRoute
- [x] `app_en.arb` / `app_zh.arb` — 30+ 插件相关本地化字符串

### 安全机制
- [x] 字段级敏感度分支（Public/Internal 直接返回，Sensitive/Critical 走 Consent）
- [x] 网络域名白名单校验（`NetworkPolicy.allowed_domains`）
- [x] Rate Limiting（Host 层 10次/分钟/字段）
- [ ] mlock 内存锁定（WASM 内存不由 Rust 直接分配，暂无法实施）
- [ ] Zeroize 敏感数据清理（WASM linear memory 生命周期由 wasmtime 管理，待研究）
- [ ] JIT 即时解密（预留，需 Vault 层配合）

### 已知限制 / 待办
- [ ] **FRB StreamSink 事件流**：`plugin_execute` 当前返回 `Future<int>`，Consent 通过独立 `frbPluginConsentResponse` 处理。事件流架构（`PluginEvent` → Dart）已预留但未接线。
- [ ] **`frb_plugin_execute` 当前为 stub**：实际执行逻辑待与 `WasmSandbox.execute` 完全集成（WASM 线程 + Store 隔离已就绪）。
- [ ] **iOS 构建**：`sandbox` feature 默认启用，iOS 需 `--no-default-features`（wasmtime asm 兼容性问题）。
- [ ] **SlotGo 官方插件**：未开始
- [ ] **Rust 未使用变量警告**：`manager.rs` 中 `plugin_id`、`session_ttl_seconds` 等（不影响编译）

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
- [ ] 用户自选数据目录 + Security-Scoped Bookmark (解决沙盒切换时的数据迁移问题，低优先级。当前 Debug/DMG 已统一禁用沙盒，若未来需上架 Mac App Store 再实施)

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
| Plugin System | 24 | 5 | 83% |
| LLM Features | 0 | 3 | 0% |
| Testing | 1 | 5 | 17% |
| **总计** | **58** | **39** | **60%** |

---

## 快速链接

- [USER_GUIDE](USER_GUIDE.md) - 用户指南
- [CLIENT_ROADMAP](CLIENT_ROADMAP.md) - 客户端路线图
- [CLAUDE.md](../CLAUDE.md) - Claude Code 开发指引
