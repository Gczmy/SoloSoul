# Android 端“用户自选 Vault 目录”技术方案

> 状态：Phase 0-3 代码实现已完成（含 P1 自动同步与进度事件），待真机验证）  
> 影响范围：Android 客户端、Rust 后端、Kotlin 原生插件、前端设置/引导流程
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

路径：`tauri/crates/solosoul-core/src/vault_file_system.rs`

```rust
pub trait VaultFileSystem: Send + Sync {
    fn read_file(&self, relative_path: &str) -> Result<Vec<u8>, String>;
    fn write_file(&self, relative_path: &str, data: &[u8]) -> Result<(), String>;
    fn remove_file(&self, relative_path: &str) -> Result<(), String>;
    fn exists(&self, relative_path: &str) -> Result<bool, String>;
    fn create_dir_all(&self, relative_path: &str) -> Result<(), String>;
    fn remove_dir_all(&self, relative_path: &str) -> Result<(), String>;
    fn list_dir(&self, relative_path: &str) -> Result<Vec<String>, String>;
    fn local_path(&self, relative_path: &str) -> Option<PathBuf>;
    fn sync_to_remote(&self) -> Result<(), String> { Ok(()) }
    fn sync_from_remote(&self) -> Result<(), String> { Ok(()) }
    fn is_remote(&self) -> bool { false }
}
```

实现：

- `LocalVaultFileSystem`：直接映射到本地 `std::path::Path`，复用现有逻辑，是桌面端与 Android App-private 模式的默认实现。
- `SafVaultFileSystem`：把所有文件操作代理到 `local_temp_dir`，通过注入的 `SafSyncDriver` 在显式调用 `sync_to_remote` / `sync_from_remote` 时与 SAF 目录做全量同步。当前 Phase 1/2 采用**本地临时副本 + 手动全量同步**策略，SQLite 数据库始终读写本地临时目录，避免直接在 SAF `content://` URI 上打开数据库的兼容性风险。

#### 5.4.2 修改 `VaultService` 初始化

`VaultService` 新增 `with_file_system(base_path, fs)` 构造：

```rust
impl VaultService {
    pub fn with_file_system(base_path: PathBuf, fs: Arc<dyn VaultFileSystem>) -> Self { ... }
}
```

内部所有文件路径均基于该 `VaultFileSystem`。  
桌面端：`VaultService::new()` 默认使用 `LocalVaultFileSystem`，路径为 `dirs::data_dir()/com.solosoul.app`。  
Android：`AppState::new` 时读取 `app_config.json` 中的 `saf_tree_uri`；存在时构建 `SafVaultFileSystem`，否则使用 `LocalVaultFileSystem`。

#### 5.4.3 `AppState` 初始化流程（含等级 A 延迟初始化）

实际实现位于 `tauri/src-tauri/src/state/app_state.rs`。

**常规启动（已有配置或已有账户）**：

```rust
let svc = match Self::load_saved_saf_uri(&data_dir) {
    Some(uri) => {
        let temp_dir = data_dir.join("saf_vault_temp");
        let sync_driver = Arc::new(TauriSafSyncDriver::<tauri::Wry>::new(handle.clone()));
        let fs = Arc::new(SafVaultFileSystem::new(uri, temp_dir.clone(), sync_driver));
        // 首次同步延迟到 AppState::init_saf_sync() 异步执行（spawn_blocking），
        // 在 setup 完成后由后台 tokio 任务触发，不阻塞应用启动。
        Some(VaultService::with_file_system(temp_dir, fs))
    }
    None if accounts_exist_in_app_private(&data_dir) => {
        // 兼容老用户：app-private 下已有账户，自动按本地模式初始化
        let svc = VaultService::with_base_path(data_dir);
        svc.load_accounts();
        Some(svc)
    }
    None => None, // 等级 A：首次安装且无配置，延迟初始化
};
```

**首次启动（无配置、无账户）**：

- `vault_service` 初始为 `None`（或占位状态）。
- 不创建 `VaultService`，不打开 SQLite。
- `check_has_account`、`vault_list_accounts` 在未初始化时分别返回 `false` 和空数组。
- 用户通过 onboarding 选择 SAF 目录后，调用 `init_vault_directory(uri)`：
  1. 创建 `SafVaultFileSystem`
  2. 创建 `VaultService` 并 `load_accounts()`
  3. 保存 `app_config.json`
  4. 将 `vault_service` 置为可用

