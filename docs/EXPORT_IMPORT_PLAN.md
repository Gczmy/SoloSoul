# 数据管理页面 — 导入/导出功能实施计划

## 概述

在数据管理页面 (`/settings/data-management`) 新增 **导入/导出** 子页面，支持将 SoloSoul 账户数据打包为可移植的加密文件（`.solosoul`），并在导入时提供分区粒度的选择 UI。

## 可行性分析

| 组件 | 可行性 | 说明 |
|------|--------|------|
| 文件加密 | ✅ | `frbEncryptWithKey` / `frbDecryptWithKey` 已存在，直接使用（无需新增 Rust 代码）|
| 文件打包 | ✅ | `archive` crate (Rust) 或 Dart `archive` package 均可创建 ZIP；Dart 端更简单 |
| 文件选择/保存 | ✅ | `file_picker: ^11.0.2` 已在 pubspec.yaml，支持 `saveFile` + `pickFiles` |
| 密码派生 | ✅ | `frbDeriveKey` 已暴露 Argon2id KDF，可导出密码 + salt → 32 字节 key |
| 分区粒度导入 | ✅ | UnifiedObject 模型天然支持：`typeId == 'collection'` 为分区，childrenIds/parentId 为归属关系 |
| 附件处理 | ✅ | 现有 `AttachmentPoolService` / `AttachmentStorageService` 可复用 |

### 关键约束
- **Vault 必须已解锁**：导出/导入均需在登录状态下操作，以便读取/写入 Vault
- **Release 构建避免 Isolate.run**：Dart 端 JSON 序列化必须在主线程执行（macOS Release 构建中 `Isolate.run` 会死锁）。ZIP 打包/解压委托给 Rust 层（`zip` crate）处理，不受此限制
- **ZIP 实现**：使用 Rust `zip` crate 在 Rust 层处理 ZIP 打包/解压，通过 FRB 暴露接口。避免使用系统 `zip` 命令（Windows/Android 不可用）或 Dart `archive` package（纯 Dart 性能差、内存占用高）

---

## 架构决策

### 导出文件格式：`.solosoul`（ZIP 封装）

```
{accountName}_{accountId}.solosoul
├── manifest.json              ← 明文：格式版本、导出时间（不含 accountName/accountId 等敏感信息）
├── checksums.json             ← 各文件的 SHA-256 校验和（明文，用于传输完整性检测；ZIP 无签名机制，不防恶意篡改）
├── account.enc                ← SOLO blob v2：password_hint、export_verify_token、export_salt（base64）
├── profile.enc                ← SOLO blob v2：ProfileData.toJson() 的完整 JSON（含 customTypes）
├── preferences.enc            ← SOLO blob v2（可选）：应用偏好设置 — 第一阶段暂不导出，后续根据需求添加
└── attachments/
      ├── {fileId}.enc         ← 附件原始文件，用导出 key 重新加密（AES-256-GCM SOLO blob）
      └── manifest.json        ← 附件元数据清单（fileId → originalFileName, mimeType, size）
```

**加密策略**：
1. 导出时生成 **新的 32 字节随机 salt**（`exportSalt`）
2. 使用 `frbDeriveKey(password, exportSalt, memoryKib=16384, iterations=3, parallelism=4)` 派生 32 字节 key
   - 使用 **Balanced preset**（16 MiB, 3 iterations），与 Vault KDF 同级。导出文件是长期保存/传输的，安全强度不应低于 Vault
3. 用该 key 调用 `frbEncryptWithKey`（已有 FRB 函数，无需新增 Rust 代码）加密 `account.enc`、`profile.enc`
4. 附件文件 **用导出 key 重新加密**（调用 `frbEncryptFile` 流式加密），生成 `.enc` 文件
   - 原因：原始 `.solo` 文件是用导出账户的 Vault session key 加密的。若导入到不同账户（不同 session key），直接复制无法解密
   - 重新加密后，导入时可用导出 key 解密，再用目标账户 Vault session key 重新加密存储

> **性能说明**：附件重新加密涉及两次加解密（Vault session key → 明文 → exportKey）和临时磁盘写入。大附件（如 84MB）会临时占用双倍磁盘空间（原文件 + 新加密文件），导出结束后自动清理。UI 中应显示"正在处理附件"的进度提示。

