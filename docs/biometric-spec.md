# 生物识别安全规范（Biometric Security Spec）

> 最后更新：2026-08-16
> 关联修复：P004（Windows Hello 仅应用层门禁——平台限制登记）

## 1. 设计目标

生物识别用于**解锁保险库**：用户指纹/面部/PIN 验证通过后，从本地存储读取**会话密钥**并解锁。生物识别凭证本质是「把主密钥的安全存储绑定到平台级用户验证」，强度对标平台自身的安全承诺。

## 2. 三平台实现对比

| 平台 | 凭证存储 | 用户验证 | 加密级绑定 | 强度 |
|------|----------|----------|------------|------|
| macOS | **当前：本地文件**（`legacy.rs`，因无 Apple Developer 会员无 keychain entitlement）；未来：Keychain（`macos_keychain.rs` 已备） | Touch ID / Face ID / 设备密码（系统弹窗先于文件读取） | ⚠️ **仅应用层门禁**：文件加密密钥由**公开的 account_id** 经 SHA-256+HKDF 派生（无秘密输入）；同用户进程可直接重算并解密，不触发生物识别弹窗。**运行时**同设备攻击受系统弹窗拦截（与 Keychain 等价），**数据副本**离线攻击可还原主密钥 | 中（见 §5 限制） |
| Windows | DPAPI 本地文件（`CryptProtectData`） | Windows Hello（`UserConsentVerifier`） | ⚠️ **仅应用层门禁**：DPAPI 密钥绑定当前 Windows 用户登录凭据，**同用户身份运行的任意进程可直接 `CryptUnprotectData` 解密，不触发 Hello 弹窗** | 中（见 §3 限制） |
| 移动端 | 系统 Keystore/Keychain（经 `keystore_plugin`） | 指纹/面部/设备 PIN | ✅ 硬件绑定 | 高 |

## 3. Windows 平台限制（P004 登记）

### 3.1 威胁模型

- **已防御**：DPAPI 加解密绑定当前 Windows 用户登录凭据——跨用户/跨机器解密失败；旧版「公开 SHA256(account_id) 派生密钥」的弱点已修复（任何用户态进程可重算密钥还原主密钥的问题不复存在）。
- **残余缺口**：同一 Windows 用户身份下运行的任意进程（含恶意软件、其他 App）可以：
  1. 读取 `{vault}/{account_id}/biometric_key` 文件；
  2. 直接调用 `CryptUnprotectData` 解密，**无需触发 Windows Hello 弹窗**；
  3. 获得会话密钥后解锁保险库。

  对比 macOS 端 Keychain 生物识别 ACL 的加密级绑定，Windows 端验证与解密是**顺序执行而非加密绑定**——「验证了谁」与「谁能解密」没有耦合。

### 3.2 影响评估

- 攻击者需已具备**同用户任意代码执行**能力（此时大部分软件边界已失守）。
- 与「不启用生物识别、只用主密码」相比：主密码在内存中，同用户进程亦可注入读取；DPAPI 至少提供了用户凭据绑定的静态保护。
- 因此实际风险**中等**，属平台能力差距而非实现缺陷。

### 3.3 中期强化路线（backlog，未排期）

1. **Windows Hello Key Attestation（推荐）**：用 `KeyCredentialManager` 创建密钥对，私钥由 Windows Hello 保护（TPM/凭据保险库），验证时拿公钥挑战-应答——解密私钥路径完全离开用户态进程可触及范围。此为与 macOS Keychain ACL 对等的加密级绑定。
2. **DPAPI per-account 随机 entropy**：每账户生成随机 entropy，使其不随文件暴露（存于需 Hello 验证的载体）。注意：若 entropy 与 blob 同目录明文存放，则不能提升强度——必须存到保护级高于文件系统的位置才有效。

### 3.4 现有缓解（代码层面已具备）

- 生物识别凭证存的是**会话密钥**（可轮换），非主密码本身；泄露后可重新锁定+重置生物识别。
- 解锁后所有敏感操作仍受 App 内会话控制。
- `trigger_windows_biometric` 在读取凭证前先弹 Hello 验证（顺序防线，非加密绑定）。

## 4. macOS 当前文件方案限制（P035 登记）

> 事实澄清：本文档此前将 macOS 列为 Keychain 方案，但**当前实现为本地文件方案**
> （`macos.rs` 自述：团队未付费加入 Apple Developer Program，无法获得有效
> `keychain-access-groups` entitlement；`macos_keychain.rs` 为未来启用时的备选实现）。

### 4.1 威胁模型

- **已防御（运行时/同设备）**：`BiometricManager::unlock` 在读取凭证**之前**先触发
  系统生物识别弹窗（`trigger_system_biometric`，Touch ID / 设备密码）——同设备
  交互式攻击者（无指纹/设备密码）无法通过系统门禁读到密钥。此场景下安全性
  **与 Keychain 等价**（运行时门禁是系统级的，与应用用什么存储无关）。
- **残余缺口（数据副本离线攻击）**：`biometric_key` 文件加密密钥由**公开的
  account_id** 派生（SHA-256 + HKDF，`legacy.rs`），无任何秘密输入。攻击者拿到
  vault 目录副本（备份/被盗存储/恶意软件读文件）后，可离线重算密钥并解密文件，
  得到**主密钥明文**（`unlock_with_session_key` 直接用该密钥打开 Vault，无密码
  校验、无 KDF 重算）——**绕过主密码**。

### 4.2 影响评估

- 与 Windows（§3）同属「应用层门禁」：攻击者需已能读取 vault 目录文件。
- 对比 Keychain：Keychain 加密材料由 OS 密钥库保管、不随用户数据副本走，副本
  离线攻击无效；文件方案无此层保护。
- 生物识别凭证存的是**会话密钥**（可轮换），泄露后可重新锁定 + 重置生物识别。

### 4.3 中期强化路线（backlog，未排期）

1. **加入 Apple Developer Program 获取 keychain-access-groups entitlement**，
   切换到 `macos_keychain.rs`（Keychain 生物识别 ACL，加密级绑定）——与 Windows
   Hello Key Attestation（§3.3）对等。
2. 在未取得 entitlement 前，可在 UI 首次设置生物识别时以**说明性文字**（非警告条）
   告知「此平台以文件存储，数据副本泄露风险高于 Keychain」，不承诺「重新设置可修复」。

## 5. 测试与验证

- `windows.rs`：DPAPI roundtrip、legacy 迁移、可用性探测 shape 测试。
- 跨平台一致性：`mod.rs` 的 `BiometricStorage` trait 三平台实现共用同一接口与调用方语义。

## 6. 变更登记

- 2026-08-16（P004）：本文档创建，明示 Windows「仅应用层门禁」平台限制与中期强化路线。
- 2026-08-20（P035）：事实澄清——macOS 当前为本地文件方案（非 Keychain），登记数据副本离线攻击威胁模型与强化路线；撤销此前误加的设置页警告条（运行时门禁为系统级，警告条暗示「生物识别降低安全」不成立，且「重新设置」无法改变存储后端）。
