# 附件存储安全规范（Attachment Storage Spec）

> 当前事实核对：2026-09-28（RF-313）；代码基线 `105798cd`。
> 云快照附件范围更新：2026-09-30（RF-014）；其余安全事实沿用 RF-313。
> 历史来源：2026-08-16 P021 的明文落盘例外登记。该日期保留为历史，不能用其旧结论代替当前实现。

## 1. 当前实现与边界

附件支持预览、系统打开和分享。当前带附件密钥的正常导入已加密落盘；历史明文兼容、Android 导入中间文件及系统打开/分享的临时明文仍需分别看待。不能概括为“附件全部明文”，也不能保证“磁盘上从无附件明文”。

## 2. 写入、存储与传输

| 路径 | 当前行为与源码 |
|------|----------------|
| 加密格式 | [attachment_crypto.rs](../tauri/crates/solosoul-core/src/attachment_crypto.rs) 从会话密钥经 HKDF 派生 32 字节附件密钥，使用独立 info `solosoul:attachments:at-rest:v1`；`encrypt_chunked_stream` 写入带 `SOLC` 头的分块密文，与导出包附件密钥域分离 |
| 桌面 GUI 导入 | [attachment_copy_file](../tauri/src-tauri/src/commands/attachment/crud.rs) 获取附件密钥后加密写入 Vault 的 `attachments/{object_id}/{attachment_id}/` 路径，文件名经净化 |
| Core / CLI | [add_attachments](../tauri/crates/solosoul-core/src/objects.rs) 有密钥时加密，`None` 分支仍明文复制；[CLI 附件入口](../solosoul_cli/src/commands/attachment.rs) 将获取密钥错误转为 `None`。这是现存 API 边界，普通已解锁路径是否能触发尚未复现，不能直接登记为已证实漏洞 |
| Android 导入 | [Kotlin 插件](../tauri/src-tauri/gen/android/app/src/main/java/com/solosoul/app/AttachmentImportPlugin.kt) 先将 content URI 明文复制到应用管理的 Vault 路径，[Rust 插件](../tauri/src-tauri/src/attachment_import_plugin.rs) 再写加密临时文件并替换；不是从源到目标全程无明文中间文件 |
| 元数据 | 名称、描述、标签、大小等位于对象 `properties.__attachments`。正常写入随属性加密，但 [存储加密层](../tauri/crates/solosoul-vault/src/encryption.rs) 保留历史明文读取/迁移兼容，不宜写“所有历史元数据永远都是密文” |
| 局域网同步 | [sync/attachments.rs](../tauri/crates/solosoul-sync/src/attachments.rs) 经同步通道传递源文件实际字节，接收临时文件后替换；不会将所有旧明文统一改成密文 |
| 导出包 | `.solosoul` 附件使用导出包的加密路径，不能与 Vault 静态附件密钥混用；云全量快照枚举未删除对象中的未删除附件 ID；手动导出空 ID 仍表示不选附件，显式范围和 GUI/CLI 共享用例见 §5 |

附件密钥入口实际位于 [vault_service/unlock.rs](../tauri/crates/solosoul-core/src/vault_service/unlock.rs) 的 `attachment_encryption_key`，需要当前会话密钥。不能按旧文档查找已拆分的 `vault_service.rs`。

## 3. 旧数据兼容与迁移范围

`copy_decrypt_file` 和 `read_file_decrypted` 检查 `SOLC` 文件头：密文解密，旧明文按原字节读取/复制。读取成功不改写原文件，因此“新版本能打开旧附件”不代表旧附件已加密迁移。

改密/KDF 升级使用的 `reencrypt_attachments` 会将其扫描范围内的旧明文转为新密文，但当前只扫描 `account_dir/attachments`。根级 `attachments/`、其他目录布局及同步收到的旧文件，不能据此认定已经全量升级。此项文档修正没有执行用户数据迁移，也没有改变兼容格式。

## 4. 明文生命周期与权限

系统打开/分享先通过 [附件命令模块](../tauri/src-tauri/src/commands/attachment/mod.rs) 的 `resolve_verified_attachment_path` 校验对象、附件与 Vault 内路径，再由 `decrypt_to_temp_dir` 生成 UUID 子目录中的明文副本。Unix 私有副本从创建时采用 `0600`，目录尝试设为 `0700`。

调用路径安排 30 分钟后的尽力清理，外部阅读器可在此期间读取。清理由进程内后台线程执行：进程提前结束、系统占用文件或删除失败，都不保证副本消失；不能承诺“退出即清理”或把 30 分钟当作绝对最长保留时间。

Android 导入有 Kotlin 明文复制到 Rust 加密替换之间的窗口；复制、加密或 rename 失败时，当前分支没有覆盖所有残留文件的清理。正常成功路径最终为密文；该中间阶段不能作为“所有新附件长期明文”的依据。应用私有目录与系统加密提供额外隔离，但不替代应用自身对临时文件的管理。

[Vault 权限辅助函数](../tauri/crates/solosoul-core/src/vault_service/mod.rs) 在 Unix 设置受管路径权限，在 Windows 使用 `icacls` 移除继承并授予当前用户权限；不能把 `0700/0600` 等同于 Windows ACL，也不能由这些函数推断全部既存显式授权均被移除。同用户进程可读的旧明文、临时副本以及用户外拷目录仍是需保留的边界。

## 5. 相关任务与验收边界

- [RF-014](REFACTOR_EXECUTION_REPORT_2026-09-25.md#rf-014)：云全量快照入口收集原会话内未删除对象的未删除附件 ID，复用现有导出包路径。该范围包括旧明文兼容附件与 SOLC 静态密文附件，包内均使用导出包加密。原单附件 100 MiB、总量 1 GiB 限制保持，文件缺失/元数据解析失败的既有行为未在本项改变；不能扩展为任意备份的完整性保证。验证状态见执行报告。
- [RF-015](REFACTOR_EXECUTION_REPORT_2026-09-25.md#rf-015)：显式导出范围仍待执行；手动空附件选择继续表示不导出附件文件。
- [RF-021](REFACTOR_EXECUTION_REPORT_2026-09-25.md#rf-021)、[RF-022](REFACTOR_EXECUTION_REPORT_2026-09-25.md#rf-022)：导入批次事务及附件恢复/重试幂等；不要把计划中的失败恢复写成当前保证。
- [RF-023](REFACTOR_EXECUTION_REPORT_2026-09-25.md#rf-023)、[RF-024](REFACTOR_EXECUTION_REPORT_2026-09-25.md#rf-024)：GUI/CLI 加密包共享用例；当前不能由一端修复推断另一端同样完成。

## 6. 核验入口

[attachment_crypto.rs](../tauri/crates/solosoul-core/src/attachment_crypto.rs) 中已有密文往返、旧明文兼容、错误密钥和私有副本权限回归；[附件命令测试](../tauri/src-tauri/src/commands/attachment/tests.rs) 覆盖路径与鉴权边界。RF-313 仅逐条核对当前源码、文档链接和路径，没有重跑业务测试、Android 原生流程或用户旧附件迁移，不能把这些核对写成完整端到端验证。

## 7. 变更登记

| 日期 | 变更 | 关联 |
|------|------|------|
| 2026-08-16 | 首次登记当时的附件明文落盘例外 | P021（历史） |
| 2026-09-28 | 按当前源码区分新密文、旧明文兼容、有限迁移与临时明文，撤回全部明文/全部自动升级的概括 | RF-313 |
| 2026-09-30 | 云全量快照显式收集未删除附件；保留手动空选择、原加密/大小限制与其他待办边界 | RF-014 |