**密码验证（独立令牌）**：
- 导出包中**不包含**原始 Vault 的 salt、verify_hash 或 KDF 参数（避免泄露账户安全信息）
- 导出时加密一个已知常量字符串 `"SOLOSOUL_EXPORT_VERIFY_v1"` → `exportVerifyToken`，存入 `account.enc`
- 导入时，用输入密码派生 exportKey，尝试解密 `exportVerifyToken`。若结果为 `"SOLOSOUL_EXPORT_VERIFY_v1"`，密码正确
- 这样导出密码与 Vault 密码完全解耦，即使导出包泄露，也无法反推 Vault 密码

### 导入流程

```
1. 点击"导入" → FilePicker.pickFiles(allowedExtensions: ['.solosoul'])
2. Rust 层解压 ZIP → 提取 manifest.json、checksums.json
3. 校验 checksums.json 中各文件的 SHA-256，确保文件未损坏
4. 弹出密码输入对话框（可显示 password_hint，来自 account.enc 解密后）
5. 用户输入密码 → frbDeriveKey(password, exportSalt, ...) → exportKey
6. frbDecryptWithKey(account.enc, exportKey) → 获取 exportVerifyToken
7. 比对 exportVerifyToken 是否等于 "SOLOSOUL_EXPORT_VERIFY_v1" → 密码验证
8. frbDecryptWithKey(profile.enc, exportKey) → ProfileData JSON
9. 解析 JSON → 构建导出数据预览模型（包含 unifiedObjects + customTypes）
10. 弹出"导入预览"对话框：
     - 左侧：所有 collection（分区）列表 + checkbox
     - 每个 collection 显示：名称、子项数量、敏感度最高的级别
     - 右侧（或下方）：每个已勾选 collection 的"导入到页面"下拉选择器
         - 下拉选项：当前账户的所有 page（typeId == 'page'）
         - 默认选择：同名 page，若无则选第一个 page
11. 用户勾选并配置后 → 点击"导入"
    - **导入为增量添加**：不会删除或覆盖现有数据，仅将选中的 collection 及其子对象合并到当前 ProfileData
12. 【保护性操作】导入前自动创建当前数据的静默备份（调用 BackupService.createBackup）
13. 对每个选中的 collection：
     - 复制 collection 对象及其所有子孙对象到当前 ProfileData
     - 重新生成所有对象 ID（避免冲突）
     - 更新 parentId 映射到新 page/collection
     - 更新 RelationProperty 的 targetObjectId（通过 idMapping 映射到新 ID）
     - 处理 customTypes：合并到当前账户的 customTypes，冲突时重命名 ID
     - 用 exportKey 解密附件 .enc 文件，再用当前 Vault session key 重新加密为 .solo，存入 attachment pool
14. 保存更新后的 ProfileData 到 Vault
15. 记录 OperationLog（类型：import）
16. 刷新 UI provider
```

---

## 文件变更清单

### Rust 层（`flutter/native/`）

| 文件 | 变更 | 说明 |
|------|------|------|
| `src/api.rs` | 新增 2 个 FRB 函数 | `frb_create_zip_package`、`frb_extract_zip_package`（ZIP 打包/解压） |
| `src/api.rs` | 无需新增 | `frb_encrypt_with_key` / `frb_decrypt_with_key` 已存在，直接使用 |
| `src/crypto/mod.rs` | 无需变更 | 复用现有 `encrypt_profile_data` / `decrypt_profile_data` |

### Dart 服务层（`flutter/lib/core/`）

| 文件 | 变更 | 说明 |
|------|------|------|
| `core/services/export_import_service.dart` | 新建 | 核心服务：导出打包、导入解析、密码验证 |
| `core/services/export_import_models.dart` | 新建 | 数据模型：`ExportPackage`、`ImportPreview`、`ImportSectionSelection` |
| `core/services/profile_storage_service.dart` | 修改 | 新增 `saveProfileDirect(String accountId, String rawJson)` 用于导入时绕过 cache 保护 |

### Dart UI 层（`flutter/lib/presentation/`）

| 文件 | 变更 | 说明 |
|------|------|------|
| `pages/export_import_page.dart` | 新建 | 导入/导出主页面 |
| `pages/data_management_page.dart` | 修改 | 添加"导入/导出"入口按钮 |
| `widgets/export_import/export_section.dart` | 新建 | 导出区域 UI |
| `widgets/export_import/import_section.dart` | 新建 | 导入区域 UI |
| `widgets/export_import/import_preview_dialog.dart` | 新建 | 导入预览/选择对话框（最复杂 UI） |
| `core/router/app_router.dart` | 修改 | 新增 `/settings/data-management/export-import` 路由 |

