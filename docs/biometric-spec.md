# SoloSoul 生物识别（Touch ID / Face ID / Windows Hello）实现规范

> 本文档记录当前 macOS 生物识别的实现细节、已踩过的坑，以及后续实现 Windows Hello 时必须遵循的约束与建议。

## 1. 当前状态

| 平台 | 状态 | 实现位置 |
|------|------|---------|
| **macOS** | ✅ 已支持 Touch ID / Face ID / 设备密码（本地加密文件 + LocalAuthentication 弹窗） | `tauri/crates/solosoul-core/src/biometric/macos.rs` |
| **macOS (未来)** | ⏳ Keychain UserPresence 方案已保留 | `tauri/crates/solosoul-core/src/biometric/macos_keychain.rs` |
| **Windows** | ❌ 未实现 | 待后续按本规范扩展 |
| **Linux** | ❌ 未实现 | 暂不支持 |

核心原则：

- 生物识别只用于**替代输入主密码**这一过程，不改变 Vault 的加密模型。
- Vault 数据密钥仍由主密码派生；生物识别凭证里保存的是该派生密钥。
- 生物识别操作必须在 Vault 已解锁或能通过当前主密码验证的上下文中进行。
- **macOS 当前使用本地加密文件**：由于团队暂未加入 Apple Developer Program，无法获得 `keychain-access-groups` entitlement，因此当前方案不使用 Keychain，而是在读取/保存前调用 `LocalAuthentication` 弹窗验证用户身份。
- **打开应用时不弹框**：`is_configured` 只检查本地 `biometric_key` 文件是否存在，不会触发系统验证框。

---

## 2. 已知的 macOS 坑与修复方案

### 2.1 当前 macOS 方案：本地加密文件 + LocalAuthentication 弹窗

**背景**：

- 团队暂未付费加入 Apple Developer Program，无法获得有效的 `keychain-access-groups` entitlement。
- 直接使用 Keychain `kSecAccessControlUserPresence` 在 ad-hoc 签名构建中会返回 `-34018`（"A required entitlement isn't present"），导致生物识别无法启用。

**当前 macOS 方案**：

- 主密钥保存在应用数据目录的本地加密文件 `biometric_key` 中（复用 `FileBiometricStorage`）。
- 文件密钥使用 HKDF 从 `account_id + 应用级 secret` 派生，不依赖 Keychain 也能解密。
- **保存凭证前**由 `BiometricManager::save_credential` 调用 `LocalAuthentication`，弹出 Touch ID / 设备密码框验证用户身份。
- **读取凭证前**由 `BiometricManager::unlock` 调用 `LocalAuthentication`，验证通过后才读取文件并解锁 Vault。
- `is_configured` 只检查 `biometricEnabled` 标记和 `biometric_key` 文件是否存在，**不会弹出任何系统验证框**。
- 旧版 `biometric_key` 文件无需迁移；当前 macOS 后端直接把它作为主存储。
- 若关闭生物识别，会清理 `biometricEnabled` 标记并删除 `biometric_key` 文件。

### 2.2 未来 macOS 方案：Keychain UserPresence（已保留实现）

- 完整实现保留在 `tauri/crates/solosoul-core/src/biometric/macos_keychain.rs`。
- 启用条件：
  1. 加入 Apple Developer Program（99 USD/年）并获取 Team ID。
  2. 使用有效的 `Developer ID Application` 证书对 App 签名。
  3. 在 `src-tauri/entitlements.plist` 中声明 `keychain-access-groups`（格式 `<TeamID>.<BundleID>`）。
  4. 在 `tauri.conf.json` 的 `bundle.macOS.entitlements` 中引用该 plist。
- 切换方式：将 `biometric/mod.rs` 中 `platform_storage()` 的 macOS 分支从 `MacOsBiometricStorage::new(base_path)` 改为 `macos_keychain::MacOsKeychainStorage::new(base_path)`。
- **请勿删除 `macos_keychain.rs`**，它是未来加入 Apple Developer Program 后的官方方案。

