# 当前 iOS 系统文件选择器与非空预览补验

> **资料精简说明（2026-10-08）：**本页保留历史验收说明，批量原图、录屏、日志和输入快照已移出当前目录；下文的结果、来源与未完成项保持原验收边界。完整 6297 份原件已逐文件备份并建立 [SHA-256 索引](../fe2-evidence-index-2026-10-08.json)，恢复方法见[归档说明](../README.md)。迁出文件的 Markdown 链接指向固定原提交；代码块中的原路径及依赖完整目录的复跑命令需先恢复原件。

2026-10-08，针对 FE2-014 的专用 iPhone17 / iOS26.3.1 补验。先复用无签名模拟器生产二进制 `98280597fff53310efb49a8849556b5820d0a218a5262a429ae5c4d85f7f277f` 复现真实附件导入失败，随后修复 iOS 中转目录并构建 `2e4006a7227f771f442a98c259f84ff6c7ceff7044b21066975365e88d041d7d`。测试助手的导航和夹具修正与产品路径修复分别保留来源。

## 环境与夹具

使用独立助手 `com.solosoul.fe2.ios.probehost` 的 Documents 提供公开 TXT/PNG。助手实际 Info.plist 含 UIFileSharingEnabled 与 LSSupportsOpeningDocumentsInPlace；启动后通过自身文件 API 写入公开文件，驱动核对字节。TXT 仅含公开测试文本，PNG 是 320×240 合成图。客户端通过真实上传按钮和 UIDocumentPicker 选择文件；没有直接给对象插入附件元数据，也没有改产品权限白名单。

Xcode 会改写 PNG 资源，单独关闭 COMPRESS_PNG_FILES 仍有 CopyPNGFile 转换。因此最终助手将原始 PNG 作为 `.data` 资源，在自己的 Documents 中写成 `.png`，运行前检查实际打包资源与源文件字节一致。此方式只用于测试助手，不改变生产资源处理。

## 已保留的失败

| 目录 | 实际结果 | 原因与边界 |
| --- | --- | --- |
| initial-localization-failure | 原生 0通过/1失败/0跳过，退出65；驱动退出1 | 系统显示中文“浏览”，助手按英文 Browse 查找，未选择文件 |
| localized-navigation-failure | 原生 0通过/1失败/0跳过，退出65；驱动退出1 | 浏览已进入“我的iPhone”，助手仍查找本地根入口；公开目录当时未出现 |
| provider-fixture-preflight-failure | 驱动退出1；未执行产品UI测试 | 助手构建优化后的 PNG 与原文件不一致，夹具字节保护拒绝 |
| real-import-failure | 原生0通过/1失败/0跳过，退出65；驱动退出1 | 已通过真实选择器选中TXT，客户端提示源路径白名单拒绝，未生成附件；属于真实产品失败 |
| remembered-location-failure | 原生 0通过/1失败/0跳过，退出65；驱动退出1 | 系统选择器已在“我的iPhone”，存在两个“浏览”按钮；公开助手目录及2份文件已经可见，但测试点中不合适入口 |

前三次原生导航失败发生在文件选择前，不构成客户端导入失败证据；real-import-failure已实际选择文件并捕获产品路径拒绝。原始日志、助手源码和已采原生PNG/层级保留。各驱动恢复数据、外观并关闭专用模拟器；最近恢复的99项内容/链接/权限与当次及最初备份已独立比较。私有文件没有进入本目录。

## 当前补验状态

真实失败确认iOS文件中转原用Library/Caches，而原生附件命令允许应用临时目录。现在仅将iOS中转改为tempDir，Android/macOS/Windows沿用缓存；没有扩大后端路径白名单。新增回归修复前2失败/3通过，修复后定向2文件/10项通过，类型和定向ESLint退出0。新无签名Simulator包构建退出0，二进制2e4006a7227f771f442a98c259f84ff6c7ceff7044b21066975365e88d041d7d，106份涉及源码冻结、9组平台保护输入恢复。源码范围从103扩至106，新增共享中转实现/测试和平台依赖来源；旧103份没有变化。

最终 [final-native](https://github.com/Gczmy/SoloSoul/blob/75f1c1594504dd6e09af57ff6debd25cccce03fa/docs/verification/fe2-ios-native-file-picker-2026-10-08/final-native/report.json) 原生 **1通过、0失败、0跳过，退出0**，助手构建与保护驱动均退出0。真实创建公开合成账户和对象，通过系统 UIDocumentPicker 分别导入 TXT 和 PNG；浅色、深色下文本正文可见，照片集和查看器图像可见，实际预览返回、查看器返回照片集、附件关闭与详情关闭均留在当前对象页面。45组PNG及阶段界面层级归档，不包含自动录屏或私有备份。

四个稳定文本/查看器帧的 [独立像素检查](https://github.com/Gczmy/SoloSoul/blob/75f1c1594504dd6e09af57ff6debd25cccce03fa/docs/verification/fe2-ios-native-file-picker-2026-10-08/final-native/preview-frame-audit.json) 通过：标题最低对比10.7865:1、原生时钟最低5.4607:1，正文实际前景像素和照片两种公开色块存在，返回/缩放操作44pt且不被底部区域覆盖；代表截图已目测。这是所采稳定帧的内容与几何证据，不是连续动画、所有弹层或真机性能证明。

106份构建前来源与原生执行来源、当前文件逐项一致。[独立恢复核对](https://github.com/Gczmy/SoloSoul/blob/75f1c1594504dd6e09af57ff6debd25cccce03fa/docs/verification/fe2-ios-native-file-picker-2026-10-08/final-native/independent-restoration.json) 比较实际最终应用目录与本次及最初备份，99项记录（含根目录）内容/链接/权限全部相同；系统外观恢复light并复读，助手Host/Runner卸载，专用模拟器实际读回Shutdown。构建保护额外纳入Cargo.lock，9组输入恢复。终止测试应用命令返回3（当时进程已不在运行）；没有把该单条结果写成退出0，原生和整体保护驱动仍实际成功。

旧包的主题/键盘结果保持历史来源，新包仅接受上述导入与预览范围；PDF、系统分享、其他iOS版本、PIN/生物识别、完整生命周期与实体性能继续待验。

## 范围

助手 app、完整生产 app、xcresult、私有数据与平台保护备份只保留在 `/tmp`。公开目录仅保存小型助手源、日志、结果、截图与层级；截图附件清单按 PNG/阶段层级过滤，不导入自动录屏和合成事件文件。

macOS 图形工具仍报告锁屏，当前包完整材质与窗口恢复验收待继续；旧隔离进程画面不升级为新二进制结论。完整目标仍14项、8完成、6待验证。未提交、推送或发布。