### 本地化

| 文件 | 变更 | 说明 |
|------|------|------|
| `l10n/app_zh.arb` | 新增键 | 中文翻译 |
| `l10n/app_en.arb` | 新增键 | 英文翻译 |

---

## 详细实施步骤

### Phase 1：Rust ZIP 接口（P0）

> **说明**：`frb_encrypt_with_key` / `frb_decrypt_with_key` 已在代码库中存在（`frb/api.dart` 第167行），无需新增。本阶段只需新增 ZIP 打包/解压接口。

**Step 1.1**：在 `flutter/native/src/api.rs` 新增 FRB 函数：

```rust
/// Create a ZIP package from a directory.
/// Streams files into ZIP to keep memory usage low.
#[frb]
pub fn frb_create_zip_package(src_dir: String, dst_path: String) -> Result<(), String> {
    use std::fs::File;
    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;
    use std::io::{Write, Read};
    
    let file = File::create(&dst_path)
        .map_err(|e| format!("Failed to create ZIP file: {}", e))?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    
    let walkdir = walkdir::WalkDir::new(&src_dir);
    let mut buffer = Vec::new();
    
    for entry in walkdir.into_iter() {
        let entry = entry.map_err(|e| format!("WalkDir error: {}", e))?;
        let path = entry.path();
        let name = path.strip_prefix(&src_dir)
            .map_err(|e| format!("Path prefix error: {}", e))?;
        
        if path.is_file() {
            let mut f = File::open(path)
                .map_err(|e| format!("Failed to open file: {}", e))?;
            f.read_to_end(&mut buffer)
                .map_err(|e| format!("Failed to read file: {}", e))?;
            zip.start_file_from_path(name, options)
                .map_err(|e| format!("ZIP start_file error: {}", e))?;
            zip.write_all(&buffer)
                .map_err(|e| format!("ZIP write error: {}", e))?;
            buffer.clear();
        }
    }
    
    zip.finish().map_err(|e| format!("ZIP finish error: {}", e))?;
    Ok(())
}

/// Extract a ZIP package to a directory.
#[frb]
pub fn frb_extract_zip_package(zip_path: String, dst_dir: String) -> Result<Vec<String>, String> {
    use std::fs::File;
    use zip::ZipArchive;
    use std::io::{copy, Read};
    
    let file = File::open(&zip_path)
        .map_err(|e| format!("Failed to open ZIP file: {}", e))?;
    let mut archive = ZipArchive::new(file)
        .map_err(|e| format!("Failed to read ZIP archive: {}", e))?;
    
    std::fs::create_dir_all(&dst_dir)
        .map_err(|e| format!("Failed to create extract dir: {}", e))?;
    
    let mut extracted = Vec::new();
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)
            .map_err(|e| format!("ZIP by_index error: {}", e))?;
        let outpath = std::path::Path::new(&dst_dir).join(file.mangled_name());
        
        if file.name().ends_with('/') {
            std::fs::create_dir_all(&outpath)
                .map_err(|e| format!("Failed to create dir: {}", e))?;
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    std::fs::create_dir_all(p)
                        .map_err(|e| format!("Failed to create parent dir: {}", e))?;
                }
            }
            let mut outfile = File::create(&outpath)
                .map_err(|e| format!("Failed to create file: {}", e))?;
            copy(&mut file, &mut outfile)
                .map_err(|e| format!("Failed to extract file: {}", e))?;
            extracted.push(outpath.to_string_lossy().to_string());
        }
    }
    
    Ok(extracted)
}
```

**Step 1.2**：在 `flutter/native/Cargo.toml` 添加依赖：
```toml
[dependencies]
zip = { version = "2.2", default-features = false, features = ["deflate"] }
walkdir = "2.5"
```

**Step 1.3**：运行 `flutter_rust_bridge_codegen generate` 重新生成 Dart FFI 绑定。

**Step 1.4**：构建 Rust Release 并验证。

### Phase 2：导出服务（P0）

**Step 2.1**：新建 `flutter/lib/core/services/export_import_service.dart`