### 2.3 修改主密码后同步更新生物识别凭证

**现象**：改密会重新生成 Vault 数据密钥（新 salt + 新派生）。旧的生物识别密钥还是从「旧密码 + 旧 salt」派生的，用旧密钥去解新 Vault 会报 `Decryption failed: aead::Error`。

**修复**：

- `VaultService::change_password` 在 re-encrypt 并重新打开 Vault 后，调用 `BiometricManager::update_credential(account_id, new_key_hex)`。
- 仅当用户之前已启用生物识别时才更新；未启用则不操作。
- 这样改密后 Touch ID / Face ID 仍然可用，无需用户手动重新启用。

### 2.4 保存后回读校验不可靠

**现象**：保存生物识别凭证后，尝试回读以做一致性校验。回读可能因权限或锁定失败，导致误报凭证失效。

**修复**：

- 改为**保存前派生校验**：用当前主密码派生 Vault 数据密钥，与 `VaultService::get_session_key()` 比对。
- 只有派生出的密钥和当前 Vault 会话密钥一致，才触发系统生物识别对话框并保存凭证。

### 2.5 错误信息需要国际化，不能直接把后端异常抛给 UI

**现象**：后端返回 `Decryption failed: aead::Error (hint: ...)` 这类长英文，UI 截断显示，用户看不懂。

**修复**：

- 后端统一返回 `__BIO_ERR__:<code>` 格式。
- 前端 `src/lib/biometricError.ts` 解析 code，查找 `settings:biometric_error_<code>` 做多语言展示。

---

## 3. 后续实现 Windows Hello 的规范

如果要在 Windows 上实现 Windows Hello 支持，**必须避免 macOS 上踩过的坑**，并遵循以下规范。

### 3.1 凭证存储方案

**推荐：本地加密文件 + 可选 DPAPI-NG（不要只放 Credential Manager）**

- 参考 macOS 实现，把主密钥保存在应用数据目录的加密文件中。
- 文件密钥使用 HKDF 从 `account_id + 应用级 secret` 派生，确保不依赖 Credential Manager 也能解密。
- 可以额外用 DPAPI-NG 对文件再做一层保护，但**必须保留不依赖 DPAPI-NG 的解密路径**，避免域策略或用户拒绝时生物识别彻底失效。
- 不推荐单独使用 Windows Credential Manager，因为它可能导致与 macOS Keychain 类似的弹窗和锁定问题。

### 3.2 必须遵守的约束

| 约束 | 原因 | 实现建议 |
|------|------|---------|
| **不依赖系统钥匙串/凭证管理器作为唯一存储** | 避免打开应用弹窗和设备锁定后失效 | 主密钥存放在本地加密文件，系统凭证管理器只作可选增强 |
| **备份文件必须使用确定性文件密钥** | 避免 Credential Manager / DPAPI 不可用时，备份文件也报废 | 复用 macOS 的 HKDF 方案：`info = b"solosoul:biometric:filekey:v1"`，salt = `account_id` |
| **修改密码后更新 Windows 凭证** | 旧凭证中的密钥无法解密新 Vault | 在 `VaultService::change_password` 中用新密钥调用 Windows 更新逻辑；仅当已启用时更新 |
| **保存前做派生校验** | 不要依赖保存后再回读系统存储 | 用当前密码派生密钥，与 `get_session_key()` 比较一致后再保存 |
| **错误返回 `__BIO_ERR__:<code>`** | 保持前端国际化逻辑一致 | 新增 Windows 专用 code：`windows_hello_unavailable`、`windows_auth_failed` 等 |
| **不直接暴露后端异常原文** | 避免 UI 截断和用户困惑 | 所有用户可见错误走 `biometricError.ts` 映射 |
| **打开应用时不应弹出系统对话框** | 用户体验 | 任何探测 `is_configured` 的操作都只检查文件存在性，不访问 Credential Manager |