要点：
- `app_config.json` 保存在 Tauri 应用私有数据目录，与 Vault 数据目录解耦，方便在创建 `VaultService` 之前读取 SAF 配置。
- 当 `saf_tree_uri` 存在且本地临时目录尚未初始化时，才从 SAF 全量拉取数据；日常启动直接复用本地临时副本，保证性能。
- 所有 Vault 文件操作最终通过 `SafVaultFileSystem` 解析为本地临时路径，对 `VaultService` 透明。

### 5.5 前端流程

#### 5.5.1 首次启动引导（等级 A 方案）

路径：`tauri/src/components/onboarding/OnboardingDialog.tsx`

在欢迎页之后插入「选择保险库数据存放位置」步骤（仅 Android 显示）：

- **仅提供外部目录选项**：删除「应用私有目录」选项，首次启动即要求用户通过 SAF 选择持久化目录。这符合 Obsidian / Logseq 等本地优先应用的主流做法。
- 选择流程：调用 `pickVaultDirectory()` → 系统 SAF 目录选择器 → 用户选定 → `initVaultDirectory(uri)` → 后端创建 `VaultService` → 成功后直接进入下一步。
- **无需重启**：首次启动时 `AppState` 延迟初始化 `VaultService`，用户选择目录后才真正创建。
- 取消选择时：停留在当前步骤，提示必须选择目录才能继续。
- 实现细节：
  - 该步骤通过 `getPlatform()` 异步检测平台，非 Android 自动跳过。
  - 选择过程中暂停自动锁定（`autoLockPauseStore.pause()/resume()`）。
  - 后端新增 `init_vault_directory` 命令，仅在没有初始化时调用。

#### 5.5.2 设置页入口（已实现）

路径：`tauri/src/pages/settings/VaultDirectoryPage.tsx`

在“设置 → 数据管理 → 保险库目录”中提供：

- **当前 Vault 目录**：显示当前类型（App-private / 外部目录）与 SAF tree URI。
- **选择/更换目录**：仅 Android 显示，调用系统 SAF 目录选择器。
- **同步到 SAF / 从 SAF 同步**：手动全量同步按钮，供用户需要时强制同步。
- **恢复为本地目录**：删除 SAF 配置，下次启动回退到 App-private 目录。
- **风险提示**：切换目录后需要重启应用；将数据存放到外部目录后，卸载应用时数据不会丢失，但用户手动删除 SAF 目录中的文件将导致数据丢失。

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

### Phase 0：基础设施与可行性验证（已完成）

- ✅ 在 `tauri/crates/solosoul-core/src/vault_file_system.rs` 中定义 `VaultFileSystem` trait、`LocalVaultFileSystem`、`SafVaultFileSystem`、`SafSyncDriver` 及 no-op 占位。
- ✅ 实现基于本地临时目录 + 手动同步的 `SafVaultFileSystem`；SQLite 等高频随机读写仍落在本地临时目录，避免直接在 SAF `content://` URI 上打开数据库。
- ✅ 在 `VaultService` 中新增 `with_file_system(base_path, fs)`，接入抽象层，桌面端默认使用 `LocalVaultFileSystem` 无回归。
- ✅ `solosoul-core` 单元测试通过（`cargo test -p solosoul-core` 139 passed）。
- ⏸ 真机基准测试（SQLite over SAF fd 重定向 vs 本地缓存）：当前实现采用本地缓存方案，基准测试留到发布前真机回归阶段。

### Phase 1：Android SAF 目录与同步命令（已完成）

- ✅ Kotlin 插件 `AttachmentImportPlugin` 新增 `pickVaultDir` / `vaultDirResult`、`syncDirToRemote`、`syncDirFromRemote`。
- ✅ Rust Tauri 命令：`vault_get_directory`、`vault_set_directory`、`vault_sync_to_remote`、`vault_sync_from_remote`，以及桥接命令 `vault_pick_directory`。
- ✅ `AppState::new` 在 Android 启动时读取 `app_config.json` 中的 `saf_tree_uri`；存在时构建 `SafVaultFileSystem`，不存在时回退到本地 App-private 目录。
- ✅ 新增 `TauriSafSyncDriver` 调用 Kotlin 插件完成本地临时目录与 SAF tree 之间的全量同步。
- ✅ ACL 已补充相关命令权限。

