# 生物识别安全规范（Biometric Security Spec）

> 最后更新：2026-08-16
> 关联修复：P004（Windows Hello 仅应用层门禁——平台限制登记）

## 1. 设计目标

生物识别用于**解锁保险库**：用户指纹/面部/PIN 验证通过后，从本地存储读取**会话密钥**并解锁。生物识别凭证本质是「把主密钥的安全存储绑定到平台级用户验证」，强度对标平台自身的安全承诺。

## 2. 三平台实现对比

| 平台 | 凭证存储 | 用户验证 | 加密级绑定 | 强度 |
|------|----------|----------|------------|------|
| macOS | Keychain（`SecItem`） | Touch ID / Face ID / 设备密码 | ✅ Keychain 生物识别 ACL：仅本 App + 生物识别成功时才能读出 kSecAttrAccessibleWhenPasscodeSetThisDeviceOnly 项 | 高（平台级） |
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

## 4. 测试与验证

- `windows.rs`：DPAPI roundtrip、legacy 迁移、可用性探测 shape 测试。
- 跨平台一致性：`mod.rs` 的 `BiometricStorage` trait 三平台实现共用同一接口与调用方语义。

## 5. 变更登记

- 2026-08-16（P004）：本文档创建，明示 Windows「仅应用层门禁」平台限制与中期强化路线。
