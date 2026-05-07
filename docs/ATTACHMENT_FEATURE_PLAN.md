# 附件（图片加密保存）功能实施计划

## 需求概述

在 Profile/Travel 等页面的「Scan Document」流程中，用户选择照片并完成 OCR 后：
1. 在确认页面增加一个 checkbox：**"Save this image to Vault"**
2. 勾选后，原始照片经 Vault 加密后保存到本地文件系统
3. 在 Passport / ID Card 等对象的卡片上增加「附件列表」按钮
4. 点击后可查看附件列表，再点击附件可预览图片

---

## 现状调研结论

| 维度 | 现状 |
|------|------|
| Vault 文件存储 | ❌ 无。Rust Vault 只存加密 JSON blob，无附件/文件概念 |
| `PropertyType` | 8 种纯文本/值类型，无 `file`/`image` |
| MRZ 扫描流程 | `ScanDocumentButton` → `MrzScannerSheet` → `MrzVaultService.saveMrzToVault()`，**图片 bytes 在 OCR 后即丢弃** |
| `ScanPreviewPage` | 属于「Local Search Import」流程，**与 MRZ 扫描无关** |
| `UnifiedObject` | 使用 `json_serializable`，`properties` 是 `Map<String, PropertyValue>` |
| 加密 API | `RustVaultService.encryptBytes()` / `decryptBytes()` 可用，基于 AES-256-GCM |
| UUID / path_provider | 项目中已有 `uuid: ^4.4.0` 和 `path_provider: ^2.1.2` |

---

## 方案选型

由于 Vault 没有原生的 blob/attachment 表，本方案采用 **"Flutter 层管理加密文件 + 对象元数据引用"** 的混合架构：

- **文件内容**：用 `RustVaultService.encryptBytes()` 加密后，存到应用文档目录 `solosoul_storage/attachments/{accountId}/{fileId}.solo`
- **元数据引用**：在 `UnifiedObject` 上新增 `attachments: List<Attachment>` 字段，记录文件名、fileId、MIME 类型、创建时间
- **优点**：不修改 Rust FFI / Go 后端，最小侵入；支持大文件（不需要全塞进 SQLite JSON blob）
- **缺点**：附件与 Vault 数据库非原子操作（文件写成功后若对象保存失败，会产生孤儿文件；可接受）

---

## 实施步骤

### Step 1：数据模型扩展

**文件**：`flutter/lib/core/models/unified_object_model.dart`

1. 新增 `Attachment` 类（`@JsonSerializable`）：
   - `id`: String — 附件记录 ID
   - `fileId`: String — 用于查找加密文件的 UUID
   - `fileName`: String — 原始文件名（如 `passport_scan.jpg`）
   - `mimeType`: String — 如 `image/jpeg`
   - `createdAt`: int — 毫秒时间戳

2. `UnifiedObject` 新增字段：
   - `attachments: List<Attachment>`（默认 `const []`）
   - 更新 `copyWith`

3. 运行 `dart run build_runner build --delete-conflicting-outputs` 重新生成 `.g.dart`

---

### Step 2：附件存储服务

**新建文件**：`flutter/lib/core/services/attachment_storage_service.dart`

提供三个核心方法：

```dart
Future<Attachment> saveAttachment({
  required String accountId,
  required String fileName,
  required Uint8List bytes,
});

Future<Uint8List?> loadAttachment({
  required String accountId,
  required String fileId,
});

Future<bool> deleteAttachment({
  required String accountId,
  required String fileId,
});
```

- 目录结构：`{appDocuments}/solosoul_storage/attachments/{accountId}/`
- 文件命名：`{fileId}.solo`（加密后的 SOLO blob）
- MIME 类型从文件名后缀推断

---

### Step 3：MRZ 扫描结果承载图片数据

**文件**：`flutter/lib/core/models/ocr_result.dart`

新增 `MrzScanResult` 类：

```dart
class MrzScanResult {
  final MrzData mrzData;
  final Uint8List? imageBytes;   // 原始照片 bytes
  final bool saveImage;          // 用户是否勾选保存
}
```

---

### Step 4：扫描 Sheet 增加保存选项

**文件**：`flutter/lib/presentation/widgets/mrz_scanner_sheet.dart`

1. `_MrzScannerSheetState` 新增状态：
   - `Uint8List? _imageBytes`
   - `bool _saveImage = false`

2. `_pickImage()` 中保留 bytes（当前 OCR 后即丢弃）：
   ```dart
   final bytes = await picked.readAsBytes();
   _imageBytes = bytes;  // 新增保留
   ```

3. `_buildResultState()` 在 Confirm 按钮上方增加：
   ```dart
   CheckboxListTile(
     title: const Text('Save this image to Vault'),
     subtitle: const Text('Encrypted and stored locally'),
     value: _saveImage,
     onChanged: (v) => setState(() => _saveImage = v ?? false),
   )
   ```

4. `Navigator.pop()` 改为返回 `MrzScanResult`：
   ```dart
   Navigator.of(context).pop(MrzScanResult(
     mrzData: _mrzResult!,
     imageBytes: _saveImage ? _imageBytes : null,
     saveImage: _saveImage,
   ));
   ```

---

### Step 5：扫描按钮适配新返回类型

**文件**：`flutter/lib/presentation/widgets/scan_document_button.dart`

1. `showModalBottomSheet` 返回类型改为 `MrzScanResult?`
2. 提取 `result.mrzData`、`result.imageBytes`、`result.saveImage`
3. 传给 `MrzVaultService.saveMrzToVault()` 新增参数

---

### Step 6：MRZ Vault Service 保存附件

**文件**：`flutter/lib/core/services/mrz_vault_service.dart`