核心导出方法：
```dart
Future<String?> exportPackage({
  required String accountId,
  required String accountName,
  required String password,
  required String? passwordHint,
  required String savePath,
}) async {
  // 1. 读取 Vault 中的 ProfileData
  final profile = await ProfileStorageService.instance.loadProfile(accountId);
  if (profile == null) return null;

  // 2. 读取账户配置（salt, verify_hash, kdf_params）
  final accountConfig = await _readAccountConfig(accountId);

  // 3. 生成导出专用 salt
  final exportSalt = await frb.frbGenerateSalt(length: 32);

  // 4. 派生导出 key（使用 Balanced preset：16 MiB, 3 iterations）
  final exportKey = await frb.frbDeriveKey(
    password: password,
    salt: exportSalt,
    memoryKib: 16384,
    iterations: 3,
    parallelism: 4,
  );

  // 5. 构建 manifest.json（**不包含敏感信息**）
  final manifest = {
    'version': '1.0',
    'exportAt': DateTime.now().toIso8601String(),
    'objectCount': profile.unifiedObjects?.objects.length ?? 0,
    'attachmentCount': await AttachmentStorageService().getAttachmentCount(accountId),
  };

  // 6. 加密 account.enc（**不包含原始 Vault 安全信息**）
  final exportVerifyPlain = Uint8List.fromList(utf8.encode('SOLOSOUL_EXPORT_VERIFY_v1'));
  final exportVerifyToken = await frb.frbEncryptWithKey(
    plaintext: exportVerifyPlain,
    key: exportKey,
  );
  final accountConfigJson = jsonEncode({
    'password_hint': passwordHint,
    'export_verify_token': base64Encode(exportVerifyToken),
    'export_salt': base64Encode(exportSalt),
  });
  final accountConfigEncrypted = await frb.frbEncryptWithKey(
    plaintext: Uint8List.fromList(utf8.encode(accountConfigJson)),
    key: exportKey,
  );

  // 7. 加密 profile（包含 unifiedObjects + customTypes）
  final profileJson = jsonEncode(profile.toJson());
  final profileEncrypted = await frb.frbEncryptWithKey(
    plaintext: Uint8List.fromList(utf8.encode(profileJson)),
    key: exportKey,
  );

  // 8. 收集附件并用 exportKey 重新加密
  final attachments = await _collectAndReEncryptAttachments(accountId, exportKey);

  // 9. 计算各文件 SHA-256 校验和
  final checksums = await _computeChecksums(workDir);

  // 10. 写入 ZIP 文件（调用 Rust FRB 接口）
  await frb.frbCreateZipPackage(
    srcDir: workDir.path,
    dstPath: savePath,
  );

  return savePath;
}
```

**Step 2.2**：处理 ZIP 创建。通过 Rust `zip` crate 流式打包，跨平台且内存友好：

```dart
Future<void> _createZipPackage({...}) async {
  final tempDir = await getTemporaryDirectory();
  final workDir = Directory('${tempDir.path}/solosoul_export_${const Uuid().v4()}');
  await workDir.create(recursive: true);

  try {
    // 写入各文件到 workDir...

    // 调用 Rust FRB 接口创建 ZIP
    await frb.frbCreateZipPackage(
      srcDir: workDir.path,
      dstPath: savePath,
    );

    // 设置文件权限（owner-only read/write）
    await BackupService.setRestrictivePermissions(savePath);
  } finally {
    // 无论成功或失败，都清理临时目录
    if (await workDir.exists()) {
      await workDir.delete(recursive: true);
    }
  }
}
```

**Step 2.3**：附件重新加密。原始 `.solo` 文件用 Vault session key 加密，导出时必须用 exportKey 重新加密：

```dart
Future<List<AttachmentEntry>> _collectAndReEncryptAttachments(
  String accountId,
  Uint8List exportKey,
) async {
  final srcFileIds = await AttachmentStorageService().getAttachmentFileIds(accountId);
  final result = <AttachmentEntry>[];
  
  for (final fileId in srcFileIds) {
    final srcPath = await AttachmentStorageService().getFilePath(accountId, fileId);
    final dstPath = '${workDir.path}/attachments/$fileId.enc';
    
    // 用 exportKey 流式加密附件
    await frb.frbEncryptFile(
      srcPath: srcPath,
      dstPath: dstPath,
      progressPath: '${tempDir.path}/enc_progress_$fileId.txt',
      cancelPath: '${tempDir.path}/enc_cancel_$fileId.txt',
    );
    
    result.add(AttachmentEntry(fileId: fileId, encryptedPath: dstPath));
  }
  
  return result;
}
```

### Phase 3：导入服务（P0）