### 3.3 建议的平台抽象

当前 `BiometricManager` 通过 `BiometricStorage` trait 注入平台后端：

```rust
pub(crate) trait BiometricStorage: Send + Sync {
    fn save(&self, account_id: &str, key_hex: &str, reason: &str) -> Result<(), BiometricError>;
    fn update(&self, account_id: &str, key_hex: &str) -> Result<(), BiometricError>;
    fn read(&self, account_id: &str, reason: &str) -> Result<String, BiometricError>;
    fn delete(&self, account_id: &str) -> Result<(), BiometricError>;
    fn exists(&self, account_id: &str) -> bool;
}
```

实现 Windows Hello 时：

- 新增 `biometric/windows.rs`，实现 `BiometricStorage`。
- 公共接口保持统一：`save_credential`、`unlock`、`delete_credential`、`availability`。
- `BiometricManager` 在 `new()` 中按平台选择后端；测试通过 `with_storage()` 注入 mock。

### 3.4 Windows 专用错误代码建议

在 `src-tauri/src/commands/biometric.rs` 的 `map_bio_error` 中增加：

- `windows_hello_unavailable`：设备不支持 Windows Hello 或未配置。
- `windows_auth_failed`：用户取消或生物识别验证失败。
- `windows_credential_store_failed`：Credential Manager 写入/读取失败。
- `windows_dpapi_failed`：DPAPI 加解密失败。

并在 `src/locales/zh-CN/settings.json` 和 `src/locales/en-US/settings.json` 中补充对应的 `biometric_error_*` 文案。

### 3.5 测试场景

Windows Hello 实现后，至少覆盖以下场景：

1. 密码登录 → 启用 Windows Hello → 锁定 → Windows Hello 解锁成功。
2. 启用 Windows Hello → 修改主密码 → Hello 凭证应自动更新为新密钥 → 仍能用 Hello 解锁。
3. 启用 Windows Hello → 禁用 Hello → 再启用 → 新凭证应能正常解锁。
4. 模拟 Credential Manager 不可用（如用测试桩）→ 应能回退到确定性文件密钥解密。
5. 取消 Windows Hello 验证 → 只显示密码输入框，不显示错误文案。

---

## 4. 相关文件速查

| 文件 | 说明 |
|------|------|
| `tauri/crates/solosoul-core/src/biometric/mod.rs` | 生物识别核心接口与 `BiometricManager` |
| `tauri/crates/solosoul-core/src/biometric/macos.rs` | macOS Keychain UserPresence 存储实现 |
| `tauri/crates/solosoul-core/src/biometric/stub.rs` | 非 macOS 占位实现 |
| `tauri/crates/solosoul-core/src/biometric/legacy.rs` | 旧版文件存储（仅测试/清理） |
| `tauri/crates/solosoul-core/src/vault_service.rs` | 改密时同步更新生物识别凭证 |
| `tauri/src-tauri/src/commands/biometric.rs` | Tauri 命令层与错误码映射 |
| `tauri/src/lib/biometricError.ts` | 前端错误码国际化解析 |
| `tauri/src/pages/auth/LoginPage.tsx` | 登录页生物识别解锁 |
| `tauri/src/pages/settings/SecuritySettingsPage.tsx` | 设置页启用/禁用生物识别 |
| `tauri/src/locales/zh-CN/settings.json` / `en-US/settings.json` | 生物识别错误文案 |

---

## 5. 版本历史

| 日期 | 变更 |
|------|------|
| 2026-06-16 | 根据 macOS Touch ID 修复经验，整理形成本规范，明确 Windows Hello 实现约束 |
| 2026-06-16 | macOS 生物识别存储升级为 Keychain `kSecAccessControlUserPresence`；新增 `BiometricStorage` trait 与错误码 |
