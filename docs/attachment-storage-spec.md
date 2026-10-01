# 附件存储安全规范（Attachment Storage Spec）

> 当前事实核对：2026-09-28（RF-313）；代码基线 `105798cd`。
> 云快照附件范围更新：2026-09-30（RF-014）；可恢复永久删除已由 RF-016 实现并完成本机验证，跨平台边界见 §8 和执行报告。
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

改密/KDF 升级使用的 `reencrypt_attachments` 会将其扫描范围内的旧明文转为新密文。除旧 `account_dir/attachments` 扫描外，RF-022 从账户加密 journal 和 marker 核验根级 `attachments/` 中 Complete 或 Abandoned 的已激活 MetadataCommitted 文件，并纳入同次换钥；marker 和 sidecar 不当作附件重加密。Abandoned 未激活文件不作为可用附件处理。其他根级未标记文件、目录布局及同步收到的旧文件，不能据此认定已经全量升级；本项不改变旧明文读取兼容，也不实现旧未标记文件的广泛归属迁移。

## 4. 明文生命周期与权限

系统打开/分享先通过 [附件命令模块](../tauri/src-tauri/src/commands/attachment/mod.rs) 的 `resolve_verified_attachment_path` 校验对象、附件与 Vault 内路径，再由 `decrypt_to_temp_dir` 生成 UUID 子目录中的明文副本。Unix 私有副本从创建时采用 `0600`，目录尝试设为 `0700`。

调用路径安排 30 分钟后的尽力清理，外部阅读器可在此期间读取。清理由进程内后台线程执行：进程提前结束、系统占用文件或删除失败，都不保证副本消失；不能承诺“退出即清理”或把 30 分钟当作绝对最长保留时间。

Android 导入有 Kotlin 明文复制到 Rust 加密替换之间的窗口；复制、加密或 rename 失败时，当前分支没有覆盖所有残留文件的清理。正常成功路径最终为密文；该中间阶段不能作为“所有新附件长期明文”的依据。应用私有目录与系统加密提供额外隔离，但不替代应用自身对临时文件的管理。

[Vault 权限辅助函数](../tauri/crates/solosoul-core/src/vault_service/mod.rs) 在 Unix 设置受管路径权限，在 Windows 使用 `icacls` 移除继承并授予当前用户权限；不能把 `0700/0600` 等同于 Windows ACL，也不能由这些函数推断全部既存显式授权均被移除。同用户进程可读的旧明文、临时副本以及用户外拷目录仍是需保留的边界。

## 5. 相关任务与验收边界

