# SoloSoul 生物识别（Touch ID / Face ID / Windows Hello）实现规范

> 本文档记录当前 macOS 生物识别的实现细节、已踩过的坑，以及后续实现 Windows Hello 时必须遵循的约束与建议。

## 1. 当前状态

| 平台 | 状态 | 实现位置 |
|------|------|---------|
| **macOS** | ✅ 已支持 Touch ID / Face ID | `tauri/crates/solosoul-core/src/biometric.rs` |
| **Windows** | ❌ 未实现 | 待后续按本规范扩展 |
| **Linux** | ❌ 未实现 | 暂不支持 |

核心原则：

- 生物识别只用于**替代输入主密码**这一过程，不改变 Vault 的加密模型。
- Vault 数据密钥仍由主密码派生；生物识别凭证里保存的是该派生密钥。
- 生物识别操作必须在 Vault 已解锁或能通过当前主密码验证的上下文中进行。

---

## 2. 已知的 macOS 坑与修复方案

### 2.1 `keyring` 默认 `security` CLI 可能“假写入”

**现象**：调用 `keyring::Entry::set_password` 返回 `Ok(())`，但实际未写入 macOS 钥匙串。再次读取时失败，导致后续解锁走备份文件。

**修复**：

- `crates/solosoul-core/Cargo.toml` 中启用 `apple-native` feature：

  ```toml
  keyring = { version = "3", features = ["apple-native"] }
  ```

- 直接使用 Security framework 而非 `security` 子进程，写入更可靠。

### 2.2 备份文件不能依赖同一级密钥存储

**现象**：macOS 设备锁定后，钥匙串可能处于锁定状态。主密钥存在钥匙串里读不出，回退到本地备份文件；但备份文件的加密密钥也放在钥匙串里，于是备份文件也解不开，最终报错「生物识别凭证已失效」。

**修复**：

- 备份文件改用**确定性 HKDF 文件密钥**：
  - `info = b"solosoul:biometric:filekey:v1"`
  - 以 `account_id` 作为 salt
  - 从一个固定应用级 secret 派生
- 这样即使钥匙串完全不可用，只要应用本身在运行，就能解密备份文件。
- 旧版使用钥匙串随机密钥加密的文件保留兼容读取，成功读取后静默迁移到新方案。

### 2.3 修改主密码后同步更新生物识别凭证

**现象**：改密会重新生成 Vault 数据密钥（新 salt + 新派生）。旧的生物识别密钥还是从「旧密码 + 旧 salt」派生的，用旧密钥去解新 Vault 会报 `Decryption failed: aead::Error`。

**修复**：

- `VaultService::change_password` 在 re-encrypt 并重新打开 Vault 后，调用 `BiometricManager::update_credential(account_id, new_key_hex)`。
- 仅当用户之前已启用生物识别时才更新；未启用则不操作。
- 这样改密后 Touch ID / Face ID 仍然可用，无需用户手动重新启用。

### 2.4 保存后回读钥匙串校验不可靠

**现象**：保存生物识别凭证后，尝试从钥匙串/文件回读以做一致性校验。在 macOS 上，回读钥匙串可能因权限或锁定失败，导致误报凭证失效。

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

推荐选项（按优先级）：

1. **Windows Credential Manager + DPAPI-NG（推荐）**
   - 将主密钥或一个用于派生主密钥的中间密钥存入 Windows Credential Manager。
   - 使用 `windows` crate 或 `keyring` crate 的 Windows native 后端。
   - 注意：Credential Manager 在某些域策略或 Windows Hello 配置下可能不可用，必须有备份方案。

2. **TPM / Windows Hello 专用密钥句柄**
   - 使用 Windows Hello 验证后，从 TPM 释放一个受生物识别保护的密钥。
   - 安全性最高，但实现复杂，需要处理不同 TPM 版本和 Windows 版本兼容性。

### 3.2 必须遵守的约束

| 约束 | 原因 | 实现建议 |
|------|------|---------|
| **备份文件必须使用确定性文件密钥** | 避免 Credential Manager / DPAPI 不可用时，备份文件也报废 | 复用 macOS 的 HKDF 方案：`info = b"solosoul:biometric:filekey:v1"`，salt = `account_id` |
| **修改密码后更新 Windows 凭证** | 旧凭证中的密钥无法解密新 Vault | 在 `VaultService::change_password` 中用新密钥调用 Windows 更新逻辑；仅当已启用时更新 |
| **保存前做派生校验** | 不要依赖保存后再回读 Credential Manager | 用当前密码派生密钥，与 `get_session_key()` 比较一致后再保存 |
| **错误返回 `__BIO_ERR__:<code>`** | 保持前端国际化逻辑一致 | 新增 Windows 专用 code：`windows_hello_unavailable`、`windows_auth_failed` 等 |
| **不直接暴露后端异常原文** | 避免 UI 截断和用户困惑 | 所有用户可见错误走 `biometricError.ts` 映射 |

### 3.3 建议的平台抽象

当前 `BiometricManager` 已经是 host-agnostic 的封装。实现 Windows Hello 时建议：

```rust
pub struct BiometricManager {
    base_path: PathBuf,
}
```

在 `BiometricManager` 内部按平台分发：

```rust
impl BiometricManager {
    pub fn save_credential(...) -> Result<(), String> {
        if cfg!(target_os = "macos") { /* macOS 实现 */ }
        else if cfg!(target_os = "windows") { /* Windows Hello 实现 */ }
        else { Err("platform not supported".into()) }
    }
}
```

- 公共接口保持统一：`save_credential`、`unlock`、`delete_credential`、`availability`。
- 平台相关代码放到私有模块，如 `biometric/macos.rs`、`biometric/windows.rs`。

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
| `tauri/crates/solosoul-core/src/biometric.rs` | 生物识别核心实现 |
| `tauri/crates/solosoul-core/src/vault_service.rs` | 改密时清除生物识别凭证 |
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
