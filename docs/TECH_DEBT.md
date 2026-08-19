# 技术债务登记（TECH DEBT）

> 目的：`docs/CODE_ANALYSIS_REPORT.md` 归档后，评估关闭/登记不处理的债务仍需可追踪的载体
> （轮次 2 复核建议）。
> 约定：每项含「触发条件」——满足后按对应修复模式处理；关闭时打勾并记录提交哈希与日期。

---

## D1. 超长函数/组件 Top 10（P022，评估后不拆）

评估结论：无功能 bug 证据、纯可读性问题；前几轮性能优化中已被多次触碰且测试稳定，机械拆分回归风险高、无用户可见收益，**随改随拆**。

触发条件：后续功能迭代需大幅修改下列任一文件时，按项目已有拆分先例（W005、P046）同步拆分。

| 行数 | 位置 | 函数/组件 |
|---|---|---|
| 391 | `src/hooks/useLlmChatCore.ts:63` | useLlmChatCore |
| 388 | `src/components/attachment/AttachmentPreviewOverlay.tsx:34` | AttachmentPreviewOverlay |
| 388 | `src/hooks/useRecoveryReceive.ts:28` | useRecoveryReceive |
| 386 | `src/pages/ai/PluginDashboardPage.tsx:35` | PluginDashboardPage |
| 376 | `src/components/layout/AddPageButton.tsx:24` | AddPageButton |
| 374 | `src/pages/ai/useLlmConfigPage.ts:48` | useLlmConfigPage |
| 369 | `src/components/settings/PinSection.tsx:20` | PinSection |
| 369 | `src/pages/settings/VaultDirectorySection.tsx:27` | VaultDirectorySection |
| 362 | `src/components/sync/RecoveryQrContent.tsx:19` | RecoveryQrContent |
| 357 | `src/components/attachment/PhotoAlbumOverlay.tsx:33` | PhotoAlbumOverlay |

## D2. 深层嵌套热点（P023，低风险两处已修，其余登记）

已修：`useAttachmentManagerBatchOps.ts` 三重 for → flatMap；`useDragToAttach.ts` drop 分支抽模块级 `handleDropFiles`（6 层→4 层）。

登记不处理（提取收益有限且无测试覆盖，改动时随改随拆）：

- 5 层控制流边界：`useExportScope.ts:251-262`、`useTouchZoom.ts:184`、`propertyFlatten.ts:86`、`useExportImportPage.tsx:247`、`settingsStore.ts:446`
- JSX brace 深度 11：`DeviceListKnownCard.tsx:93-106`（建议抽子组件）

## D3. 主密码验证不限速残留路径（P012 登记残留）

轮次 2.5 修复了复核枚举的三处（导出校验 / biometric 保存删除命令层 / pin_disable）。以下路径仍直走 `verify_password_core` 或 `VaultService::verify_password`（不计失败、不触发阶梯锁定）：

- `BiometricManager::verify_password`（`solosoul-core/src/biometric/mod.rs`）——`save_credential`（:296）/`delete_credential`（:357）内部路径，CLI `security.rs` + 桌面命令层 `biometric.rs:357/709` 调用；构造不持有 VaultService，接入锁定需改 API 与测试夹具
- CLI 各 `verify_password` 调用（`solosoul_cli/src/app.rs:631`、`commands/export_import.rs:355`、`commands/security.rs:326`）——本地终端工具，shell 访问已等同持有账户目录，oracle 价值低

触发条件：若 UI 端新增不经过 PasswordVerificationDialog 的主密码验证入口（或安全威胁模型升级），将三条路径统一接入 `VaultService::verify_password_with_lockout`。

## D4. P001 次要瑕疵（复核登记，非打回项）

- `Zeroizing<[u8;32]>` 多处拷为普通 `[u8;32]` 用后不清零（`attachment_crypto.rs` 等）——密钥残留堆内存，属纵深防御项
- `attachment_import_content_uri` Kotlin 先明文落盘再由 Rust 就地加密——崩溃窗口仍在（仅缩短）
- CLI `attachment.rs:106` 取附件密钥失败 `.ok()` 静默降级为明文写入——应改为显式报错并中止

## D5. P013 导出快照批量查询（8612e564）两点登记

- 错误处理语义变化：由「静默跳过失败快照」变为「中止整个导出」（更合理但报告未提及，改动时需知悉）
- 批量 SQL 外层无 `ORDER BY` 保证（doc 注释声称的排序不成立；对导出导入无实际影响）

## D6. 提交信息措辞纪律（P005/P023 复核登记）

- P005 commit（54b02ac8）称「避免整条 data 解密」不实——`list_trash_items` 本身仍为提取 `contract_type_id` 解密每行 data（非本次引入），省掉的是第二次解密
- P023 commit（7ca9c667）称「该批文件均无单测」不实——`settingsStore.test.ts`、`propertyFlatten.test.ts` 存在

要求：后续 commit 中的性能/测试断言先与代码现状核对再落笔。