# SoloSoul 功能目录 — 按技术层级组织

> 本文档是 SoloSoul 全功能的技术层级化梳理，从底层基础设施到顶层用户界面逐层展开。
> 
> **适用场景**：
> - **重构开发**：按层迁移，确保下层稳定后再动上层
> - **从零开始**：按层实现，每层提供明确的接口契约
> - **架构审查**：清晰展示依赖方向（上层依赖下层，禁止逆向）
>
> **状态**：基于 2026-06-04 代码库快照  
> **依赖方向**：L0 → L1 → L2 → L3 → L4 → L5 → L6 → L7（上层可依赖同层或下层，禁止依赖上层）

---

## 目录

- [L0 基础设施与平台层](#l0-基础设施与平台层)
- [L1 数据持久化层](#l1-数据持久化层)
- [L2 密码学与安全核心层](#l2-密码学与安全核心层)
- [L3 核心业务服务层](#l3-核心业务服务层)
- [L4 状态管理层](#l4-状态管理层)
- [L5 UI 组件与设计系统层](#l5-ui-组件与设计系统层)
- [L6 页面与路由层](#l6-页面与路由层)
- [L7 应用入口与全局系统](#l7-应用入口与全局系统)
- [L8 测试与质量保障层](#l8-测试与质量保障层)
- [L9 其他端（Go / Web）](#l9-其他端go--web)
- [跨层依赖矩阵](#跨层依赖矩阵)

---

## L0 基础设施与平台层

> **定位**：与操作系统直接交互的能力，提供原生 API 封装、构建体系、国际化基础。不持有业务逻辑。

### L0.1 原生平台通道（Platform Channels）

| 功能 | 描述 | 涉及平台 | 技术要点 |
|------|------|---------|---------|
| **macOS 原生通道** | 菜单栏锁定回调、系统睡眠回调 | macOS | `MethodChannel` + Swift handler；`NativeChannelService` |
| **macOS QuickLook** | PPTX/PPT 文件原生预览 | macOS | `QLPreviewController` 封装；`QuickLookService` |
| **Apple Vision OCR** | 备用 OCR 方案（非 ONNX） | macOS/iOS | Apple Vision 框架 VNRecognizeTextRequest；`AppleVisionOcr` |
| **剪贴板监控** | 敏感数据复制后定时清理 | 全平台 | `ClipboardMonitorService`；监听复制事件，按安全设置延迟清空 |
| **文件选择器** | 导入/导出时的文件选择 | 全平台 | `file_picker` 插件；支持 macOS 沙箱路径 |
| **路径获取** | 应用文档目录、临时目录、支持目录 | 全平台 | `path_provider`；`getApplicationDocumentsDirectory()` 等 |
| **包信息** | 版本号、构建号获取 | 全平台 | `package_info_plus`；用于版本检测与备份命名 |

### L0.2 原生安全存储

| 功能 | 描述 | 涉及平台 | 技术要点 |
|------|------|---------|---------|
| **iOS Keychain** | 安全存储敏感小数据 | iOS/macOS | `flutter_secure_storage`；`first_unlock_this_device` 访问级别 |
| **Android EncryptedSharedPreferences** | 安全存储敏感小数据 | Android | `flutter_secure_storage`；`encryptedSharedPreferences: true` |
| **Fallback Secure Storage** | Keychain 失败后的文件回退 | 全平台 | `FallbackSecureStorage`；AES 加密后存文件 |
| **Rust Safe Storage** | Rust 端直接操作平台安全存储 | 全平台 | `native/src/safe_storage.rs`；macOS Keychain / Windows Credential / Linux Secret Service |

### L0.3 构建与分发体系

| 功能 | 描述 | 技术要点 |
|------|------|---------|
| **Rust 静态库编译** | `flutter/native/` 编译为 `staticlib/cdylib` | `cargo build --release`；macOS `.dylib` / Windows `.dll` |
| **交叉编译脚本** | `flutter/native/` 多平台构建 | `build_rust.sh --all`；macOS Universal + Linux + Windows |
| **macOS Release 构建** | Flutter macOS Release 带混淆 | `flutter build macos --release --obfuscate --split-debug-info` |
| **DMG 打包** | macOS 安装包生成 | `build_dmg.sh`；输出 `SoloSoul-v1.0.dmg` |
| **FFI 验证** | Rust 签名 ↔ Dart 声明一致性检查 | `validate_ffi.sh` |
| **调试符号管理** | Release 调试信息分离存储 | `debug_info/macos/`；已加入 `.gitignore` |

### L0.4 CI/CD 流水线

| 工作流 | 触发条件 | 执行内容 |
|--------|---------|---------|
| **`ci_cd.yml`** | push master/main 或 PR | Rust 测试 → Dart 单元测试 → Widget 测试 → macOS 集成测试 → Release 构建 → DMG 打包 → Draft Pre-release |
| **`pr_check.yml`** | PR | `cargo fmt --check` + `cargo clippy` + `dart analyze --fatal-infos` + 测试 |

### L0.5 国际化基础

| 功能 | 描述 | 技术要点 |
|------|------|---------|
| **ARB 翻译系统** | 英文/中文双语支持 | `lib/l10n/`；`app_en.arb` / `app_zh.arb` |
| **动态字段标签** | 内置类型字段标签不走静态存储 | `FieldLabelResolver` + `translateFieldLabel`；运行时从 ARB 读取 |
| **Locale 切换** | 实时切换无需重启 | `languageProvider`；`Locale('en')` / `Locale('zh')` |
| **本地化委托** | Material/Cupertino 组件本地化 | `GlobalMaterialLocalizations` 等 4 个 delegate |

---

## L1 数据持久化层

> **定位**：所有数据的持久化存储，包括加密数据库、文件系统、缓存。对上层提供 CRUD 接口，隐藏加密细节。

### L1.1 Vault 存储（Rust 核心）

**核心实现**：`native/src/vault/`

| 模块 | 功能 | 技术细节 |
|------|------|---------|
| **`store.rs`** | SQLCipher 加密 SQLite 的底层封装 | `rusqlite` + SQLCipher；连接时 `PRAGMA key = ...`；事务支持 |
| **`profile.rs`** | Profile 数据在 Vault 中的序列化/反序列化 | JSON 字符串存入 SQLite BLOB/TEXT 字段 |
| **`processor.rs`** | 数据处理流水线（加密前/解密后） | 数据校验、压缩、分块 |
| **`migration.rs`** | Vault 级别的数据迁移 | 版本号检测，自动执行迁移脚本 |

**Vault 配置**：
- 路径：`{appSupportDir}/solosoul/{account_id}/`
- 目录权限：`0700`
- 状态：`Locked` / `Unlocked` / `Corrupted`

### L1.2 Dart 端 Vault 服务封装

**核心实现**：`lib/core/services/rust_vault_service.dart`

| 功能 | 描述 | 接口契约 |
|------|------|---------|
| **加密 Bytes** | 任意字节数组 → SOLO blob v2 | `encryptBytes(Uint8List data) → Uint8List?`；需 Vault 已解锁 |
| **解密 Bytes** | SOLO blob v2 → 明文 | `decryptBytes(Uint8List combined) → Uint8List?` |
| **流式加密文件** | 大文件分块加密 → SOLO blob v3 | `encryptFile(src, dst, progressPath, cancelPath) → bool`；1MB 分块 |
| **流式解密文件** | SOLO blob v3 → 明文文件 | `decryptFile(src, dst, progressPath, cancelPath) → bool` |
| **Vault 统计** | Profile 数量、总大小、最后修改 | `getVaultStats() → VaultStats` |

### L1.3 Profile 存储服务

**核心实现**：`lib/core/services/profile_storage_service.dart`

| 功能 | 描述 | 技术细节 |
|------|------|---------|
| **保存 Profile** | 将 `ProfileData` 序列化为 JSON → 加密 → 存入 Vault | `saveProfile(accountId, profileData)` |
| **加载 Profile** | 从 Vault 取出 → 解密 → 反序列化 | `loadProfile(accountId) → ProfileData?` |
| **删除 Profile** | 从 Vault 移除指定 Profile | `deleteProfile(accountId)` |
| **LRU 缓存** | 内存中缓存最近 3 个账户的 Profile | `_maxCacheSize = 3`；避免频繁解密 |
| **Schema 迁移** | 自动将旧版本数据升级到新版本 | 当前 v6；v4→v5（typeId 去页面化）；v5→v6（清除内置 propertyLabels）|

### L1.4 附件存储系统

**核心实现**：`lib/core/services/attachment_storage_service.dart`

| 功能 | 描述 | 技术细节 |
|------|------|---------|
| **小文件保存**（≤50MB） | 内存一次性加密 → 写入磁盘 | `saveAttachment()` → SOLO blob v2；进度回调 0.1/0.7/0.9/1.0 |
| **小文件加载** | 读取磁盘 → 内存一次性解密 | `loadAttachment()` → `Uint8List` |
| **大文件保存** | Rust 流式加密 → 磁盘 | `saveAttachmentFromPath()` → SOLO blob v3；支持取消 |
| **大文件解密** | Rust 流式解密到目标路径 | `decryptAttachmentToPath()`；支持取消与进度 |
| **附件预览控制** | >10MB 的 v3 文件拒绝内存加载 | `maxPreviewSize = 10MB` |
| **附件元数据** | `Attachment` 对象存于 `UnifiedObject.attachments` | id, fileId, fileName, mimeType, size, encryptedSize, createdAt |
| **附件目录** | `{appDocDir}/solosoul_storage/attachments/{accountId}/` | 自动创建 |

### L1.5 备份数据层

**核心实现**：`lib/core/services/backup_service.dart`

| 功能 | 描述 | 技术细节 |
|------|------|---------|
| **备份目录结构** | `{appSupportDir}/solosoul_backups/{accountId}/` | 常规备份 + `special/` 子目录 |
| **备份文件命名** | `backup_YYYY-MM-DD_HH-mm-ss[_vX.Y.Z].backup` | 含版本号，便于追溯 |
| **附件备份** | 备份文件同名的 `.attachments/` 目录 | 附件加密副本 |
| **保留策略** | 常规最多 5 份，特别最多 5 份 | 超出时删除最旧 |
| **权限设置** | `chmod 600` | 仅限所有者读写 |

### L1.6 操作日志持久化

**核心实现**：`lib/core/services/operation_logger.dart`

| 功能 | 描述 | 技术细节 |
|------|------|---------|
| **日志文件** | 本地 JSON Lines 文件 | 每条操作一个 JSON 对象 |
| **日志刷新** | 从磁盘重新加载 | `refreshFromDisk()`；用于操作日志页面 |
| **30 天清理** | 软删除对象 30 天后永久清理时同步清理相关日志 | 与 Trash 联动 |

### L1.7 扫描缓存

**核心实现**：`lib/core/services/scan/scan_cache_service.dart`

| 功能 | 描述 | 技术细节 |
|------|------|---------|
| **缓存文件** | 记录已扫描文件的路径、修改时间、大小 | JSON 格式 |
| **跳过逻辑** | 文件未变更时跳过重新扫描 | 对比 mtime + size |
| **缓存修剪** | 删除已不存在的文件记录 | `prune(allPaths)` |

---

## L2 密码学与安全核心层

> **定位**：所有加密、密钥派生、安全内存管理。对上层提供"黑箱"接口：输入密码/数据，输出密文/明文，隐藏算法细节。

### L2.1 密钥派生（KDF）

**Rust 实现**：`native/src/crypto/argon2.rs`

| 功能 | 描述 | 技术细节 |
|------|------|---------|
| **Argon2id** | 密码 → 256-bit 密钥 | `argon2` crate；Algorithm::Argon2id；Version::V0x13 |
| **开发参数** | 8 MiB / 2 iterations / 4 parallelism | 默认；Apple Silicon 避免挂起 |
| **生产参数** | 64 MiB / 3 iterations / 4 parallelism | `SOLOSOUL_SECURE=1` 启用；OWASP 推荐 |
| **Salt 生成** | 32 字节密码学安全随机数 | `rand::RngCore::fill_bytes` / `OsRng` |
| **KDF 调用** | Flutter 通过 FRB 调用 Rust KDF | `native/src/crypto/argon2.rs`；`derive_key()` |

### L2.2 对称加密

**Rust 实现**：`native/src/crypto/aes.rs`

| 功能 | 描述 | 技术细节 |
|------|------|---------|
| **AES-256-GCM** |  authenticated encryption | `aes_gcm` crate；Aes256Gcm；12-byte nonce；16-byte tag |
| **Bytes 加密** | 明文 bytes → nonce + ciphertext + tag | `encryptBytes()` → SOLO blob v2 |
| **Bytes 解密** | SOLO blob v2 → 明文 bytes | `decryptBytes()` |
| **流式加密** | 大文件分块处理 | `crypto/stream.rs`；1MB 分块；每块独立 nonce；SOLO blob v3 |
| **Dart 回退** | Android 等平台的纯 Dart 加密 | `pointycastle` / `cryptography` / `encrypt`；与 Rust 实现格式兼容 |

### L2.3 安全内存管理

| 功能 | 描述 | 技术细节 |
|------|------|---------|
| **显式清零** | 敏感数据使用后立即覆盖内存 | Rust：`zeroize` crate；Go：`SecureWipe` |
| **自动清零** | 值离开作用域时自动清零 | Rust：`Zeroizing<T>` wrapper |
| **内存锁定** | 防止敏感数据被交换到磁盘 | Rust：`mlock`；Go：`runtime.SetFinalizer` |
| **链接时优化** | 消除冗余副本 | `lto = true` |

### L2.4 验证与令牌

| 功能 | 描述 | 技术细节 |
|------|------|---------|
| **验证令牌** | 不存储密码哈希，而是存储加密后的固定字符串 | `"SOLOSOUL_VAULT_V1"` 的 AES-GCM 密文 |
| **密码验证** | 解锁时解密验证令牌，成功则密码正确 | 避免存储密码的任何派生形式 |
| **常数时间比较** | 防止时序攻击 | `subtle.ConstantTimeCompare`（Go）/ `constant_time_eq`（Rust）|

### L2.5 生物识别凭证安全

**核心实现**：`lib/core/services/biometric_credential_service.dart`

| 功能 | 描述 | 技术细节 |
|------|------|---------|
| **凭证生成** | 生物识别验证通过后生成安全凭证 | 随机高熵字符串 |
| **凭证存储** | 存入平台安全存储（Keychain/EncryptedSharedPrefs） | 与 accountId 绑定 |
| **凭证验证** | 生物识别解锁时验证凭证有效性 | 不存储主密码 |
| **凭证清除** | 禁用生物识别时删除凭证 | `clearBiometricCredential(accountId)` |

---

## L3 核心业务服务层

> **定位**：持有所有业务逻辑，处理数据转换、校验、流程编排。对上层提供高内聚的服务接口，对下层调用 L1/L2 能力。

### L3.1 账户管理服务

**Rust 实现**：`native/src/account/manager.rs`  
**Dart 封装**：`lib/presentation/providers/auth/auth_services.dart`

| 功能 | 描述 | 接口契约 |
|------|------|---------|
| **创建账户** | 生成 Salt → KDF → 创建 Vault → 保存配置 | `createAccount(name, password) → CreateAccountResult` |
| **删除账户** | 删除 Vault 目录 + 账户索引 | `deleteAccount(accountId)` |
| **账户列表** | 返回所有本地账户信息 | `listAccounts() → List<AccountInfo>` |
| **切换账户** | 切换当前活跃账户，重新加载 Vault | `selectAccount(accountId)` |
| **默认账户** | 标记/获取默认账户 | `setDefaultAccount(accountId)` |

### L3.2 Vault 生命周期服务

**Dart 实现**：`lib/presentation/providers/auth/auth_notifier.dart`  
**Rust 实现**：`native/src/vault/store.rs`

| 功能 | 描述 | 接口契约 |
|------|------|---------|
| **初始化 Vault** | 首次使用创建加密 Vault | `initVault(password)` |
| **解锁 Vault** | 密码验证 → 派生密钥 → 打开 SQLCipher | `unlockVault(accountId, password) → UnlockVaultResult` |
| **锁定 Vault** | 关闭 SQLCipher 连接 → 擦除内存密钥 | `lockVault()` |
| **修改密码** | 旧密码验证 → 用新密码重新加密所有数据 | `changePassword(oldPassword, newPassword)` |
| **Vault 状态查询** | 当前状态（locked/unlocked/initialized） | `vaultStateProvider` |

### L3.3 Unified Object 服务

**核心实现**：`lib/core/services/unified_object_service.dart`

| 功能 | 描述 | 技术细节 |
|------|------|---------|
| **对象类型注册表** | 内置类型 + 自定义类型统一管理 | `ObjectTypeRegistry`；内置类型以 `__preset_` 为前缀 |
| **默认页面 ID** | Profile/Travel/Financial/Professional 固定标识 | `DefaultPageIds` |
| **默认分区 ID** | 各页面下的固定分区标识 | `DefaultSectionIds` |
| **构建属性** | 根据类型定义生成空属性 Map | `buildPropertiesFromType(typeId)` |
| **构建标签** | 自定义类型保留用户定义的标签 | `buildPropertyLabelsFromType(typeId)`；内置类型返回空 |
| **ID 生成** | UUID v4 | `Uuid().v4()` |
| **时间戳** | 毫秒级 Unix 时间戳 | `DateTime.now().millisecondsSinceEpoch` |

### L3.4 字段历史服务

**核心实现**：`lib/core/services/field_history_service.dart`

| 功能 | 描述 | 技术细节 |
|------|------|---------|
| **记录变更** | 字段值变化时保存历史 | 值 + 时间戳 + 来源（用户/插件/导入） |
| **查询历史** | 获取某对象某字段的所有历史 | `getFieldHistory(objectId, fieldKey)` |
| **恢复历史值** | 从历史中恢复到指定版本 | `restoreFieldHistory(...)` |

### L3.5 OCR 引擎服务

**Dart 封装**：`lib/core/services/ocr_service.dart`  
**Rust 实现**：`native/src/ocr/`

| 功能 | 描述 | 技术细节 |
|------|------|---------|
| **引擎初始化** | 从 asset 加载 ONNX 模型 → Rust FFI | `OcrService.initialize()`；det + cls + rec 三模型 |
| **MRZ 识别** | 定位并提取护照/身份证 MRZ 码 | `extractMrz(imageBytes)`；10s 超时；置信度检查 |
| **通用 OCR** | 检测 + 识别图像中的所有文本 | `recognizeImage(imageBytes)`；返回 `GeneralOcrResult` |
| **引擎状态查询** | 查询各模型加载状态 | `OcrEngineStatus`：detLoaded / clsLoaded / recLoaded |
| **模型卸载** | 释放 ONNX Session 内存 | `unloadModels()` |
| **错误分级** | 未初始化/超时/MRZ 未找到/低置信度/无文本 | `OcrException` 子类 |

### L3.6 同步引擎服务

**Dart 封装**：`lib/core/services/sync_service.dart`  
**Rust 实现**：`native/src/sync/`

| 功能 | 描述 | 技术细节 |
|------|------|---------|
| **mDNS 广播** | 在局域网广播本机存在 | `advertise(deviceName, port=9900)` |
| **mDNS 发现** | 发现其他 SoloSoul 设备 | `discoverDevices(timeoutMs=3000)` |
| **发起同步** | 作为主动方连接远程设备 | `syncAsInitiator(accountId, remoteAddr, pairingKey, deviceSalt)` |
| **响应同步** | 作为被动方监听 9900 端口 | `syncAsResponder(accountId, pairingKey, deviceSalt)` |
| **CRDT 合并** | 自动合并双方数据差异 | `native/src/sync/crdt.rs`；无冲突复制数据类型 |
| **附件同步** | 同步完成后传输附件文件 | 增量传输；不完整标记 |
| **Noise_IK 加密** | 所有通信端到端加密 | `native/src/sync/protocol.rs` |
| **30 秒超时** | 单步同步操作超时保护 | `.timeout(const Duration(seconds: 30))` |

### L3.7 插件系统服务

**Dart 封装**：`lib/core/services/plugin_service.dart`  
**Rust 实现**：`native/src/plugin/`

| 功能 | 描述 | 技术细节 |
|------|------|---------|
| **加载已安装插件** | 从插件目录读取所有 manifest | `loadInstalledPlugins()`；跳过损坏插件 |
| **运行插件** | 启动 Wasmtime 沙盒执行 | `runPlugin(pluginId, params)` → `Stream<PluginEvent>` |
| **列出活跃 Session** | 查询当前运行的插件会话 | `listActiveSessions()` |
| **强制卸载** | 终止插件并清理资源 | `forceUnload(pluginId)` |
| **iOS 限制** | 不支持 Wasmtime JIT | 抛出 `UnsupportedError` |

**插件 Dart 端补充服务**：
- `PluginRegistryService` — 插件市场注册表管理
- `PluginInstallerService` — 安装/卸载/更新
- `PluginDataStructureService` — 插件数据结构转换

### L3.8 LLM 服务

**核心实现**：`lib/core/services/llm/`

| 功能 | 描述 | 技术细节 |
|------|------|---------|
| **云端 LLM** | OpenAI / Anthropic / Google 等 | `LlmCloudService`；API Key 加密存储；HTTP 流式响应 |
| **本地 LLM** | Ollama 本地模型 | `LlmLocalService`；自动检测 Ollama 状态；本地 HTTP 调用 |
| **统一接口** | 云/本地统一抽象 | `LlmServiceInterface`；`sendMessage()` / `streamMessage()` |
| **配置管理** | 模型选择、温度、maxTokens、系统提示词 | `LlmConfigService`；`LlmConfigModels` |
| **会话管理** | 创建/删除/重命名会话；消息历史 | `ChatSessionService`；`ChatHistoryService` |
| **上下文注入** | 将用户 Profile 数据作为上下文注入对话 | `LlmContextService` |
| **用量统计** | Token 数、费用、按模型/时间聚合 | `LlmUsageStats`；Sparkline + 饼图数据 |
| **字段映射解析** | 将 LLM JSON 输出解析为 SoloSoul 字段 | `LlmFieldMappingParser` |
| **提示词模板** | 预定义提示词（提取、映射、问答等） | `LlmPromptTemplates` |
| **模型状态管理** | 模型下载进度、可用状态 | `LlmModelManager`；`LlmModelState` |

### L3.9 本地文件扫描服务

**核心实现**：`lib/core/services/scan/`

| 功能 | 描述 | 技术细节 |
|------|------|---------|
| **文件列举** | 递归扫描目标路径 | `ScanFileLister`；`kMaxFilesPerPath` 限制 |
| **内容解析** | 提取 PDF/Word/Excel 文本 | `ContentParserService` |
| **图像扫描** | OCR 识别图片中的文字 | `ScanImageScanner` |
| **分区检测** | 根据文件名+内容推断 SoloSoul 分区 | `ScanSectionDetector` |
| **AI 映射** | 调用 LLM 将文件内容映射到字段 | `local_search_provider.dart`；`performAiMapping()` |
| **导入服务** | 将扫描结果写入 Vault | `ScanImportService`；冲突检测 |
| **取消机制** | 全程可取消 | `CancelToken`；检查 `isCanceled` |
| **后台服务** | 扫描在独立 Isolate 中进行 | `ScanBackgroundService` |

### L3.10 备份/导出/导入服务

| 功能 | 描述 | 技术细节 |
|------|------|---------|
| **创建备份** | 加密导出 Profile + 附件 | `BackupService.createBackup()`；进度回调 |
| **创建特别备份** | 用户命名备份 | `createSpecialBackup(name)` |
| **列出备份** | 常规 + 特别备份列表 | 含时间、大小、版本号 |
| **恢复备份** | 从备份文件恢复 | `restoreBackup(entry)`；需 Vault 已解锁 |
| **删除备份** | 手动删除备份文件 | `deleteBackup(entry)` |
| **导出 Profile** | 导出为加密 JSON 文件 | `ExportImportService.exportProfile()` |
| **导入 Profile** | 从加密文件导入 | `importProfile()`；预览 + 冲突检测 |

### L3.11 操作日志服务

**核心实现**：`lib/core/services/operation_logger.dart` / `audit_log_service.dart`

| 功能 | 描述 | 技术细节 |
|------|------|---------|
| **记录操作** | CRUD 操作生成日志条目 | `OperationEntry`：action + objectName + before/after diff |
| **通知发布** | 操作完成后发布通知 | `OperationNotification`；SnackBar 提示 |
| **日志过滤** | 按操作类型过滤 | 创建/更新/删除/恢复/备份/导入/导出 |
| **日志搜索** | 关键词搜索 | 对象名、字段名匹配 |

### L3.12 搜索服务

**核心实现**：`lib/presentation/providers/search_provider.dart`

| 功能 | 描述 | 技术细节 |
|------|------|---------|
| **全文索引** | 构建 UnifiedObject 的搜索索引 | 名称 + 属性值 + 标签 |
| **实时搜索** | 输入即搜索，防抖处理 | 通常 300ms 防抖 |
| **类型过滤** | 按四大分类过滤 | Profile/Travel/Financial/Professional/All |
| **结果排序** | 相关性排序 | 名称匹配优先，属性值匹配次之 |

### L3.13 其他辅助服务

| 服务 | 功能 | 位置 |
|------|------|------|
| **AppVersionTracker** | 检测版本变化，标记待备份 | `lib/core/services/app_version_tracker.dart` |
| **UserPreferencesService** | 用户偏好设置持久化 | `lib/core/services/user_preferences_service.dart` |
| **LanguageService** | 语言检测与切换 | `lib/core/services/language_service.dart` |
| **MachineKeyGenerator** | 设备唯一标识生成 | `lib/core/services/machine_key_generator.dart` |
| **DocumentFieldExtractor** | 文档字段提取（PDF 等） | `lib/core/services/document_field_extractor.dart` |
| **PdfRenderService** | PDF 渲染为图片预览 | `lib/core/services/pdf_render_service.dart` |
| **PptxThumbnailExtractor** | PPTX 缩略图提取 | `lib/core/services/pptx_thumbnail_extractor.dart` |
| **UserGuideService** | 功能指南索引加载 | `lib/core/services/user_guide_service.dart` |
| **DebugLogger** | 文件日志记录 | `lib/core/services/debug_logger.dart`；Release 诊断 |
| **PageSectionLinkRegistry** | 页面-分区关联注册 | `lib/core/services/page_section_link_registry.dart` |

---

## L4 状态管理层

> **定位**：基于 Riverpod 的全局状态容器，连接 L3 服务与 L5 UI 组件。每个 Provider 管理一块独立的状态域，提供不可变状态 + 变更方法。

### L4.1 认证状态（Auth）

**核心实现**：`lib/presentation/providers/auth/`

| Provider | 状态范围 | 关键行为 |
|----------|---------|---------|
| **`authNotifierProvider`** | 登录/解锁/锁定全流程 | `unlockVault()` / `lockVault()` / `createAccount()` / `selectAccount()`；解锁后自动加载 Profile |
| **`authStateProvider`** | 当前认证状态 | `locked` / `unlocked` / `loading` / `error` |
| **`sensitivePageAccessProvider`** | 敏感页面访问权限 | 密码验证后授予 1 分钟有效期 |
| **`isSensitiveAccessGrantedProvider`** | 当前是否在敏感访问有效期内 | 布尔值，自动过期 |

### L4.2 Profile 状态

**核心实现**：`lib/presentation/providers/profile_provider.dart`

| Provider | 状态范围 | 关键行为 |
|----------|---------|---------|
| **`profileNotifierProvider`** | 当前加载的 Profile 数据 | `loadProfile()` / `saveProfile()` / `clearProfile()`；500ms debounce 自动保存 |
| **`fieldHistoriesProvider`** | 所有字段历史记录 | `addHistory()` / `clearHistories()` |

### L4.3 Unified Object 状态

**核心实现**：`lib/presentation/providers/unified_object_*.dart`

| Provider | 状态范围 | 关键行为 |
|----------|---------|---------|
| **`unifiedObjectProvider`** | 所有 UnifiedObject 的权威数据源 | `addObject()` / `updateObject()` / `deleteObject()` / `restoreObject()` |
| **`unifiedObjectCacheProvider`** | 预计算索引缓存 | `objectById` / `itemChildren` / `workspaceChildren` / `rootObjects`；O(1) 查询 |
| **`unifiedObjectNotifier`** | 对象变更通知 | 驱动 UI 重建 |

### L4.4 敏感度状态

**核心实现**：`lib/presentation/providers/sensitivity_provider.dart`

| Provider | 状态范围 | 关键行为 |
|----------|---------|---------|
| **`formFieldRegistryProvider`** | 全局字段敏感度注册表 | 所有已知字段的默认敏感度级别 |
| **`accountStyleProvider`** | 账户级字段敏感度覆盖 | 用户对特定字段的自定义敏感度 |

### L4.5 LLM 状态

**核心实现**：`lib/presentation/providers/llm/`

| Provider | 状态范围 | 关键行为 |
|----------|---------|---------|
| **`llmConfigProvider`** | LLM 配置（后端、模型、Key、参数） | 增删改配置项 |
| **`llmChatSessionProvider`** | 当前会话的消息流 | `sendMessage()`；流式更新 AI 回复 |
| **`chatSessionListProvider`** | 所有会话列表 | 创建/删除/重命名 |
| **`selectedChatSessionIdProvider`** | 当前选中的会话 ID | |
| **`llmModelProvider`** | 可用模型列表与状态 | 加载/下载状态 |

### L4.6 扫描状态

**核心实现**：`lib/presentation/providers/scan/`

| Provider | 状态范围 | 关键行为 |
|----------|---------|---------|
| **`localSearchProvider`** | 扫描全流程状态 | `startScan()` / `prepareImport()` / `performAiMapping()` / `doImport()` / `reset()` |
| **`localSearchState`** | 扫描状态机 | `idle` / `scanning` / `importPreview` / `importing` / `completed` |
| **`scanConfigProvider`** | 扫描配置（路径、深度、扩展名） | |

### L4.7 其他状态 Provider

| Provider | 状态范围 | 位置 |
|----------|---------|------|
| **`syncProvider`** | 同步页面状态（设备列表、日志） | `lib/presentation/providers/sync_provider.dart` |
| **`pluginProvider`** / `pluginDashboardProvider` | 插件市场/看板状态 | `lib/presentation/providers/plugin_provider.dart` |
| **`searchProvider`** | 搜索状态（关键词、结果、过滤） | `lib/presentation/providers/search_provider.dart` |
| **`operationLogProvider`** | 操作日志列表 | `lib/presentation/providers/operation_log_provider.dart` |
| **`trashFilterProvider`** | 回收站过滤条件 | `lib/presentation/providers/trash_filter_provider.dart` |
| **`languageProvider`** | 当前语言 Locale | `lib/presentation/providers/language_provider.dart` |

---

## L5 UI 组件与设计系统层

> **定位**：可复用的 UI 构建块，不持有业务逻辑，通过参数和回调与上层交互。所有组件遵循 Liquid Glass 设计语言。

### L5.1 设计系统基础

| 组件/系统 | 描述 | 位置 |
|-----------|------|------|
| **Liquid Glass 包装器** | 全局玻璃质感启用 | `LiquidGlassWidgets.wrap()`；`GlassTheme`；`GlassAdaptiveScope` |
| **Material 3 主题** | 明暗主题配置 | `lib/presentation/theme/app_theme.dart`；`lightTheme` / `darkTheme`；`ThemeMode.system` |
| **Glass 适配器** | Liquid Glass 与 Material 的桥接组件 | `lib/presentation/theme/glass_adapters.dart`；`SoloGlassAppBar` 等 |
| **SnackBar 系统** | 全局提示消息 | `showOverlaySnackBar()`；`SnackBarType`：success / error / warning / info |
| **图标解析器** | 字符串图标名 → IconData | `lib/presentation/utils/icon_resolver.dart` |

### L5.2 布局组件

| 组件 | 描述 | 位置 |
|------|------|------|
| **`ScaffoldWithSidebar`** | 带常驻侧边栏的页面骨架 | `lib/presentation/widgets/scaffold_with_sidebar.dart` |
| **`AppSidebar`** | 常驻侧边栏（页面树、账户、设置入口） | `lib/presentation/widgets/app_sidebar.dart` |
| **侧边栏子组件** | 头部、页面树瓷砖、导航瓷砖、添加输入 | `lib/presentation/widgets/sidebar/*.dart` |

### L5.3 数据展示组件

| 组件 | 描述 | 位置 |
|------|------|------|
| **`ObjectCard`** | 对象卡片（含编辑模式、历史、属性列表、附件） | `lib/presentation/widgets/object_card/`（8 个子组件） |
| **`ObjectTile`** | 对象列表项瓷砖 | `lib/presentation/widgets/object_tile.dart` |
| **`SectionCard`** | 分区卡片 | `lib/presentation/widgets/section_card.dart` |
| **`EntryCardWidget`** | 条目卡片（用于列表展示） | `lib/presentation/widgets/entry_card_widget.dart` |
| **`UniversalEntryCard`** | 通用条目卡片 | `lib/presentation/widgets/universal_entry_card.dart` |
| **`DynamicSectionCard`** | 动态分区卡片（根据类型渲染） | `lib/presentation/widgets/dynamic_section_card.dart` |
| **`SectionRendererRegistry`** | 分区渲染器注册表 | `lib/presentation/widgets/section_renderer_registry.dart` |
| **`PredefinedObjectSection`** | 预置对象分区 | `lib/presentation/widgets/predefined_object_section.dart` |

### L5.4 敏感数据组件

| 组件 | 描述 | 位置 |
|------|------|------|
| **`SensitivityTag`** | 敏感度级别标签（颜色+文字） | `lib/presentation/widgets/sensitivity_tag.dart` |
| **`SensitiveValueWidget`** | 敏感值展示（默认掩码，点击揭示） | `lib/presentation/widgets/sensitive_value_widget.dart` |
| **`SensitivityBlurredWidget`** | 模糊遮罩容器 | 与 `SensitiveValueWidget` 配合使用 |

### L5.5 表单与输入组件

| 组件 | 描述 | 位置 |
|------|------|------|
| **`IconPickerSheet`** / **`IconPicker`** | 图标选择器（分类网格） | `lib/presentation/widgets/icon_picker_sheet.dart`；`home/icon_picker.dart` |
| **`SemanticTypePicker`** | 语义类型选择器 | `lib/presentation/widgets/semantic_type_picker.dart` |
| **`DatePickerFormField`** | 日期选择表单字段 | `lib/presentation/widgets/date_picker_form_field.dart` |
| **`CharacterCounter`** | 字符计数器 | `lib/presentation/widgets/object_editor/character_counter.dart` |
| **`ResponsiveLabelField`** | 响应式标签-字段布局 | `lib/presentation/widgets/responsive_label_field.dart` |
| **`FormFieldDef`** | 表单字段定义组件 | `lib/presentation/widgets/form_field_def.dart` |
| **`CategorizedIconGrid`** | 分类图标网格 | `lib/presentation/widgets/categorized_icon_grid.dart` |

### L5.6 对话框组件

| 组件 | 描述 | 位置 |
|------|------|------|
| **`PasswordVerificationDialog`** | 密码验证对话框（统一入口） | `lib/presentation/widgets/password_verification_dialog.dart` |
| **`ChangePasswordDialog`** | 修改密码对话框 | `lib/presentation/widgets/change_password_dialog.dart` |
| **`LockVaultDialog`** | 锁定 Vault 确认对话框 | `lib/presentation/widgets/lock_vault_dialog.dart` |
| **`AddSectionDialog`** | 添加分区对话框 | `lib/presentation/widgets/add_section_dialog.dart` |
| **`FolderPickerDialog`** | 文件夹选择对话框 | `lib/presentation/widgets/folder_picker_dialog.dart` |
| **`FieldHistoryDialog`** | 字段历史查看对话框 | `lib/presentation/widgets/field_history_dialog.dart` |
| **`PluginConsentDialog`** | 插件授权对话框 | `lib/presentation/widgets/plugin_consent_dialog.dart` |
| **`PluginDetailDialog`** | 插件详情对话框 | `lib/presentation/widgets/plugin_detail_dialog.dart` |
| **`PluginAccessReviewDialog`** | 插件权限审查对话框 | `lib/presentation/widgets/plugin_access_review_dialog.dart` |
| **`ImportPreviewDialog`** | 导入预览对话框 | `lib/presentation/widgets/export_import/import_preview_dialog.dart` |
| **`PptxPreviewDialog`** | PPTX 预览对话框 | `lib/presentation/widgets/pptx_preview_dialog.dart` |
| **`LegalDocumentSheet`** | 法律文档底部弹窗 | `lib/presentation/widgets/legal_document_sheet.dart` |
| **`AttachmentListSheet`** | 附件列表底部弹窗 | `lib/presentation/widgets/attachment_list_sheet.dart` |

### L5.7 首页专用组件

| 组件 | 描述 | 位置 |
|------|------|------|
| **`PageEditor`** | 页面编辑器（内联） | `lib/presentation/widgets/home/page_editor.dart` |
| **`QuickActionTile`** / **`QuickAction`** | 快速操作瓷砖 | `lib/presentation/widgets/home/quick_action_tile.dart` |
| **`AddButton`** | 添加按钮 | `lib/presentation/widgets/home/add_button.dart` |
| **`AddQuickActionDialog`** | 添加快捷操作对话框 | `lib/presentation/widgets/home/add_quick_action_dialog.dart` |
| **`SecurityItem`** | 安全项提醒 | `lib/presentation/widgets/home/security_item.dart` |
| **`DeleteBadge`** | 删除标记 | `lib/presentation/widgets/home/delete_badge.dart` |
| **`DashedPlaceholder`** | 虚线占位 | `lib/presentation/widgets/home/dashed_placeholder.dart` |

### L5.8 LLM 专用组件

| 组件 | 描述 | 位置 |
|------|------|------|
| **`LlmChatPanel`** | 聊天面板（消息列表 + 输入框） | `lib/presentation/widgets/llm/llm_chat_panel.dart` |
| **`LlmChatBubble`** | 聊天消息气泡 | `lib/presentation/widgets/llm/llm_chat_bubble.dart` |
| **`ChatSessionSidebar`** | 会话列表侧边栏 | `lib/presentation/widgets/llm/chat_session_sidebar.dart` |
| **`EmptyProfilesState`** | 空状态（无 Profile 数据时） | `lib/presentation/widgets/llm/empty_profiles_state.dart` |

### L5.9 扫描专用组件

| 组件 | 描述 | 位置 |
|------|------|------|
| **`ScanDocumentButton`** | 扫描文档按钮 | `lib/presentation/widgets/scan_document_button.dart` |
| **`ScanProgressBanner`** | 扫描进度横幅 | `lib/presentation/widgets/scan_progress_banner.dart` |
| **`OcrScannerSheet`** | OCR 扫描底部弹窗 | `lib/presentation/widgets/ocr_scanner_sheet.dart` |
| **`OcrScannerResultCard`** | OCR 结果卡片 | `lib/presentation/widgets/ocr_scanner_result_card.dart` |
| **`OcrScannerActionButton`** | OCR 操作按钮 | `lib/presentation/widgets/ocr_scanner_action_button.dart` |
| **`OcrScannerLlmSection`** / **`LlmOption`** | LLM 提取选项 | `lib/presentation/widgets/ocr_scanner_llm_section.dart` |
| **`ExtractedFieldsPreview`** | 提取字段预览 | `lib/presentation/widgets/extracted_fields_preview.dart` |
| **`MrzPreviewCard`** | MRZ 预览卡片 | `lib/presentation/widgets/mrz_preview_card.dart` |

### L5.10 设置/数据管理专用组件

| 组件 | 描述 | 位置 |
|------|------|------|
| **`SettingsTile`** | 设置项瓷砖 | `lib/presentation/widgets/settings/settings_tile.dart` |
| **`BiometricSettingsWidget`** | 生物识别设置控件 | `lib/presentation/widgets/biometric_settings_widget.dart` |
| **`VaultInfoCard`** | Vault 信息卡片 | `lib/presentation/widgets/data_management/vault_info_card.dart` |
| **`BackupSection`** / **`RestoreSection`** | 备份/恢复区域 | `lib/presentation/widgets/data_management/` |
| **`BackupProgressIndicator`** | 备份进度指示器 | `lib/presentation/widgets/data_management/backup_progress_indicator.dart` |
| **`TrashFilterSection`** | 回收站过滤区 | `lib/presentation/widgets/trash/trash_filter_section.dart` |
| **`UnifiedObjectTrashCard`** | 回收站对象卡片 | `lib/presentation/widgets/trash/unified_object_trash_card.dart` |
| **`OperationLogFilterSection`** | 操作日志过滤区 | `lib/presentation/widgets/operation_log_filter_section.dart` |
| **`OperationTile`** | 操作日志条目 | `lib/presentation/widgets/operation_tile.dart` |
| **`SearchFilters`** / **`SearchResultTile`** / **`SearchEmptyState`** | 搜索相关 | `lib/presentation/widgets/search_*.dart` |
| **`HeaderActionButtons`** | 页面头部操作按钮组 | `lib/presentation/widgets/header_action_buttons.dart` |

---

## L6 页面与路由层

> **定位**：完整的用户界面页面，组装 L5 组件并绑定 L4 状态。每个页面对应一个独立的路由。

### L6.1 路由系统

**核心实现**：`lib/core/router/app_router.dart`

| 特性 | 描述 |
|------|------|
| **路由器** | `GoRouter` 声明式路由 |
| **路由守卫** | 根据 `authState` 自动重定向（锁定 → 登录页） |
| **Deep Link** | 支持 URL 直接访问特定页面 |
| **返回路由** | `SoloGlassAppBar` 支持 `backRoute` 参数 |

### L6.2 启动与引导页面

| 页面 | 描述 | 路由 |
|------|------|------|
| **Splash / Bootstrap** | 启动画面 + 后台初始化 | `/`（初始化完成后自动跳转） |

### L6.3 认证页面

| 页面 | 描述 | 路由 |
|------|------|------|
| **LoginPage** | 登录/创建账户/生物识别解锁 | `/login` |

### L6.4 首页与 Dashboard

| 页面 | 描述 | 路由 |
|------|------|------|
| **HomePage** | 主仪表板（快速操作 + 页面树 + 安全提醒） | `/home` |

### L6.5 对象管理页面

| 页面 | 描述 | 路由 |
|------|------|------|
| **ObjectWorkspacePage** | 对象工作区（层级浏览） | `/objects` / `/objects/:id` |
| **ObjectEditorPage** | 对象编辑器（创建/编辑） | `/objects/edit` / `/objects/edit/:id` |

### L6.6 分类数据页面

| 页面 | 描述 | 路由 |
|------|------|------|
| **ProfilePage** | 个人资料 | `/profile` |
| **TravelPage** | 旅行信息 | `/travel` |
| **FinancialPage** | 财务信息 | `/financial` |
| **ProfessionalPage** | 职业信息 | `/professional` |

### L6.7 功能工具页面

| 页面 | 描述 | 路由 |
|------|------|------|
| **SearchPage** | 全局搜索 | `/search` |
| **TrashPage** | 回收站 | `/trash` |
| **OperationLogPage** | 操作日志 | `/operation-log` |

### L6.8 设置页面（主设置 + 子页面）

| 页面 | 描述 | 路由 |
|------|------|------|
| **SettingsPage** | 设置主页（包含所有子 Section） | `/settings` |
| **SecuritySettingsPage** | 安全设置（自动锁定、生物识别等） | `/settings/security` |
| **SensitivitySettingsPage** | 敏感度设置（字段注册表浏览） | `/settings/sensitivity` |

### L6.9 高级功能页面

| 页面 | 描述 | 路由 |
|------|------|------|
| **PluginDashboardPage** | 插件看板（安装/卸载/运行） | `/plugins` |
| **SyncPage** | 设备同步（mDNS + 手动连接） | `/sync` |
| **LlmChatPage** | AI 聊天对话 | `/llm_chat` |
| **LlmConfigPage** | LLM 配置 | `/llm_config` |
| **LlmStatsPage** | LLM 用量统计 | `/llm_stats` |
| **DataManagementPage** | 数据管理（备份/恢复/统计） | `/settings/data-management` |
| **ExportImportPage** | 导出/导入 | `/settings/export-import` |

### L6.10 扫描流程页面

| 页面 | 描述 | 路由 |
|------|------|------|
| **LocalSearchConfigPage** | 扫描配置（路径、深度） | `/scan/config` |
| **LocalSearchProgressPage** | 扫描进度 | `/scan/progress` |
| **ScanPreviewPage** | 扫描结果预览与选择导入 | `/scan/preview` |
| **ScanImportResultPage** | 导入结果展示 | `/scan/result` |

### L6.11 模板与编辑器页面

| 页面 | 描述 | 路由 |
|------|------|------|
| **SectionTemplatePage** | 分区模板选择 | `/templates` |
| **PageEditorPage** | 页面编辑器（独立全屏） | `/page-editor/:id` |

---

## L7 应用入口与全局系统

> **定位**：应用生命周期、全局初始化、跨层协调。这是代码执行的起点。

### L7.1 启动引导（Bootstrap）

**核心实现**：`lib/main.dart` → `AppBootstrap`

| 阶段 | 初始化内容 | 失败策略 |
|------|-----------|---------|
| **1. Shader 预热** | `LiquidGlassWidgets.initialize()` | 阻塞，失败则启动失败 |
| **2. Rust FFI 初始化** | `RustLib.init()`；动态库路径解析 | 阻塞，失败则启动失败 |
| **3. 原生通道** | `NativeChannelService.initialize()`（macOS） | 非阻塞 |
| **4. 用户指南索引** | `UserGuideService.loadIndex()` | 后台异步，失败不阻塞 |
| **5. OCR 预热** | `OcrService.initialize()` | 后台异步，失败不阻塞 |
| **6. 临时文件清理** | 删除超过 1 小时的残留文件 | 后台异步 |
| **7. 安全设置加载** | `SecurityService.loadSettings()` | 后台异步 |
| **8. 生物识别初始化** | `BiometricCredentialService.initialize()` | 后台异步 |
| **9. QuickLook 初始化** | `QuickLookService().initialize()` | 后台异步 |
| **10. 版本检测** | `AppVersionTracker.checkVersion()` | 后台异步 |

### L7.2 主应用（SoloSoulApp）

**核心实现**：`lib/main.dart` → `SoloSoulApp`

| 系统 | 描述 | 技术细节 |
|------|------|---------|
| **生命周期监听** | `WidgetsBindingObserver` | `paused`/`inactive` → 启动自动锁定倒计时；`resumed` → 检查是否需锁定；`detached` → 清理剪贴板监控 |
| **自动锁定** | 窗口失焦/后台后延迟锁定 | 根据 `SecurityService.settings.autoLockDelayMinutes`；倒计时期间用户交互重置计时 |
| **状态擦除** | 锁定后清除敏感内存状态 | `profileNotifier.clearProfile()` / `fieldHistories.clearHistories()` / `unifiedObjectProvider.reset()` |
| **路由守卫** | 状态变为 locked 时自动跳转登录 | `ref.listen(authNotifierProvider)` → `_router.go('/login')` |
| **全局错误处理** | `FlutterError.onError` + `ErrorWidget.builder` | 记录到 `DebugLogger`；已知 bug（HardwareKeyboard）静默忽略 |
| **用户交互追踪** | 记录最后活动时间 | `Listener.onPointerDown/Move` → `_recordActivity()` |

### L7.3 调试与诊断系统

| 系统 | 描述 | 位置 |
|------|------|------|
| **DebugLogger** | 文件日志（Release 诊断用） | `lib/core/services/debug_logger.dart`；`/tmp/solosoul_dart.log` |
| **SoloLog** | 带标签的分级日志 | `lib/core/utils/solo_log.dart`；d/i/w/e + 计时器 |
| **Rust 日志** | Rust 端文件日志 | `/tmp/solosoul_rust.log`；Mutex 保护 |

---

## L8 测试与质量保障层

> **定位**：覆盖各层的自动化测试，确保重构时行为不变。

### L8.1 Rust 测试

| 测试目标 | 文件 | 说明 |
|----------|------|------|
| Vault 迁移 | `native/src/vault/migration_tests.rs` | 数据迁移正确性 |
| 其他模块 | 各模块内 `#[cfg(test)]` | crypto、vault、sync、plugin、ocr 等 |

### L8.2 Rust 测试（native/）

| 模块 | 测试文件 | 覆盖范围 |
|------|---------|---------|
| Crypto | `cipher_test.go`, `kdf_test.go`, `secure_mem_test.go`, `utils_test.go`, `errors_test.go` | AES-GCM、KDF、安全内存、工具函数、错误 |
| Vault | `file_store_test.go` | 全生命周期（init/unlock/lock/changePassword/CRUD）|
| Schema | `profile_test.go`, `validator_test.go` | Profile 构造、字段类型、正则验证 |
| API | `account_manager_test.go`, `plugin_manager_test.go`, `plugin_test.go`, `types_test.go` | 账户、插件、类型 |
| OCR | `engine_test.go`, `job_test.go`, `mrz_test.go`, `preprocess_test.go` | MRZ、Job 生命周期、图像预处理 |

### L8.3 Dart/Flutter 测试

| 类型 | 目录 | 覆盖范围 |
|------|------|---------|
| **单元测试** | `test/unit/` | Provider 逻辑、迁移指纹、版本检测、Vault Service |
| **Widget 测试** | `test/widget/` | 页面渲染、敏感标签组件、旅行页面交互 |
| **集成测试** | `integration_test/` / `test/integration_test/` | 应用启动导航、OCR 对话框、FFI 端到端（创建/保存/加载/删除 Profile）|

### L8.4 CI/CD 质量门

| 工作流 | 检查项 |
|--------|--------|
| **PR Check** | `cargo fmt --check` → `cargo clippy -- -D warnings` → `dart analyze --fatal-infos --fatal-warnings` → Rust 测试 → Dart 单元测试 → Widget 测试 |
| **CI/CD** | 上述全部 + macOS 集成测试 + Release 构建 + DMG 打包 + Draft Release |

---

## L9 构建工具与开发环境

> **定位**：Flutter 主项目的构建系统、CI/CD 流水线、开发工具链。属于支撑层，不直接参与运行时功能。
>
> **说明**：以下已废弃组件已从代码库中移除，不再参与 Tauri 迁移：
> - Go HTTP API 服务器（`cmd/solosould/`）
> - Go CLI 客户端（`cmd/solosoul/`）
> - Go 业务核心库（`core/`）
> - Web UI（`web/`，Next.js 项目）
> - crypto-argon2（`crypto-argon2/`，供 Go 后端 CGO 调用的 Rust FFI 库）
>
> Flutter 主项目的实际架构始终是 **Flutter（Dart UI）+ Rust（原生核心，通过 flutter_rust_bridge FFI）**，上述组件是独立的遗留交付物，从未被 Flutter 客户端直接依赖。

### L9.1 构建系统

| 构建项 | 命令 | 输出 |
|--------|------|------|
| **Flutter macOS Release** | `flutter build macos --release --obfuscate --split-debug-info` | `.app` + DMG |
| **Rust Native 库** | `cd flutter/native && cargo build --release` | `.dylib` / `.a` / `.so` |
| **DMG 打包** | `./build_dmg.sh` | `SoloSoul-v1.0.dmg` |

### L9.2 CI/CD 流水线

| 流水线 | 内容 |
|--------|------|
| **PR Check** | `cargo fmt --check` → `cargo clippy` → `dart analyze` → Rust 测试 → Dart 单元测试 → Widget 测试 |
| **CI/CD** | 上述全部 + macOS 集成测试 + Release 构建 + DMG 打包 + Draft Release |

### L9.3 开发工具链

| 工具 | 版本 | 用途 |
|------|------|------|
| Flutter | 3.41.6 | UI 框架 |
| Dart | 3.6 | 编程语言 |
| Rust | stable | 原生核心 |
| Xcode | 16 | macOS/iOS 构建 |

---

## 跨层依赖矩阵

> 展示各功能领域在 L0–L7 中的分布，以及 Tauri 迁移时的"下沉"方向。

| 功能领域 | L0 平台 | L1 数据 | L2 加密 | L3 服务 | L4 状态 | L5 UI 组件 | L6 页面 | L7 全局 |
|---------|--------|--------|--------|--------|--------|-----------|--------|--------|
| **多账户管理** | — | ✅ Vault | ✅ KDF | ✅ 服务 | ✅ Provider | — | ✅ 登录 | ✅ 引导 |
| **Vault 加解密** | — | ✅ SQLCipher | ✅ AES-GCM | ✅ 服务 | ✅ 状态 | — | — | ✅ 生命周期 |
| **Unified Object** | — | ✅ Profile 存储 | — | ✅ 类型注册表 | ✅ 对象状态 | ✅ 卡片/编辑器 | ✅ 工作区 | — |
| **敏感数据分级** | — | — | — | ✅ 字段注册表 | ✅ 覆盖状态 | ✅ 标签/遮罩 | ✅ 设置页 | — |
| **生物识别** | ✅ Keychain | — | — | ✅ 凭证服务 | ✅ 状态 | — | ✅ 登录 | ✅ 初始化 |
| **自动锁定** | ✅ 生命周期 | — | — | — | ✅ 状态 | — | — | ✅ 计时器 |
| **OCR 引擎** | ✅ Asset 加载 | — | — | ✅ 引擎服务 | — | ✅ 扫描组件 | ✅ 扫描页 | ✅ 预热 |
| **本地文件扫描** | ✅ 文件系统 | ✅ 扫描缓存 | — | ✅ 扫描服务 | ✅ 扫描状态 | ✅ 进度/预览 | ✅ 扫描流程 | — |
| **LLM 对话** | ✅ HTTP | — | ✅ API Key 存储 | ✅ 云/本地服务 | ✅ 会话状态 | ✅ 气泡/面板 | ✅ 聊天页 | — |
| **插件系统** | — | ✅ 插件存储 | — | ✅ Wasmtime | ✅ 插件状态 | ✅ 卡片/对话框 | ✅ 看板 | — |
| **同步引擎** | ✅ mDNS/网络 | — | ✅ Noise | ✅ CRDT | ✅ 同步状态 | — | ✅ 同步页 | — |
| **备份恢复** | ✅ 文件系统 | ✅ 备份目录 | ✅ 加密 | ✅ 备份服务 | — | ✅ 进度卡片 | ✅ 数据管理 | ✅ 版本检测 |
| **搜索** | — | ✅ 索引 | — | ✅ 搜索服务 | ✅ 搜索状态 | ✅ 结果瓷砖 | ✅ 搜索页 | — |
| **操作日志** | ✅ 文件日志 | ✅ 日志文件 | — | ✅ 日志服务 | ✅ 日志状态 | ✅ 日志条目 | ✅ 日志页 | — |
| **回收站** | — | ✅ 软删除标记 | — | ✅ 对象服务 | ✅ 过滤状态 | ✅ 回收卡片 | ✅ 回收页 | — |
| **主题/i18n** | ✅ 系统 Locale | ✅ 偏好存储 | — | — | ✅ 语言状态 | ✅ Glass 主题 | ✅ 设置页 | ✅ 初始化 |

### Tauri 迁移时的层下沉方向

| 当前（Flutter） | Tauri 目标 | 说明 |
|----------------|-----------|------|
| L3 服务（Dart FFI 调用 Rust） | **全部下沉到 Rust 后端** | 作为 `tauri::command` 暴露 |
| L2 加密（Rust 实现） | **提取为独立 Rust crate** | `solosoul-crypto`，解耦 FFI 绑定 |
| L1 数据（SQLCipher + 文件） | **Rust 后端直接持有** | `rusqlite` + 文件系统 |
| L0 平台（Flutter 插件） | **替换为 Tauri API / 自定义命令** | `local_auth` → `tauri-plugin-biometric` 或系统命令 |
| L4 状态（Riverpod） | **前端状态管理**（React/Zustand 等） | 部分状态下沉到 Rust（如 auth） |
| L5/L6（Dart UI） | **重写为 Web 前端**（React/Vue/Svelte） | 复刻 Liquid Glass 视觉 |
| L7（main.dart） | **Tauri 入口 + 前端生命周期** | Rust `main()` + JS `beforeUnload` |

---

*文档版本：v1.0*  
*创建日期：2026-06-04*  
*状态：用于 Tauri 迁移参考*