### Phase 2：迁移与设置页（已完成）

- ✅ 设置页新增“保险库目录”入口（`VaultDirectoryPage.tsx`）。
- ✅ 支持在 App-private 目录与 SAF 用户目录之间切换；切换时自动把现有 Vault 数据复制到新的 SAF 临时目录并首次同步到远端。
- ✅ 切换目录后提示“需要重启应用”，使用 Tauri `relaunch()` 重启后生效。
- ✅ 提供手动“同步到 SAF / 从 SAF 同步”按钮，供用户需要时强制同步。
- ✅ i18n 中英双语 key 已补全。
- ✅ 授权失效检测与重新选择引导：`vault_get_directory` 命令新增 `valid` 字段，调用 Kotlin `checkVaultDirAccess` 查询 SAF URI 可访问性；VaultDirectoryPage 显示失效红色警告卡片（含重新选择按钮）；登录后通过 toast 提示用户。

### Phase 3：发布前验证（代码实现已完成）

- [x] **授权撤销场景**：`vault_check_directory` / `vault_get_directory.valid` 检测 SAF URI 有效性 + VaultDirectoryPage 红色警告 + 登录后 toast 通知。
- [x] **首次启动引导**：OnboardingDialog 已添加 Android 限定的「选择保险库数据存放位置」步骤，仅提供 SAF 外部目录选择；首次启动延迟初始化 `VaultService`，选择目录后直接继续，无需重启。
- [x] **文档更新与 i18n 补全**：设计文档已同步与实际代码一致；中英双语 key 完备（onboarding 11 个 + settings 20+ 个）。
- [x] **启动同步异步化**：首次 `sync_from_remote` 从构造函数移除，改为 `spawn_blocking` 延迟执行，不阻塞应用启动。
- [x] **增量同步**：Kotlin 双向同步增加 mtime+size 比较跳过未变更文件，减少 I/O。
- [x] **原子写入**：Kotlin syncDirToRemote / syncDirFromRemote / exportToTreeUri 全部改为先写 .tmp 文件、成功后重命名，防止中途失败丢数据。
- [x] **自动同步（dirty flag + 定期后台同步）**：`SafVaultFileSystem` 添加 `AtomicBool` 脏标记，`write_file`/`remove_file`/`remove_dir_all` 成功后设为脏；后台 task 每 30 秒通过 `sync_if_dirty()` 检查并自动同步到 SAF；`VaultFileSystem` trait 和 `VaultService` 均已暴露 `sync_if_dirty()`。
- [x] **同步与迁移进度事件**：`vault_sync_to_remote`/`vault_sync_from_remote` 命令 emit `sync-progress` 事件（开始/完成）；`vault_set_directory` 迁移阶段 emit 3 阶段进度事件（start/migrate/sync/complete）。

### 后续跟踪（发布前真机验证）

以下验证项因依赖真机环境，不作为代码实现任务跟踪，由发布负责人在发布前执行：

- [ ] 多 ROM 真机回归：Pixel / 小米 / 华为 / 三星等常见 ROM。
- [ ] 性能基准：对比 App-private 与 SAF 模式下的解锁、对象列表、附件写入、搜索耗时。
- [ ] 卸载重装测试：确认 SAF 模式下卸载后数据保留，重装后可正常读取。
- [ ] 发布 Android 版本。

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
  - 新增 `pickVaultDir` / `vaultDirResult`
  - 新增 `syncDirToRemote` / `syncDirFromRemote`
  - 复用既有 `AttachmentImportPlugin`，避免引入新插件

### Rust 层（core）
- `tauri/crates/solosoul-core/src/vault_file_system.rs`
  - 定义 `VaultFileSystem` trait
  - 实现 `LocalVaultFileSystem`
  - 实现 `SafVaultFileSystem`、`SafSyncDriver`、`NoOpSafSyncDriver`
- `tauri/crates/solosoul-core/src/lib.rs`
  - 导出相关类型
- `tauri/crates/solosoul-core/src/vault_service.rs`
  - 新增 `with_file_system(base_path, fs)` 构造

### Rust 层（Tauri app）
- `tauri/src-tauri/src/fs/vault_file_system.rs`
  - 从 `solosoul-core` re-export