**Step 3.1**：新建导入解析方法：

```dart
Future<ImportPreview?> parseImportPackage(String filePath) async {
  // 1. 解压 ZIP 到临时目录
  final tempDir = await _extractZip(filePath);

  // 2. 读取 manifest.json
  final manifestFile = File('${tempDir.path}/manifest.json');
  final manifest = jsonDecode(await manifestFile.readAsString());

  // 3. 读取 account.enc（仍为加密状态，仅提取 export_salt）
  final accountEncFile = File('${tempDir.path}/account.enc');
  final accountEncBytes = await accountEncFile.readAsBytes();

  return ImportPreview(
    manifest: manifest,
    accountEncBytes: accountEncBytes,
    profileEncPath: '${tempDir.path}/profile.enc',
    attachmentsDir: '${tempDir.path}/attachments',
    tempDir: tempDir,
  );
}
```

**Step 3.2**：密码验证 + 解密：

```dart
Future<ProfileData?> decryptAndVerify(ImportPreview preview, String password) async {
  // 1. export_salt 放在 manifest.json 中（明文，salt 不需要保密）
  final exportSalt = base64Decode(preview.manifest['exportSalt'] as String);

  // 2. 派生 key（使用与导出相同的 Balanced preset）
  final exportKey = await frb.frbDeriveKey(
    password: password,
    salt: exportSalt,
    memoryKib: 16384,
    iterations: 3,
    parallelism: 4,
  );

  // 3. 解密 account.enc
  final accountPlain = await frb.frbDecryptWithKey(
    ciphertext: preview.accountEncBytes,
    key: exportKey,
  );
  final accountConfig = jsonDecode(utf8.decode(accountPlain));

  // 4. 验证密码：解密 exportVerifyToken 并比对
  final verifyTokenBytes = base64Decode(accountConfig['export_verify_token'] as String);
  final verifyPlain = await frb.frbDecryptWithKey(
    ciphertext: verifyTokenBytes,
    key: exportKey,
  );
  final verifyString = utf8.decode(verifyPlain);
  if (verifyString != 'SOLOSOUL_EXPORT_VERIFY_v1') {
    throw WrongPasswordException();
  }

  // 5. 解密 profile.enc
  final profileEncFile = File(preview.profileEncPath);
  final profilePlain = await frb.frbDecryptWithKey(
    ciphertext: await profileEncFile.readAsBytes(),
    key: exportKey,
  );
  final profileJson = jsonDecode(utf8.decode(profilePlain)) as Map<String, dynamic>;
  return ProfileData.fromJson(profileJson);
}
```

> **安全设计**：导出包中**不包含**原始 Vault 的 salt、verify_hash 或 KDF 参数。密码验证完全依赖独立的 `exportVerifyToken`，即使导出包泄露也无法反推 Vault 密码。

**Step 3.3**：构建导入预览模型：

```dart
class ImportPreview {
  final Map<String, dynamic> manifest;
  final List<ImportCollection> collections;
}

class ImportCollection {
  final String originalId;
  final String name;
  final String iconName;
  final int itemCount;
  final SensitivityLevel highestSensitivity;
  final List<UnifiedObject> items; // 该 collection 下的所有对象
  bool selected;
  String? targetPageId; // 导入到当前账户的哪个 page
}
```

**Step 3.4**：构建预览数据：

```dart
List<ImportCollection> buildImportCollections(ProfileData exportedProfile) {
  final objects = exportedProfile.unifiedObjects?.objects ?? [];
  final pages = objects.where((o) => o.typeId == 'page').toList();
  final collections = objects.where((o) => o.typeId == 'collection').toList();

  return collections.map((col) {
    final items = objects.where((o) => o.parentId == col.id || o.id == col.id).toList();
    final highestSens = _computeHighestSensitivity(items);
    return ImportCollection(
      originalId: col.id,
      name: col.name ?? 'Untitled',
      iconName: col.iconName ?? 'folder',
      itemCount: items.length,
      highestSensitivity: highestSens,
      items: items,
      selected: true,
      targetPageId: null,
    );
  }).toList();
}
```

### Phase 4：导入执行（P0）

**Step 4.1**：ID 重映射 + 数据合并：

