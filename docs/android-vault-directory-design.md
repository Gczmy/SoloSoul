# Android 端“用户自选 Vault 目录”技术方案

> 状态：设计阶段（Design），已选定方案 D）  
> 影响范围：Android 客户端、Rust 后端、Kotlin 原生插件、前端设置/引导流程  
> 关联文档：`AGENTS.md`、docs/design_map/*、docs/sync-roadmap.md

---

## 1. 背景与问题

### 1.1 现状

SoloSoul 桌面端把 Vault 数据放在 `~/.solosoul`（或 `dirs::data_dir()/com.solosoul.app`），卸载应用时不会自动删除。  
Android 端目前使用 Tauri 的 `BaseDirectory::Data`，对应 `/data/data/com.solosoul.app/files/`，这是应用私有目录，**卸载应用时会被 Android 系统自动清理**。

| 平台 | 数据目录 | 卸载后行为 |
|------|----------|------------|
| 桌面端 | `~/.solosoul` / `dirs::data_dir()/com.solosoul.app` | 保留 |
| Android | `BaseDirectory::Data` | 被系统删除 |

### 1.2 用户影响

- 与桌面端体验不一致。
- 用户误卸载或重装应用后，本地保险库数据全部丢失。
- 与 SoloSoul “本地优先、数据主权”的产品定位存在潜在冲突。

---

## 2. 设计目标

1. **持久化**：Android 用户把 Vault 数据存放到“卸载后仍然保留”的位置。  
2. **用户主权**：数据位置由用户可见、可选、可迁移。  
3. **与桌面端对齐**：长期看，Android 与桌面端都应“卸载不删数据”。  
4. **安全可控**：不申请宽泛存储权限，仅通过 SAF（Storage Access Framework）获取用户授权目录的读写权。  
5. **性能可控**：核心 SQLite 数据库的读写性能不因 SAF 而明显退化。  
6. **向后兼容**：因 Android 版本尚未发布，**不存在存量用户数据迁移成本**，可一次做对。

---

## 3. 方案对比与决策

| 维度 | 方案 A：保持现状 | 方案 B：仅 SAF 备份 | 方案 C：混合持久化 | **方案 D：文件系统级 SAF 适配层（推荐）** |
|------|---------------|------------------|----------------|------------------------------------------|
| **核心思路** | App-private 目录，数据卸载即删 | App-private 目录 + 用 SAF 导出 `.solosoul` 备份包 | SQLite/核心数据留在 App-private，附件/导出走 SAF | Rust 侧抽象 `VaultFileSystem`，所有 Vault 文件（含 SQLite）均可存放到用户选定的 SAF 目录 |
| **用户成本** | 低 | 中（需手动导出/恢复） | 中 | 低（首次启动选一次目录即可） |
| **数据安全** | 卸载丢失 | 备份加密安全 | 卸载后核心数据仍丢 | 卸载后数据保留在用户选定目录 |
| **产品一致性** | 差 | 中 | 中 | 高（与桌面端一致） |
| **实现复杂度** | 最低 | 低 | 中 | 中高 |
| **性能影响** | 无 | 小 | 中 | 中（需验证 SQLite over SAF） |
| **迁移成本** | 无 | 无 | 无 | 因未发布 Android，无历史数据迁移成本 |

### 3.1 决策结论

**选择方案 D：文件系统级 SAF 适配层。**

理由：
- Android 版本尚未发布，**没有存量用户数据需要迁移**，是引入此改动的最佳窗口期。
- 一旦发布后再改，存量用户将面临从 App-private 到 SAF 目录的迁移，复杂度与风险大幅上升。
- 方案 D 从架构上彻底解决了“卸载即删数据”的问题，符合 SoloSoul “本地优先、数据主权”的核心理念，且与桌面端长期对齐。
- 通过 `VaultFileSystem` 抽象层，桌面端与移动端共享统一接口，未来维护成本最低。

---

## 4. 方案 D 总体架构

```
┌────────────────────────────────────────────────────────────────────┐
│                           Frontend                                 │
│  SetupPage / SettingsPage → pickVaultDir → set_vault_dir           │
└─────────────────────────────────────┬──────────────────────────────┘
                                      │
                                      ▼
┌────────────────────────────────────────────────────────────────────┐
│                        Kotlin 插件层                                │
│  ACTION_OPEN_DOCUMENT_TREE → takePersistableUriPermission          │
│  返回 tree URI 给 Rust                                             │
└─────────────────────────────────────┬──────────────────────────────┘
                                      │
                                      ▼
┌────────────────────────────────────────────────────────────────────┐
│                        Rust 层                                      │
│  VaultFileSystem (trait)                                            │
│    ├── LocalVaultFileSystem   ← 桌面端、App-private 模式            │
│    └── SafVaultFileSystem     ← Android 用户选定目录                 │
│                                                                     │
│  VaultService::with_file_system(fs)                                 │
│    ├── SQLite DB、快照、附件、插件数据                              │
│    └── 所有文件读写通过 VaultFileSystem 接口                         │
└────────────────────────────────────────────────────────────────────┘
```

---

## 5. 详细设计

### 5.1 设计原则

- **用户授权最小化**：仅使用 `ACTION_OPEN_DOCUMENT_TREE`，不申请 `READ_EXTERNAL_STORAGE` / `WRITE_EXTERNAL_STORAGE`，更不申请 `MANAGE_EXTERNAL_STORAGE`。  
- **透明可逆**：首次启动引导用户选择目录；设置页可随时切换目录。  
- **文件系统级抽象**：Rust 所有 Vault 文件操作均通过 `VaultFileSystem` trait，不直接调用 `std::fs`。  
- **数据仍加密**：SQLite 数据库、快照、附件均已加密，SAF 目录只改变存储位置，不改变安全模型。

### 5.2 AndroidManifest.xml

无需新增危险权限。为了让 `resolveActivity` 预检通过，在 `<queries>` 中追加目录选择器意图（已在前序批量下载修复中加入）：

```xml
<intent>
    <action android:name="android.intent.action.OPEN_DOCUMENT_TREE" />
</intent>
```

**注意**：不要申请 `MANAGE_EXTERNAL_STORAGE`，Google Play 会拒绝非文件管理器类应用。

### 5.3 Kotlin 原生插件

建议复用 `AttachmentImportPlugin` 或在同一插件中新增命令，避免引入新插件。

#### 5.3.1 选择 Vault 目录

```kotlin
@Command
fun pickVaultDir(invoke: Invoke) {
    val intent = Intent(Intent.ACTION_OPEN_DOCUMENT_TREE)
    startActivityForResult(invoke, intent, "vaultDirPicked")
}

@ActivityCallback
fun vaultDirPicked(invoke: Invoke, result: ActivityResult) {
    if (result.resultCode == Activity.RESULT_OK) {
        val uri = result.data?.data ?: run {
            invoke.resolve(JSONObject())
            return
        }
        contentResolver.takePersistableUriPermission(
            uri,
            Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_WRITE_URI_PERMISSION
        )
        invoke.resolve(JSObject().apply { put("uri", uri.toString()) })
    } else {
        invoke.resolve(JSONObject()) // 用户取消
    }
}
```

#### 5.3.2 基于 SAF 的原始文件读写

SAF tree URI 下的文件操作主要有两种方式：

1. **通过 `DocumentsContract` 查找/创建子文档**，拿到子 URI 后用 `ContentResolver.openFileDescriptor(uri, mode)` 读写。  
2. 在已知子文件 URI 后，可直接 `openFileDescriptor` 交给 Rust 或 SQLite 使用。

为了降低复杂度并支持 SQLite 直接打开文件，推荐方案：

- Kotlin 侧提供辅助命令：`openFileDescriptorForPath(uri, relativePath, mode)` → 返回 ParcelFileDescriptor 的 fd 或一个临时本地路径。
- Rust 侧通过 `ParcelFileDescriptor` / `ContentResolver.openFileDescriptor` 拿到文件描述符后，用 `std::os::fd` 或 `sqlite3` 的 URI 文件名打开。

> 实践经验：SQLite 可以通过 `openFileDescriptor` 拿到 `fd`，然后用 `sqlite3_open_v2` 打开 `/proc/self/fd/{fd}`。但某些 ROM 对 `/proc/self/fd` 的随机访问有限制。  
> 更稳妥的做法：SAF 目录下文件变更不频繁时，采用**本地临时副本 + 写回 SAF**的策略；对 SQLite 这种高频随机读写文件，需要真机基准测试后决定最终策略。

### 5.4 Rust 后端改动

#### 5.4.1 新增 `VaultFileSystem` trait

路径：`tauri/src-tauri/src/fs/vault_file_system.rs`

```rust
use std::path::Path;

/// Vault 文件系统抽象层。
/// 桌面端与 App-private 模式使用本地文件系统；
/// Android 用户自定义目录使用 SAF-backed 文件系统。
pub trait VaultFileSystem: Send + Sync {
    /// 打开一个只读文件，返回内容。
    fn read_file(&self, relative_path: &str) -> Result<Vec<u8>, String>;
    /// 写入文件（覆盖）。
    fn write_file(&self, relative_path: &str, data: &[u8]) -> Result<(), String>;
    /// 删除文件。
    fn remove_file(&self, relative_path: &str) -> Result<(), String>;
    /// 判断是否存在。
    fn exists(&self, relative_path: &str) -> Result<bool, String>;
    /// 列出目录下所有条目（相对路径）。
    fn list_dir(&self, relative_path: &str) -> Result<Vec<String>, String>;
    /// 创建目录（若需要）。
    fn create_dir_all(&self, relative_path: &str) -> Result<(), String>;
    /// 获取可用于 SQLite 的本地/虚拟路径描述。
    fn sqlite_path(&self, relative_path: &str) -> Result<SqlitePath, String>;
}

/// 用于 SQLite 的路径形态。
pub enum SqlitePath {
    /// 可直接用 std::fs 访问的本地路径。
    Local(std::path::PathBuf),
    /// 需要通过 SAF 描述符临时映射的路径（含 fd 或 Content URI）。
    Saf { uri: String, fd: i32 },
}
```

实现：

- `LocalVaultFileSystem`：直接映射到本地 `std::path::Path`，复用现有逻辑。  
- `SafVaultFileSystem`：持有 `tree_uri`，通过 Kotlin `ContentResolver` / `DocumentsContract` 将相对路径映射为文件 URI，再调用 `openFileDescriptor`。

#### 5.4.2 修改 `VaultService` 初始化

`VaultService` 当前使用 `with_base_path(PathBuf)`。需要新增：

```rust
impl VaultService {
    pub fn with_file_system(fs: Arc<dyn VaultFileSystem>) -> Self { ... }
}
```

内部所有文件路径均基于该 `VaultFileSystem`。  
桌面端：`LocalVaultFileSystem` 使用 `dirs::data_dir()/com.solosoul.app`。  
Android：`AppState::new` 时读取用户偏好中的 `vault_dir_uri`，构建 `SafVaultFileSystem`。

#### 5.4.3 `AppState` 初始化流程

当前 `state/app_state.rs`：

```rust
let data_dir = handle
    .path()
    .resolve(".", tauri::path::BaseDirectory::Data)
    .map_err(...)?;
let svc = VaultService::with_base_path(data_dir);
```

改造后：

```rust
// 1. 读取 ui_preferences / user_data_preferences 中的 vault_dir_uri
let prefs = load_ui_preferences(&handle)?;
let vault_fs: Arc<dyn VaultFileSystem> = if cfg!(target_os = "android") {
    if let Some(uri) = prefs.vault_dir_uri {
        Arc::new(SafVaultFileSystem::new(uri)?)
    } else {
        // 首次启动默认使用 App-private，待引导后切换
        Arc::new(LocalVaultFileSystem::new(app_data_dir(&handle)?))
    }
} else {
    Arc::new(LocalVaultFileSystem::new(desktop_data_dir()))
};

let svc = VaultService::with_file_system(vault_fs);
```

### 5.5 前端流程

#### 5.5.1 首次启动引导

在欢迎页 / 创建账户前增加一步，且**不可跳过**（提供默认选项）：

```
选择保险库数据存放位置

○ 应用私有目录（不推荐）
   数据存储在应用内部，卸载 SoloSoul 时会被系统删除。

● 外部目录（推荐）
   数据保存在 Documents/SoloSoul 等外部目录，
   卸载后数据不会丢失，可随时迁移或备份。
   [选择目录...]
```

- 选择“外部目录” → 调用 `pickVaultDir` → 用户选择目录 → `set_vault_dir` → 创建账户。
- 选择“应用私有目录” → 默认使用 `BaseDirectory::Data`，后续可在设置中迁移。

#### 5.5.2 设置页入口

在“设置 → 数据与安全”中新增：

- **当前 Vault 目录**：显示当前类型（App-private / 外部目录）与路径/URI。
- **迁移到外部目录**：仅 App-private 模式下显示。
- **更改目录**：外部目录模式下可重新选择。
- **风险提示**：
  - App-private 模式：“卸载应用会删除本地保险库数据”。
  - 外部目录模式：“请妥善保管该目录，若手动删除文件将无法恢复”。

---

## 6. 关键问题：SQLite over SAF 的性能与可行性

### 6.1 技术可行性

SQLite 打开 SAF 文件有两种思路：

| 方案 | 描述 | 优点 | 缺点 |
|------|------|------|------|
| **fd 重定向** | 用 `ContentResolver.openFileDescriptor` 拿到 fd，再 `sqlite3_open_v2("/proc/self/fd/{fd}")` | 零拷贝、性能最好 | 部分 ROM 限制 `/proc/self/fd` 随机读写；关闭 fd 后路径失效 |
| **本地缓存 + 写回** | 启动/写操作时把 SQLite 复制到本地临时目录，操作完成后再写回 SAF | 兼容性好 | 写放大、并发控制复杂、意外关闭可能丢数据 |
| **WAL + 临时合并** | SQLite 本地运行，WAL 文件定期合并回 SAF | 性能好 | 实现复杂，需要额外同步逻辑 |

**推荐路径**：先用方案 A（fd 重定向）做真机基准测试，若主流 ROM 兼容性达标则采用；否则降级为方案 B（本地缓存 + 写回）。

### 6.2 性能基准

需要在真机上测试以下指标：

| 测试项 | 可接受阈值 | 备注 |
|--------|-----------|------|
| 解锁耗时 | < 500 ms（冷启动） | SQLite 首次连接 + 解密 |
| 对象列表查询 | < 200 ms（1000 条对象） | 索引命中情况 |
| 附件写入 10 MB | < 3 s | 对比 App-private 基线 |
| 随机搜索 1000 条 | < 300 ms | 含快照/审计日志查询 |
| 连续使用 30 分钟 | 无明显卡顿、无 ANR | 综合体验 |

---

## 7. 安全与隐私

- **权限最小化**：仅通过 SAF 获取用户选定目录的授权，不申请全存储权限。  
- **数据仍加密**：Vault 数据库、快照、附件均使用 AES-256-GCM 加密，即使放在外部目录也无法被其他应用读取。  
- **元数据可见**：外部目录中的文件名、目录结构、文件大小对用户可见，这是持久化的必要代价。  
- **授权持久化**：`takePersistableUriPermission` 在应用更新后仍然有效，但卸载后失效（Android 设计，无法避免；数据文件本身保留）。  
- **路径校验**：`SafVaultFileSystem` 拒绝任何企图访问 `../` 或根目录外的路径，防止路径穿越。

---

## 8. 实施路线图

### Phase 0：基础设施与可行性验证（1 周）

- 实现 `VaultFileSystem` trait、`LocalVaultFileSystem`。
- 实现 `SafVaultFileSystem` 基础读写（不含 SQLite）。
- 在 `VaultService` 中接入抽象层，确保桌面端无回归。
- 真机基准测试：SQLite over SAF（fd 重定向 vs 本地缓存）。
- 产出：《SAF 性能基准报告》。

### Phase 1：Android 默认外部目录（1 周）

- 在 Kotlin 插件中实现 `pickVaultDir` / `vaultDirPicked`。
- 新增 Rust commands：`get_vault_dir`、`set_vault_dir`。
- 修改 `AppState` 初始化，读取 `vault_dir_uri` 并构建 `SafVaultFileSystem`。
- 首次启动引导：默认推荐“外部目录”。
- 测试：创建账户、创建对象、附件读写、解锁/锁定循环。

### Phase 2：迁移与设置页（1 周）

- 设置页新增“Vault 目录”项。
- 实现 App-private ↔ 外部目录 的数据迁移。
- 授权失效检测与重新选择引导。
- 完整测试：迁移、授权撤销、卸载重装、路径切换。

### Phase 3：发布前验证（1 周）

- 多 ROM 真机回归。
- 性能基准复测。
- 文档更新与 i18n 补全。
- 发布 Android 版本。

---

## 9. 风险与应对

| 风险 | 影响 | 应对 |
|------|------|------|
| SQLite over SAF 性能不达标 | 高 | Phase 0 先做基准测试，不达标则采用本地缓存方案或推迟发布 |
| 用户选择目录后手动删除文件 | 高 | 设置页明确风险；提供一键导出/恢复 |
| 某些 ROM 对 DocumentsUI 支持差 | 中 | 预检 `resolveActivity`，失败时允许回退到 App-private |
| 授权失效（用户撤销） | 中 | 启动时检查授权状态，失效时弹出重新选择 |
| 数据迁移失败 | 中 | 迁移前校验，失败自动回滚，保留旧目录 |
| SAF 文件操作兼容性问题 | 中 | 在常见 ROM（小米/华为/三星/Pixel）上充分测试 |

---

## 10. 与桌面端对齐

选择方案 D 后，Android 与桌面端在“卸载不删数据”这一语义上完全对齐：

| 平台 | 数据目录 | 卸载后行为 |
|------|----------|------------|
| 桌面端 | `dirs::data_dir()/com.solosoul.app` | 保留 |
| Android | 用户通过 SAF 选择的外部目录 | 保留 |

唯一的差异是：Android 需要用户通过 SAF 授权目录，而桌面端默认使用固定目录。这是因为 Android 平台限制所致，符合用户预期。

---

## 11. 参考与延伸阅读

- Android 官方文档：Storage Access Framework  
  https://developer.android.com/guide/topics/providers/document-provider
- Obsidian 移动端数据目录实践  
  https://help.obsidian.md/Getting+started/Download+and+install+Obsidian#Mobile
- Tauri v2 Mobile Plugin 开发指南  
  https://v2.tauri.app/develop/mobile-plugins/

---

## 12. 附录：改动清单（实施时引用）

### Kotlin 层
- `tauri/src-tauri/gen/android/app/src/main/java/com/solosoul/app/AttachmentImportPlugin.kt`
  - 新增 `pickVaultDir` / `vaultDirPicked`
  - 新增辅助命令：`openFileDescriptorForPath` / `copyUriToLocal`（供 SQLite fd 模式使用）

### Rust 层
- 新增 `tauri/src-tauri/src/fs/vault_file_system.rs`
  - 定义 `VaultFileSystem` trait、`SqlitePath` enum
  - 实现 `LocalVaultFileSystem`
  - 实现 `SafVaultFileSystem`（Android only）
- `tauri/src-tauri/src/fs/mod.rs`
  - 导出新模块
- `tauri/src-tauri/src/state/app_state.rs`
  - 根据 `vault_dir_uri` 选择文件系统
- `tauri/src-tauri/src/commands/settings.rs`
  - 新增 `get_vault_dir`、`set_vault_dir`
- `tauri/src-tauri/src/lib.rs`
  - 注册新增命令
- `tauri/src-tauri/permissions/solo-soul/default.toml`
  - 将新增命令加入 ACL 允许列表
- `tauri/src-tauri/Cargo.toml`
  - 如需新增 JNI/SAF 辅助 crate 则在此声明

### 前端层
- `tauri/src/pages/setup/SetupPage.tsx`（或 onboarding 流程）
  - 新增“选择 Vault 目录”步骤
- `tauri/src/pages/settings/SettingsPage.tsx`
  - 新增“Vault 目录”设置项
- 新增/更新 i18n key
  - `settings:vault_dir_title`
  - `settings:vault_dir_app_private`
  - `settings:vault_dir_external`
  - `settings:vault_dir_select`
  - `settings:vault_dir_change`
  - `settings:vault_dir_migrate`
  - `settings:vault_dir_app_private_hint`
  - `settings:vault_dir_external_hint`

### 测试
- Rust 单元测试：`LocalVaultFileSystem` 基础操作
- Rust 集成测试：`SafVaultFileSystem` 路径映射与读写
- 前端测试：引导流程、设置页交互
- 真机回归：选择目录、创建账户、对象/附件读写、迁移、卸载重装、授权撤销
- 性能基准：SQLite over SAF 关键指标