- `tauri/src-tauri/src/fs/saf_sync_driver.rs`
  - 新增 `TauriSafSyncDriver`，桥接到 Kotlin 插件
- `tauri/src-tauri/src/fs.rs`
  - 导出新模块
- `tauri/src-tauri/src/state/app_state.rs`
  - 读取 `app_config.json` 中的 `saf_tree_uri` 并选择文件系统
- `tauri/src-tauri/src/commands/vault_directory.rs`
  - 新增 `vault_get_directory`、`vault_set_directory`、`vault_sync_to_remote`、`vault_sync_from_remote`
- `tauri/src-tauri/src/attachment_import_plugin.rs`
  - 新增 `pick_vault_directory`、`sync_dir_to_remote`、`sync_dir_from_remote` 插件命令封装
- `tauri/src-tauri/src/lib.rs`
  - 注册新增命令
- `tauri/src-tauri/permissions/solo-soul/default.toml`
  - 将 `vault_get_directory`、`vault_set_directory`、`vault_sync_to_remote`、`vault_sync_from_remote`、`vault_pick_directory` 加入 ACL 允许列表
- `tauri/src-tauri/Cargo.toml`
  - 无需新增 crate；复用现有 `solosoul-core`

### 前端层
- `tauri/src/pages/settings/VaultDirectoryPage.tsx`（新增）
  - 显示当前目录类型与 SAF URI
  - 提供选择/更换目录、同步到 SAF、从 SAF 同步、恢复本地目录按钮
  - 切换目录后提示重启
- `tauri/src/lib/vaultDirectory.ts`（新增）
  - 封装 `vault_get_directory`、`vault_set_directory`、`vault_pick_directory`、`vault_sync_to_remote`、`vault_sync_from_remote` 的 invoke 调用
- `tauri/src/pages/settings/SettingsPage.tsx`
  - 在“数据管理”分组中新增“保险库目录”入口（Android 限定）
- `tauri/src/App/routes.tsx`
  - 新增 `/settings/vault-directory` 路由
- i18n key（中英双语）
  - `settings:vault_directory`
  - `settings:vault_directory_desc`
  - `settings:vault_directory_current_type`
  - `settings:vault_directory_type_local`
  - `settings:vault_directory_type_saf`
  - `settings:vault_directory_saf_uri`
  - `settings:vault_directory_choose`
  - `settings:vault_directory_change`
  - `settings:vault_directory_reset_local`
  - `settings:vault_directory_sync_to_remote`
  - `settings:vault_directory_sync_from_remote`
  - `settings:vault_directory_sync_*_success` / `settings:vault_directory_sync_*_failed`
  - `settings:vault_directory_set_success` / `settings:vault_directory_set_failed`
  - `settings:vault_directory_reset_success` / `settings:vault_directory_reset_failed`
  - `settings:vault_directory_load_failed`
  - `settings:vault_directory_retry`
  - `settings:vault_directory_restart_required`
  - `settings:vault_directory_restart_required_desc`
  - `settings:vault_directory_explanation`
  - `settings:vault_directory_unavailable`
  - `settings:vault_directory_restart`
  - `settings:vault_directory_restart_failed`
  - `settings:items.vault_directory`
  - `settings:desc.vault_directory`

### 测试与验证
- `VaultService` 单元测试：20+ 个（`cargo test -p solosoul-core` 通过，总计 139 个）
- `LocalVaultFileSystem` 基础操作测试：3 个
- `SafVaultFileSystem` 单元测试：13 个（覆盖读写/路径校验/目录操作/同步委派/元数据/脏标记）
- 桌面端回归：`cargo check`（0 errors, 0 warnings）/ `cargo test -p solosoul-core`（139/139）/ `npx tsc --noEmit` 全部通过
- 前端静态检查：`npx tsc --noEmit` / ESLint 通过
- 待完成：多 ROM 真机回归、性能基准、卸载重装测试

---

## 13. 等级 A 实施方案：首次启动延迟初始化

### 13.1 目标

在保持「后续切换目录仍需重启」的前提下，消除首次启动后的重启步骤，提升首次使用体验。

### 13.2 核心设计