1. `saveMrzToVault` 签名扩展：
   ```dart
   static Future<({bool success, String message})> saveMrzToVault(
     WidgetRef ref, {
     required MrzData mrzData,
     Uint8List? imageBytes,
     bool saveImage = false,
   })
   ```

2. `_createPassport` / `_createIdCard` 内部：
   - 先按现有逻辑 `createObject(...)` 创建对象
   - 成功后，若 `saveImage && imageBytes != null`：
     - 获取 `accountId = ref.read(authNotifierProvider.notifier).selectedAccountId`
     - 调用 `AttachmentStorageService.saveAttachment(accountId: ..., fileName: 'passport_scan.jpg', bytes: imageBytes)`
     - 获取刚创建的对象 ID（当前 `createObject` 返回 `bool`，需要找到对象）
     - 调用 `notifier.updateObject(objectId, attachments: [attachment])`

   > **待决策**：`createObject` 返回 `bool` 不返回 ID。可通过 `state.objects.last` 或 `state.objects.firstWhere((o) => o.name == name && o.typeId == typeId)` 获取新对象。更安全的做法是在 `UnifiedObjectNotifier` 中新增 `createObjectWithResult` 返回 `(bool, String? objectId)`，但为最小改动，建议先通过 `name + typeId + parentId` 匹配。

---

### Step 7：卡片上显示附件按钮

**文件**：`flutter/lib/presentation/widgets/entry_card_widget.dart`

在 `build()` 或 `_buildActions()` 中：

1. 检查 `widget.item is UnifiedObject`
2. 若 `(widget.item as UnifiedObject).attachments.isNotEmpty`
3. 在 actions 行增加附件图标按钮（`Icons.attach_file` 或 `Icons.image`），显示附件数量徽标
4. 点击打开附件列表 BottomSheet

**文件**：`flutter/lib/presentation/widgets/object_card/object_card_item_tile.dart`

同理，在 `ObjectCardItemTile` 的 action row 中也增加附件按钮（用于非 `EntryCardWidget` 场景，如 Financial / Professional 页面）。

---

### Step 8：附件列表与预览 UI

**新建文件**：`flutter/lib/presentation/widgets/attachment_list_sheet.dart`

实现 `AttachmentListSheet`：

- 接收 `List<Attachment> attachments`、`String accountId`
- 显示附件名称列表（带图标：`Icons.image` / `Icons.insert_drive_file`）
- 点击某项：
  - 调用 `AttachmentStorageService.loadAttachment()` 解密
  - 若 `mimeType.startsWith('image/')`，使用 `Image.memory(decryptedBytes)` 全屏/对话框预览
  - 若其他类型，显示文件信息（文件名、大小、类型）+ "无法预览此文件类型"

**预览对话框**：可用 `Dialog` 或 `showModalBottomSheet`，图片支持 pinch-to-zoom（可选，MVP 可先用 `InteractiveViewer`）。

---

### Step 9：对象删除时级联清理附件

**文件**：`flutter/lib/presentation/providers/unified_object_provider.dart`

在 `deleteObject` / `deleteDefaultItem` 中：
- 获取被删除对象的 `attachments`
- 遍历调用 `AttachmentStorageService.deleteAttachment()`
- 再执行现有删除逻辑

---

## 文件变更清单

| 文件 | 操作 | 说明 |
|------|------|------|
| `flutter/lib/core/models/unified_object_model.dart` | 修改 | 新增 `Attachment` 类，`UnifiedObject` 加 `attachments` |
| `flutter/lib/core/models/unified_object_model.g.dart` | 自动生成 | 运行 build_runner |
| `flutter/lib/core/models/ocr_result.dart` | 修改 | 新增 `MrzScanResult` 类 |
| `flutter/lib/core/services/attachment_storage_service.dart` | 新建 | 加密文件存取服务 |
| `flutter/lib/presentation/widgets/mrz_scanner_sheet.dart` | 修改 | 保留图片 bytes，加 checkbox，改返回类型 |
| `flutter/lib/presentation/widgets/scan_document_button.dart` | 修改 | 适配 `MrzScanResult` |
| `flutter/lib/core/services/mrz_vault_service.dart` | 修改 | 创建对象后保存附件 |
| `flutter/lib/presentation/widgets/entry_card_widget.dart` | 修改 | 附件按钮 + 列表入口 |
| `flutter/lib/presentation/widgets/object_card/object_card_item_tile.dart` | 修改 | 附件按钮 + 列表入口 |
| `flutter/lib/presentation/widgets/attachment_list_sheet.dart` | 新建 | 附件列表 BottomSheet + 图片预览 |
| `flutter/lib/presentation/providers/unified_object_provider.dart` | 修改 | 删除对象时清理附件文件 |

---

## 待确认问题

1. **对象创建后获取 ID 的方式**：当前 `createObject` 只返回 `bool`。计划通过 `name + typeId + parentId` 在刚写入的 `state.objects` 中反向查找。是否接受？还是更希望在 `UnifiedObjectNotifier` 中新增返回 ID 的 API？

2. **图片预览交互**：MVP 先用 `Dialog` + `InteractiveViewer` 实现基础缩放。是否需要更复杂的图片查看器（如左右滑动、全屏）？

3. **附件命名**：扫描的图片统一命名为 `passport_scan.jpg` / `id_card_scan.jpg`，还是保留用户原文件名（相册选择的图片通常有原始文件名，相机拍摄的可能没有）？

4. **删除对象的附件清理**：对象软删除（`isDeleted = true`）时是否也删除附件文件？建议仅在永久删除（从 Trash 清空或硬删除）时清理，软删除保留附件以防恢复。