```dart
Future<bool> executeImport({
  required String currentAccountId,
  required ProfileData currentProfile,
  required List<ImportCollection> selections,
  required Uint8List exportKey,
  required String tempAttachmentsDir,
}) async {
  // 1. 导入前自动创建静默备份（保护性操作）
  // 注：第一阶段直接创建备份；后续可优化为先检查磁盘空间（预估当前数据大小的 1.2 倍），
  //     空间不足时提示用户并询问是否跳过备份（风险自担）。
  await BackupService.instance.createBackup(currentAccountId);

  final currentObjects = List<UnifiedObject>.from(
    currentProfile.unifiedObjects?.objects ?? []
  );
  final currentCustomTypes = List<ObjectTypeDefinition>.from(
    currentProfile.unifiedObjects?.customTypes ?? []
  );
  final idMapping = <String, String>{}; // oldId -> newId
  final typeIdMapping = <String, String>{}; // oldTypeId -> newTypeId

  for (final selection in selections.where((s) => s.selected)) {
    // 为每个对象生成新 ID
    for (final obj in selection.items) {
      idMapping[obj.id] = const Uuid().v4();
    }
  }

  // 处理 customTypes：合并到当前账户，冲突时重命名
  for (final selection in selections.where((s) => s.selected)) {
    for (final obj in selection.items) {
      final typeId = obj.typeId;
      if (typeId == null) continue;
      final isBuiltin = typeId.startsWith('__preset_') ||
          {'page', 'collection', 'note', 'task', 'contact', 'item'}.contains(typeId);
      if (isBuiltin) continue; // 内置类型无需映射
      
      // 检查是否已存在
      final existing = currentCustomTypes.any((t) => t.id == typeId);
      if (!existing) {
        // 查找导出数据中的 type 定义
        final exportedType = selection.exportedCustomTypes.firstWhere((t) => t.id == typeId);
        currentCustomTypes.add(exportedType);
        typeIdMapping[typeId] = typeId;
      } else {
        // 冲突：重生成 ID
        final newTypeId = const Uuid().v4();
        typeIdMapping[typeId] = newTypeId;
        // 更新导出数据中该 type 的定义并添加
        final exportedType = selection.exportedCustomTypes.firstWhere((t) => t.id == typeId);
        currentCustomTypes.add(exportedType.copyWith(id: newTypeId));
      }
    }
  }

  // 递归更新 customType properties 中的 relation targetTypeId
  for (var i = 0; i < currentCustomTypes.length; i++) {
    final type = currentCustomTypes[i];
    final newProperties = <PropertyDefinition>[];
    for (final prop in type.properties) {
      if (prop.type == PropertyType.relation) {
        final targetTypeId = prop.config?['targetTypeId'] as String?;
        if (targetTypeId != null && typeIdMapping.containsKey(targetTypeId)) {
          final newConfig = Map<String, dynamic>.from(prop.config ?? {});
          newConfig['targetTypeId'] = typeIdMapping[targetTypeId];
          newProperties.add(prop.copyWith(config: newConfig));
          continue;
        }
      }
      newProperties.add(prop);
    }
    currentCustomTypes[i] = type.copyWith(properties: newProperties);
  }

  for (final selection in selections.where((s) => s.selected)) {
    // 找到 collection 对应的 target page
    final targetPageId = selection.targetPageId ?? _findDefaultPage(currentObjects);

    // 重写对象并添加到当前 profile
    for (final obj in selection.items) {
      final newId = idMapping[obj.id]!;
      final newParentId = obj.parentId == null
          ? targetPageId
          : idMapping[obj.parentId];

      final newChildrenIds = obj.childrenIds
          .map((id) => idMapping[id])
          .whereType<String>()
          .toList();

      // 更新 RelationProperty 的 targetObjectId
      final newProperties = <String, PropertyValue>{};
      for (final entry in obj.properties.entries) {
        final value = entry.value;
        if (value is RelationProperty && value.targetObjectId != null) {
          final newTargetId = idMapping[value.targetObjectId];
          if (newTargetId != null) {
            newProperties[entry.key] = value.copyWith(targetObjectId: newTargetId);
          } else {
            // 目标对象未被导入：保留原值，记录警告日志
            newProperties[entry.key] = value;
            DebugLogger.instance.logWarning(
              'IMPORT',
              'RelationProperty target ${value.targetObjectId} not in import scope, keeping original',
            );
          }
        } else {
          newProperties[entry.key] = value;
        }
      }

      // 映射 typeId（若是 customType 且冲突）
      final newTypeId = typeIdMapping[obj.typeId] ?? obj.typeId;

      final newObj = obj.copyWith(
        id: newId,
        typeId: newTypeId,
        parentId: newParentId,
        childrenIds: newChildrenIds,
        properties: newProperties,
        createdAt: DateTime.now().millisecondsSinceEpoch,
        updatedAt: DateTime.now().millisecondsSinceEpoch,
      );
      currentObjects.add(newObj);
    }

    // 复制并重新加密附件
    for (final attachment in selection.attachments) {
      final srcEncPath = '$tempAttachmentsDir/${attachment.fileId}.enc';
      final decryptedPath = '${tempDir.path}/dec_${attachment.fileId}';
      
      // 用 exportKey 解密
      await frb.frbDecryptFile(
        srcPath: srcEncPath,
        dstPath: decryptedPath,
        progressPath: '${tempDir.path}/dec_progress.txt',
        cancelPath: '${tempDir.path}/dec_cancel.txt',
      );
      
      // 用当前 Vault session key 重新加密为 .solo
      await AttachmentStorageService().saveFile(
        accountId: currentAccountId,
        fileId: attachment.fileId,
        srcPath: decryptedPath,
      );
      
      // 清理临时解密文件
      await File(decryptedPath).delete();
    }
  }

  // 更新 current profile
  final newProfile = currentProfile.copyWith(
    unifiedObjects: (currentProfile.unifiedObjects ?? const UnifiedObjectData(objects: []))
        .copyWith(objects: currentObjects, customTypes: currentCustomTypes),
  );

  // 保存到 Vault
  final saved = await ProfileStorageService.instance.saveProfile(
    currentAccountId, newProfile, preserveCache: false,
  );

  // 记录操作日志
  if (saved) {
    await OperationLogService.instance.addEntry(
      OperationLogger.logImport(
        action: LogAction.import,
        description: 'Imported ${selections.where((s) => s.selected).length} collections',
      ),
    );
  }

  return saved;
}
```