- **启动时不创建 `VaultService`**：首次安装且无配置时，`AppState.vault_service` 为 `None`。
- **Onboarding 强制选择 SAF 目录**：删除「内部/外部」二选一，只提供 SAF 选择。
- **选择后即时初始化**：通过新命令 `init_vault_directory` 在 Rust 端创建 `VaultService`。
- **后续目录切换**：仍通过 `vault_set_directory` + 重启实现。

### 13.3 后端设计

#### 13.3.1 `AppState` 字段与初始化

```rust
pub struct AppState {
    pub handle: tauri::AppHandle,
    pub vault_service: Arc<RwLock<Option<VaultService>>>, // 改为 Option
    pub sync_service: Arc<SyncService>,
    pub plugin_manager: Arc<PluginManager>,
}
```

`AppState::new()` 启动时：

1. 读取 `app_config.json`。
2. 若存在 SAF URI → 按 SAF 初始化。
3. 若不存在配置但 `app-private` 目录下已有账户 → 按 App-private 初始化（兼容老用户）。
4. 否则 → `vault_service = None`，等待 onboarding 初始化。

#### 13.3.2 新增命令 `init_vault_directory`

- 参数：`{ saf_tree_uri?: string }`
- 行为：
  1. 检查 `vault_service` 是否已初始化，已初始化则报错。
  2. 根据 `saf_tree_uri` 创建 `SafVaultFileSystem`（或 App-private）。
  3. 创建 `VaultService` 并 `load_accounts()`。
  4. 保存 `app_config.json`。
  5. 将 `vault_service` 置为可用。
  6. 触发后台 `init_saf_sync()`。

#### 13.3.3 命令守卫

以下命令在 `vault_service` 为 `None` 时的行为：

| 命令 | 未初始化行为 |
|---|---|
| `check_has_account` | 返回 `false` |
| `vault_list_accounts` | 返回空数组 |
| `bootstrap` / `login` / `unlock` / 对象相关命令 | 返回错误「保险库尚未初始化」 |

### 13.4 前端设计

#### 13.4.1 `OnboardingDialog`

- 删除「应用私有目录」选项按钮。
- 用户点击「选择外部目录」→ `pickVaultDirectory()` → `initVaultDirectory(uri)`。
- 成功 → 自动进入下一步。
- 失败 → 显示错误，允许重试。
- 取消 → 停留在当前步骤。

#### 13.4.2 `vaultDirectory.ts`

新增：

```ts
export async function initVaultDirectory(
  safTreeUri: string,
): Promise<InitVaultDirectoryResult> {
  return invoke<InitVaultDirectoryResult>('init_vault_directory', {
    payload: { safTreeUri },
  });
}
```

### 13.5 边界情况

| 场景 | 处理 |
|---|---|
| 用户取消 SAF picker | 停留在当前步骤，提示必须选择 |
| SAF 初始化失败 | 显示错误，允许重试 |
| App 在 onboarding 中被杀 | 下次启动仍显示 onboarding |
| 已有 app-private 账户 | 自动初始化，不显示 onboarding |
| 后续切换目录 | 仍用 `vault_set_directory` + 重启 |

### 13.6 文件改动清单

- `tauri/src-tauri/src/state/app_state.rs`：`vault_service` 改为 `Option`，新增延迟初始化逻辑。
- `tauri/src-tauri/src/commands/vault_directory.rs`：新增 `init_vault_directory`。
- `tauri/src-tauri/src/commands/*.rs`：加守卫。
- `tauri/src-tauri/src/lib.rs`：注册命令。
- `tauri/src-tauri/permissions/solo-soul/default.toml`：加 ACL。
- `tauri/src/lib/vaultDirectory.ts`：新增 `initVaultDirectory`。
- `tauri/src/components/onboarding/OnboardingDialog.tsx`：删除内部选项，用新命令。

### 13.7 验证清单

- [ ] `cargo check` 通过。
- [ ] `cargo test -p solosoul-core` 通过。
- [ ] `npx tsc --noEmit` 通过。
- [ ] `npx eslint` 通过。
- [ ] 真机：首次安装 → 显示 SAF 目录选择 → 选完后不重启进入引导。
- [ ] 真机：onboarding 完成后可正常创建账户。
- [ ] 真机：设置页切换目录后仍提示重启，重启后生效。
- [ ] 真机：已有账户用户不显示 onboarding。
- [ ] 真机：取消 SAF picker 后仍可重试。