- [RF-014](REFACTOR_EXECUTION_REPORT_2026-09-25.md#rf-014)：云全量快照入口收集原会话内未删除对象的未删除附件 ID，复用现有导出包路径。该范围包括旧明文兼容附件与 SOLC 静态密文附件，包内均使用导出包加密。原单附件 100 MiB、总量 1 GiB 限制保持，文件缺失/元数据解析失败的既有行为未在本项改变；不能扩展为任意备份的完整性保证。验证状态见执行报告。
- [RF-015](REFACTOR_EXECUTION_REPORT_2026-09-25.md#rf-015)：显式导出范围仍待执行；手动空附件选择继续表示不导出附件文件。
- [RF-021](REFACTOR_EXECUTION_REPORT_2026-09-25.md#rf-021) 的业务数据库批次与 [RF-022](REFACTOR_EXECUTION_REPORT_2026-09-25.md#rf-022) 的账户加密 journal/稳定附件阶段分开验收。正常 Session+key 生产路径可恢复冻结任务；Direct/无 key 一次性兼容 API 不因此获得该保证。实际检查结果以执行报告和验证证据为准。
- [RF-023](REFACTOR_EXECUTION_REPORT_2026-09-25.md#rf-023)、[RF-024](REFACTOR_EXECUTION_REPORT_2026-09-25.md#rf-024)：GUI/CLI 加密包共享用例；当前不能由一端修复推断另一端同样完成。

## 6. 核验入口

[attachment_crypto.rs](../tauri/crates/solosoul-core/src/attachment_crypto.rs) 中已有密文往返、旧明文兼容、错误密钥和私有副本权限回归；[附件命令测试](../tauri/src-tauri/src/commands/attachment/tests.rs) 覆盖路径与鉴权边界。RF-313 仅逐条核对当前源码、文档链接和路径，没有重跑业务测试、Android 原生流程或用户旧附件迁移，不能把这些核对写成完整端到端验证。

## 7. 变更登记

| 日期 | 变更 | 关联 |
|------|------|------|
| 2026-08-16 | 首次登记当时的附件明文落盘例外 | P021（历史） |
| 2026-09-28 | 按当前源码区分新密文、旧明文兼容、有限迁移与临时明文，撤回全部明文/全部自动升级的概括 | RF-313 |
| 2026-09-30 | 云全量快照显式收集未删除附件；保留手动空选择、原加密/大小限制与其他待办边界 | RF-014 |
| 2026-09-30 | 元数据与本机清理意图原子提交，失败保留明确重试；GUI/CLI 区分接受与物理完成 | RF-016 |

## 8. 永久删除的提交与文件清理（RF-016）

GUI 单删、同对象批删和 CLI 永久删除统一使用清理意图执行器。在同一 SQLite 事务中移除真实附件元数据、更新对象版本/HLC并登记本机 `attachment_cleanup_intents`，数据库提交失败时文件不动。Schema 26→27 只增加该本机表；许可包含账户、元数据 owner、物理 storage 对象 ID、附件 ID、时间和固定重试状态，不保存文件名、绝对路径或原始错误，不进入包导出或同步。

执行时在原 `VaultSession` 和 Immediate 事务内重载许可，严格解密解析全部当前对象引用（含软删除对象），不能将坏记录降级为空。仍被引用、无法证明引用完整性、路径不安全或磁盘操作失败都保留许可；成功及确切的 `NotFound` 才确认完成。文件已清理但确认 SQL 失败时，下次允许以 `NotFound` 完成。标准删除目标仅由权威 storage ID/附件 ID 组成，`vaultPath/srcPath` 只作保守引用识别，不能作为销毁路径。

GUI 派发阻塞 worker 前、CLI 打开确认框前捕获会话。元数据提交后文件失败不回滚已接受的删除；GUI 仍触发同步并返回 `attachment_cleanup_pending`，显示“记录已删除，文件清理待重试”，重新加载已变更的元数据。CLI 同样区分完成与 pending，不能显示物理清理已完成。GUI 解锁维护和 CLI 进入解锁首页重试明确许可；CLI 手动清理若仍有 pending，不交给宽松孤儿扫描绕过保护。

显式永久删除不承诺历史快照能恢复已删除的实体，也不等同安全擦除存储介质。逐节点拒绝 symlink/junction/reparse、核对规范目录并在动作前复核；不承诺防住同用户外部进程在检查后替换路径。跨客户端目录互斥、在途附件发布窗口和广泛孤儿归属扫描仍由 RF-905/RF-903 承接。验证状态见 [RF-016](REFACTOR_EXECUTION_REPORT_2026-09-25.md#rf-016)。

GUI 异步删除及列表刷新还校验原会话、组件生命周期与每次对象切换代次；工作区父计数回调先拒绝旧页面身份，避免取消新列表请求（RF-1065）。本机 Windows 的 43 项原生故障回归、完整 Rust workspace 与 CLI、前端检查均通过；Unix 专属 symlink 场景未在此机运行，未计作通过。原生界面与多端实测没有由 Hook 测试代替。详见 [RF-016 验证记录](verification/rf016-recoverable-attachment-deletion-2026-09-30.json)。

## 9. 导入任务的阶段与保全（RF-022）

持久导入仅记录加密计划、进度和加密 stages，不持久保存密码、包密钥或会话密钥。临时解包明文随真实 worker Drop 清理；进程退出残留由受控启动清理处理，不把正常 Drop 等同异常退出时也完成清理。

源 proof 与解包使用 Native 拥有的同一加密字节。业务批次与 journal 原子接纳后，按稳定 ZIP ordinal/附件 UUID 加密 staging，再以不覆盖目标的动作发布；各数据库阶段校验原 Session 和当前 epoch；文件发布或复用核验归属 marker 与当前附件钥 AEAD；元数据提交重读对象并核验账户、删除状态和附件基线。所有选中文件先发布，再按 owner 写入真实附件元数据。旧本地可用附件在数据库阶段保留，零附件选择不清空；并发其他字段变化不会由旧完整对象快照覆盖。

同一任务的实际已落盘文件数与已激活附件数分开；发布成功但阶段 SQL 失败可报告文件已写，仍不能宣称元数据关联完成。同一 ID Complete 返回原任务结果，不复活之后删除的附件。Recovery 只有全部 source-dependent 加密材料 ready 才接纳业务提交；准备失败不承诺失去随机口令后还能按原 ID 恢复。

marker/sidecar 目录不进入宽松旧孤儿扫描。受控删除重载 authenticated journal 与全部当前/软删除对象引用；未知、坏标记、foreign root/account 或 pending 操作保全。root 维护只覆盖已接入的本进程任务；存在任何本项 journal 的目录迁移拒绝，SAF 失效保留原 cache root。RF-905 的全目录/跨进程互斥仍未完成；旧未标记文件归属由 RF-903 后续处理，完整 relocation 另需专门协议。

本机 SQLite/加密包、实际 child checkpoint 重开和前端模拟证据分别登记。未在 Android 真实设备运行 SAF/目录切换/文件选择器或多端原生界面，不将 Windows Rust 或 Hook 测试写成这些验证通过。六项既有平台/CI条件保持，验证状态见 [RF-022](REFACTOR_EXECUTION_REPORT_2026-09-25.md#rf-022)。