### Phase 5：UI 实现（P1）

**Step 5.1**：新建 `ExportImportPage` 页面

布局：
- 顶部：两个 Tab（导出 / 导入）或上下分区
- 导出区：
  - 显示当前账户信息（名称、ID）
  - "导出全部数据" 按钮
  - 点击后弹出 `FilePicker.saveFile` 选择保存位置
  - 默认文件名：`{accountName}_{accountId}.solosoul`
  - 导出进度指示器
- 导入区：
  - "选择文件" 按钮 → `FilePicker.pickFiles`
  - 密码输入框（带 password_hint 显示）
  - "预览并导入" 按钮
  - 点击后弹出 `ImportPreviewDialog`

**Step 5.2**：`ImportPreviewDialog`（最复杂 UI）

设计：
```
┌─────────────────────────────────────────────────────────┐
│  导入预览 — {accountName} ({exportDate})                  │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  ☑ Profile    → 导入到：[Home ▼]                        │
│     └─ 4 个对象 (最高敏感度: 🔴 Restricted)               │
│                                                         │
│  ☑ Travel     → 导入到：[Travel ▼]                      │
│     └─ 12 个对象 (最高敏感度: 🔴 Restricted)              │
│                                                         │
│  ☐ Financial  → 导入到：[Financial ▼]                   │
│     └─ 8 个对象 (最高敏感度: 🟡 Private)                  │
│                                                         │
│  ☑ Professional → 导入到：[Work ▼]                      │
│     └─ 6 个对象 (最高敏感度: 🟢 Public)                   │
│                                                         │
├─────────────────────────────────────────────────────────┤
│  已选择：3 个分区，共 22 个对象，5 个附件                  │
│                              [取消]  [确认导入]           │
└─────────────────────────────────────────────────────────┘
```

每个分区的展开项显示：
- 分区下的具体对象列表（只读预览）
- 敏感度标签
- 附件数量
- **关系字段数量**：显示该分区包含的 RelationProperty 数量
- **跨分区关系警告**：若某分区包含指向未选中分区的 RelationProperty，显示黄色警告提示："该分区包含 N 个关系字段，导入后可能指向不存在对象"

**Step 5.3**：在 `DataManagementPage` 中添加入口

在现有 VaultInfoCard 下方添加新的 SectionCard：
```
┌─────────────────────────────┐
│  导入 / 导出                  │
│                             │
│  [导出全部数据]  [导入数据]    │
│                             │
│  将数据打包为可移植加密文件   │
│  或从其他设备导入数据        │
└─────────────────────────────┘
```

