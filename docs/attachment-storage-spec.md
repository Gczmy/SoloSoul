# 附件存储安全规范（Attachment Storage Spec）

> 当前事实核对：2026-09-28（RF-313）；代码基线 `105798cd`。
> 附件导出范围更新：2026-10-01（RF-015）；可恢复永久删除已由 RF-016 实现并完成本机验证，跨平台边界见 §8 和执行报告。
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
| 导出包 | `.solosoul` 附件使用导出包的加密路径，不能与 Vault 静态附件密钥混用；云全量快照与恢复包显式选择共享 `AttachmentExportScope::All`；手动导出空 ID 仍表示不选附件，显式范围和 GUI/CLI 共享用例见 §5 |

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

- [RF-014](REFACTOR_EXECUTION_REPORT_2026-09-25.md#rf-014)：云全量快照纳入原会话内未删除对象的未删除附件，复用现有导出包路径（原 ID 枚举接线已由 RF-015 的 All 替换）。该范围包括旧明文兼容附件与 SOLC 静态密文附件，包内均使用导出包加密。原单附件 100 MiB、总量 1 GiB 限制保持，文件缺失/元数据解析失败的既有行为未在本项改变；不能扩展为任意备份的完整性保证。验证状态见执行报告。
- [RF-015](REFACTOR_EXECUTION_REPORT_2026-09-25.md#rf-015)：共享 Core 枚举 `None / All / Selected(ids)` 已接入收集和估算。GUI 关闭总开关为 None、开启为 Selected，空数组仍为零选中；Core/CLI 的既有总开关开启为所选对象内 All，云/恢复直接选择 All。范围不包含删除的对象/附件，不改变格式、密钥域、单附件/总量限制或现有缺失文件处理；GUI/CLI完整共享用例仍待后续任务。本机验证见 [证据](verification/rf015-explicit-attachment-export-scope-2026-10-01.json)。
- [RF-021](REFACTOR_EXECUTION_REPORT_2026-09-25.md#rf-021) 的业务数据库批次与 [RF-022](REFACTOR_EXECUTION_REPORT_2026-09-25.md#rf-022) 的账户加密 journal/稳定附件阶段分开验收。正常 Session+key 生产路径可恢复冻结任务；Direct/无 key 一次性兼容 API 不因此获得该保证。实际检查结果以执行报告和验证证据为准。
- [RF-023](REFACTOR_EXECUTION_REPORT_2026-09-25.md#rf-023)：GUI/Cloud/Recovery 与旧 CLI/Core 兼容入口共用完整加密包导出执行器和原子 writer；Advanced/LegacyDirect 的包差异保留，旧 CLI 仍不提供 SOLC 源附件解密密钥。RF-024 已将完整导入迁入 Core；Advanced 与 CLI 的历史、选择和计数策略分别适配，共用 journal 提交与附件/偏好恢复管线。

## 6. 核验入口

[attachment_crypto.rs](../tauri/crates/solosoul-core/src/attachment_crypto.rs) 中已有密文往返、旧明文兼容、错误密钥和私有副本权限回归；[附件命令测试](../tauri/src-tauri/src/commands/attachment/tests.rs) 覆盖路径与鉴权边界。RF-313 仅逐条核对当前源码、文档链接和路径，没有重跑业务测试、Android 原生流程或用户旧附件迁移，不能把这些核对写成完整端到端验证。

## 7. 变更登记

| 日期 | 变更 | 关联 |
|------|------|------|
| 2026-08-16 | 首次登记当时的附件明文落盘例外 | P021（历史） |
| 2026-09-28 | 按当前源码区分新密文、旧明文兼容、有限迁移与临时明文，撤回全部明文/全部自动升级的概括 | RF-313 |
| 2026-09-30 | 云全量快照显式收集未删除附件；保留手动空选择、原加密/大小限制与其他待办边界 | RF-014 |
| 2026-10-01 | 共享 None/All/Selected 范围；GUI空选择不变，Core/CLI兼容总开关，云/恢复显式All | RF-015 |
| 2026-09-30 | 元数据与本机清理意图原子提交，失败保留明确重试；GUI/CLI 区分接受与物理完成 | RF-016 |
| 2026-10-01 | 原会话维护窗口、完整恢复引用与非空密文归属认证保护孤儿扫描，按实际unlink计数 | RF-903 |

## 8. 永久删除的提交与文件清理（RF-016）

GUI 单删、同对象批删和 CLI 永久删除统一使用清理意图执行器。在同一 SQLite 事务中移除真实附件元数据、更新对象版本/HLC并登记本机 `attachment_cleanup_intents`，数据库提交失败时文件不动。Schema 26→27 只增加该本机表；许可包含账户、元数据 owner、物理 storage 对象 ID、附件 ID、时间和固定重试状态，不保存文件名、绝对路径或原始错误，不进入包导出或同步。

执行时在原 `VaultSession` 和 Immediate 事务内重载许可，严格解密解析全部当前对象引用（含软删除对象），不能将坏记录降级为空。仍被引用、无法证明引用完整性、路径不安全或磁盘操作失败都保留许可；成功及确切的 `NotFound` 才确认完成。文件已清理但确认 SQL 失败时，下次允许以 `NotFound` 完成。标准删除目标仅由权威 storage ID/附件 ID 组成，`vaultPath/srcPath` 只作保守引用识别，不能作为销毁路径。

GUI 派发阻塞 worker 前、CLI 打开确认框前捕获会话。元数据提交后文件失败不回滚已接受的删除；GUI 仍触发同步并返回 `attachment_cleanup_pending`，显示“记录已删除，文件清理待重试”，重新加载已变更的元数据。CLI 同样区分完成与 pending，不能显示物理清理已完成。GUI 解锁维护和 CLI 进入解锁首页重试明确许可；CLI 手动清理若仍有 pending，不交给宽松孤儿扫描绕过保护。

显式永久删除不承诺历史快照能恢复已删除的实体，也不等同安全擦除存储介质。逐节点拒绝 symlink/junction/reparse、核对规范目录并在动作前复核；不承诺防住同用户外部进程在检查后替换路径。桌面同 canonical root 的协作式目录排他与在途附件 activity 由 RF-905 接入；旧未标记附件的归属扫描由 RF-903 处理，实施边界与当前验收见 §11。所有权和维护范围见 §10。验证状态见 [RF-016](REFACTOR_EXECUTION_REPORT_2026-09-25.md#rf-016)。

GUI 异步删除及列表刷新还校验原会话、组件生命周期与每次对象切换代次；工作区父计数回调先拒绝旧页面身份，避免取消新列表请求（RF-1065）。本机 Windows 的 43 项原生故障回归、完整 Rust workspace 与 CLI、前端检查均通过；Unix 专属 symlink 场景未在此机运行，未计作通过。原生界面与多端实测没有由 Hook 测试代替。详见 [RF-016 验证记录](verification/rf016-recoverable-attachment-deletion-2026-09-30.json)。

## 9. 导入任务的阶段与保全（RF-022）

持久导入仅记录加密计划、进度和加密 stages，不持久保存密码、包密钥或会话密钥。临时解包明文随真实 worker Drop 清理；进程退出残留由受控启动清理处理，不把正常 Drop 等同异常退出时也完成清理。

源 proof 与解包使用 Native 拥有的同一加密字节。业务批次与 journal 原子接纳后，按稳定 ZIP ordinal/附件 UUID 加密 staging，再以不覆盖目标的动作发布；各数据库阶段校验原 Session 和当前 epoch；文件发布或复用核验归属 marker 与当前附件钥 AEAD；元数据提交重读对象并核验账户、删除状态和附件基线。所有选中文件先发布，再按 owner 写入真实附件元数据。旧本地可用附件在数据库阶段保留，零附件选择不清空；并发其他字段变化不会由旧完整对象快照覆盖。

同一任务的实际已落盘文件数与已激活附件数分开；发布成功但阶段 SQL 失败可报告文件已写，仍不能宣称元数据关联完成。同一 ID Complete 返回原任务结果，不复活之后删除的附件。Recovery 只有全部 source-dependent 加密材料 ready 才接纳业务提交；准备失败不承诺失去随机口令后还能按原 ID 恢复。

marker/sidecar 目录不进入宽松旧孤儿扫描。受控删除重载 authenticated journal 与全部当前/软删除对象引用；未知、坏标记、foreign root/account 或 pending 操作保全。RF-905 在桌面以同 canonical Native root 的 OS 锁协调独立 GUI/CLI 实例，并以 owner/activity 保留本机实际任务生命周期。存在任何本项 journal 的目录迁移仍拒绝，SAF 失效保留原 cache root；不将目录锁等同于 journal relocation。旧未标记文件归属见 §11，完整 relocation 另需专门协议。

本机 SQLite/加密包、实际 child checkpoint 重开和前端模拟证据分别登记。未在 Android 真实设备运行 SAF/目录切换/文件选择器或多端原生界面，不将 Windows Rust 或 Hook 测试写成这些验证通过。六项既有平台/CI条件保持，验证状态见 [RF-022](REFACTOR_EXECUTION_REPORT_2026-09-25.md#rf-022)。

## 10. 目录所有权与附件工作寿命（RF-905）

[VaultRootOwner](../tauri/crates/solosoul-vault/src/root_owner.rs) 规范化真实 Native root，桌面 GUI/CLI 在数据库打开、迁移和业务写入前取得同协议 OS 排他锁；获取失败返回错误，不以未持锁构造继续写。服务、Store、Session 和受管 FS 克隆显式保留同一 owner；同应用多连接使用 `open_owned`/`try_with_root_owner`，不按路径隐式复用另一实例。该协议不约束任意外部文件程序或用户外拷副本。

兼容 API `VaultStore::open` 只有在账户目录的 `config.json` 确切不存在时才作为 standalone 入口自行取得声明根的 owner；配置存在、不可读或无法判定均返回 `VAULT_ROOT_OWNER_REQUIRED`，不打开受管数据库。Core/GUI/CLI 使用严格 Result 构造与显式 owner。桌面锁采用 `std::fs::File::try_lock`，要求 Rust 1.89 或更新版本，并与既有基于 `fs2` 的 `ProcessLock` 协议互操作。

插件全局 data root 与 Native root 相同时显式复用已有 owner，不同根时独立获取；取得原 VaultStore 的运行，其真实 Wasm worker 同时保留插件根与 Native 根的 activity/owner，直到执行与 workspace 清理结束；无 Vault 的运行仅保留插件根。默认 GUI 的插件降级只使用明确持锁的 TEMP 根，不以未持锁的 busy home 根继续写；native-perf 入口失败即停止。CLI 日志 writer 保留 Native owner；GUI 日志目录实际位于 Native root 内时，其真实 writer 和 panic fallback 也保留该 owner，位于 root 外时不扩大持锁范围。

GUI 附件复制、解密下载、系统打开/分享和清理 worker 在派发前冻结原 owner/所需会话并登记 activity，将 guard 移入真正工作体；取消 await 不提前释放。同步的附件写入也持同一 root activity，维护期间新会话拒绝进入。系统打开/分享完成源文件复制后，外部阅读器与后续临时副本清理仅处理临时目录；其权限和尽力清理边界仍按 §4，不承诺退出时安全擦除。

改密/KDF 升级、删除账户和目录替换先取得维护 guard，阻止新 activity，并以 `disable_and_wait`/`stop_and_wait` 等待旧同步 worker 真实退出并回收其持有的 Store 句柄；原 guard 借给 Core 维护主体，不能另叠 activity 或在排队后重新选择新服务。仍有在途 activity 时拒绝维护并稍后重试。存在任意 RF-022 journal 的目录迁移仍拒绝，SAF 失效保留原 cache；广泛未标记附件扫描与完整 relocation 未在 RF-905 实现。

Android/iOS 的 OS 文件锁为 no-op；本机 activity/maintenance gate 不代表远端 SAF provider 被排他锁定。实际执行命令、源码 SHA 与本机 Windows 结果以 [RF-905 验证记录](verification/rf905-root-ownership-2026-10-01.json) 为准；未执行平台、Android 真机 SAF、可见 GUI 操作和 release 透明 KDF 升级不从 Windows 原生或模拟测试推断通过。RF-016/RF-022 的六项既有平台/CI 条件和各自验证边界保持，任务状态由执行报告验收。

## 11. 原会话内的孤儿附件清理（RF-903）

调用方先捕获原 `VaultSession`，[Core 清理入口](../tauri/crates/solosoul-core/src/orphan_cleanup.rs) 校验该会话并取得同一 Native root 的排他维护 guard；调用方已持有 guard 时显式借用，不能叠加普通 activity。CLI `/attach cleanup` 先在同一窗口重试 RF-016 的明确删除意图，仍有 pending 就停止，不借孤儿扫描绕过原许可。可信附件写入在派发前持有 activity，所以文件已发布而元数据尚未提交时无法同时进入清理。

[引用视图](../tauri/crates/solosoul-vault/src/storage/attachment_references.rs) 逐行扫描当前账户的全部当前/软删除对象、对象与页面回收站、全部历史、数据库 Profile、可重新应用的未决同步冲突，以及 RF-022 的加密 journal/steps。没有 UI 分页截断；坏密文、非空坏 JSON、重复字段、未知恢复编码或单行超限均拒绝清理，不降级为空引用。真实 writer 的空 property labels、空 Profile 与旧冲突缺失历史哨兵按各自已知格式处理，不扩大为任意坏数据兼容。附件 ID 与 `vaultPath/srcPath` 等字面别名都用于保守保护；引用路径不授权删除位置。此处的 Profile 是数据库内数据。当前 `backups/*.solosoul_backup` 的恢复入口仅解码并保存 Profile，不重建 Object/附件实体；本扫描未把这些备份文件或任意位置的旧 metadata-only 导出包作为可重新应用的对象附件源，不承诺其中旧路径永久可用。

只有当前附件密钥完整认证的非空 SOLC v2 候选可进入删除。认证冻结真实打开句柄、头、分块数及长度，按固定大小分块读取并擦除明文缓冲；历史明文、SOLC v1、零分块、错误密钥、损坏、尾部追加、未知文件和混合目录均保留。未知归属不能由文件名、伪头或空目录推断为当前账户。已知 import marker/sidecar 只认可两个精确位置和已认证终态 journal；其他同名文件仍按附件载荷认证。

最终动作在原会话门闩及 SQLite Immediate 引用 guard 内复核完整数据库视图、引用、root/目录身份及文件身份。真实文件句柄身份用于比较，拒绝 symlink/junction/reparse 和多 hardlink；不以字符串路径或时间戳代替身份。候选变化或占用失败保留未删除部分；会话失效、根目录或数据库视图变化停止整轮，不迁移到新账户继续执行。文件动作之后没有新的可失败 SQL 提交步骤，已发生的删除不会因后续数据库错误丢失计数。

结果的 `removed` 仅计完整移除的附件目录，`files_removed/freed_bytes` 仅计实际成功 unlink 的普通文件，含已验证控制文件；部分删除失败也保留真实已移除数。字节数是逻辑文件长度，不代表磁盘分配量或安全擦除。`preserved/failed` 和整轮停止码区分保留与失败，CLI 有失败时显示未全部完成或已停止。

上述检查与排他协议不能保证任意同用户外部进程在系统调用间恶意改写的原子安全。移动 OS 锁及 SAF 的边界沿用 §10。本项当前验证状态以 [RF-903 执行报告](REFACTOR_EXECUTION_REPORT_2026-09-25.md#rf-903) 为准，未执行的平台不计作通过。

本机 Windows 正式 R **1738通过/0失败/3原有忽略**、CLI **323通过/0失败/2原有忽略**，fmt与Clippy均通过；完整来源、原Host断言保全、失败修正及精确SHA见 [RF-903验证记录](verification/rf903-owned-recoverable-attachment-cleanup-2026-10-01.json)。新Windows目录替换在后代打开前真实执行；认证后实际重命名被身份pins阻止并保持正常清理。Unix认证后目录替换与symlink测试保留但本机未运行，不计作通过。

## 12. 完整导出用例的共享边界（RF-023）

Core `export_import::export` 持有对象/模板/历史准备、密码校验、payload/manifest、HKDF附加条目、附件范围/大小校验与ZIP执行；`export_output` 持有同目录临时输出、finish/flush/sync/关闭与发布。GUI/Cloud/Recovery由原会话派生附件密钥并在原会话门闩内发布；实际worker持有owner/activity直到结束。LegacyDirect保留原公开签名、包字段、usize与无key SOLC错误，只加强输出原子性，不获得Session最终发布校验。

Advanced 全量含账户全部模板、每对象最新50历史及可选偏好/审计；Legacy只含引用模板，不追加历史、偏好、审计或contract_type_id。两种范围均不裁剪对象properties.__attachments。验证见 [RF-023证据](verification/rf023-core-encrypted-export-2026-10-01.json)。

## 13. 完整导入服务的共享边界（RF-024）

Core `export_import::import` 负责验证、解密预览、对象/模板/历史计划、提交与恢复，Host 只处理路径授权、阻塞池调度、wire 映射和进度通知。GUI、手动/自动云快照、Recovery 与 CLI 共用 Core 的原会话 journal 提交及后续恢复执行；原任务的源 proof、root binding、请求 fingerprint、计数和幂等规则保持。

Advanced 保留 SkipExisting/Overwrite/KeepBoth、逐对象策略、对象与附件 `None`/空数组选择、历史时间线和偏好。CLI 仍使用 SkipExisting/Overwrite/Merge 及原重复写计数、旧历史保持策略；共用提交规则不等于改成 GUI 的历史策略。Recovery handoff 与部分完成后的任务材料、根级密文和清理归属规则保持。旧 Direct、无附件密钥的一次性 API 仍不获得持久恢复保证。

真实包、事务失败、附件已发布但元数据失败后的关闭句柄重开恢复与原回归证据见 [RF-024 验证记录](verification/rf024-core-encrypted-import-2026-10-01.json)。验证限于本机 Windows，不代表已验证其他原生平台、可见 GUI 或远端云。