点击后导航到 `ExportImportPage` (`context.go(AppRoutes.exportImport)`)。

### Phase 6：路由与本地化（P1）

**Step 6.1**：`app_router.dart`：
```dart
static const String exportImport = '/settings/data-management/export-import';
// ...
GoRoute(
  path: AppRoutes.exportImport,
  builder: (context, state) => const ExportImportPage(),
),
```

**Step 6.2**：`app_zh.arb` / `app_en.arb` 新增键：
```json
{
  "exportImportTitle": "导入 / 导出",
  "exportButton": "导出全部数据",
  "importButton": "导入数据",
  "exportFileNameHint": "{accountName}_{accountId}.solosoul",
  "importPasswordHint": "输入导出时使用的密码",
  "importPreviewTitle": "导入预览",
  "importSectionPageLabel": "导入到页面",
  "importObjectsCount": "{count} 个对象",
  "importConfirm": "确认导入",
  "importSuccess": "导入成功",
  "importWrongPassword": "密码错误，请重试"
}
```

---

## 风险与缓解

| 风险 | 影响 | 缓解 |
|------|------|------|
| ZIP 跨平台兼容性 | Windows/Android 无系统 `zip` 命令 | 使用 Rust `zip` crate 统一处理，编译到全平台 |
| macOS Release 构建中 ZIP 操作耗时过长 | 用户认为应用卡死 | Rust `zip` crate 是原生性能，且支持流式写入大文件 |
| 大附件（84MB）导入/导出内存溢出 | OOM crash | 附件使用 `frbEncryptFile`/`frbDecryptFile` 流式处理，1MB 分块 |
| 附件跨账户导入无法解密 | 导入后附件打不开 | 导出时用 exportKey 重新加密，导入时先解密再用目标 Vault 重新加密 |
| ID 冲突（导入对象与现有对象 ID 重复） | 数据覆盖/损坏 | 导入时强制为所有对象生成新 UUID |
| customType ID 冲突 | 导入的对象 Schema 错乱 | 冲突时重生成 customType ID，并同步更新所有引用该 typeId 的对象 |
| RelationProperty 指向失效 | 关系字段显示异常 | ID 重映射阶段同步更新 RelationProperty.targetObjectId |
| 密码错误多次尝试 | 无安全风险（纯本地验证） | 使用独立 exportVerifyToken 验证，不暴露 Vault verify_hash |
| 导入操作无法撤销 | 数据意外丢失 | 导入前自动创建静默备份（BackupService.createBackup） |
| FRB 代码生成失败 | 编译错误 | 严格按照 flutter_rust_bridge 语法规范编写 |

---

## 测试策略

1. **单元测试**：
   - `ExportImportService.exportPackage` — 验证 ZIP 文件结构、checksums.json 正确
   - `ExportImportService.decryptAndVerify` — 验证密码正确/错误分支、exportVerifyToken 机制
   - ID 重映射逻辑 — 验证 parentId/childrenIds 正确更新
   - RelationProperty 映射 — 验证 targetObjectId 正确映射到新 ID
   - customType 合并 — 验证冲突时重生成 ID，无冲突时直接复用

2. **集成测试**：
   - 导出 → 修改当前数据 → 导入 → 验证合并结果
   - 跨账户导入：账户A导出 → 账户B导入 → 验证数据完整、附件可打开
   - 附件完整性：大附件（84MB）导出/导入后 SHA-256 比对

3. **手动测试**：
   - Release 构建下 84MB 附件的导出/导入性能
   - 不同账户间导入（不同 Vault 密码）的行为
   - 导入后撤销：删除导入数据，从静默备份恢复

---

## 预估工作量

| 阶段 | 工作量 | 说明 |
|------|--------|------|
| Phase 1：Rust ZIP 接口 | 0.5d | 新增 ZIP 打包/解压 FRB 函数（加密接口已存在） |
| Phase 2：导出服务 | 1d | ZIP 打包、附件重新加密、checksums |
| Phase 3：导入服务 | 1.5d | ZIP 解压、密码验证（exportVerifyToken）、预览模型 |
| Phase 4：导入执行 | 1.5d | ID 重映射、customType 合并、RelationProperty 映射、附件重新加密、自动备份 |
| Phase 5：UI 实现 | 2d | 导出/导入页面 + 预览对话框 + 进度指示 |
| Phase 6：路由+本地化 | 0.5d | |
| **总计** | **~7.0d** | |
