# 当前 iOS PDF 附件导入、现有失败反馈与返回

> **资料精简说明（2026-10-08）：**本页保留历史验收说明，批量原图、录屏、日志和输入快照已移出当前目录；下文的结果、来源与未完成项保持原验收边界。完整 6297 份原件已逐文件备份并建立 [SHA-256 索引](../fe2-evidence-index-2026-10-08.json)，恢复方法见[归档说明](../README.md)。迁出文件的 Markdown 链接指向固定原提交；代码块中的原路径及依赖完整目录的复跑命令需先恢复原件。

2026-10-08，iPhone17/iOS26.3.1，复用当前2e4006a7…生产包，没有改客户端源码或新增PDF渲染器。生成两页公开合成PDF，pypdf核对页数/文字，Poppler渲染两页并目测无缺字或裁剪；实际助手资源字节相同，通过UIDocumentPicker真实导入。

当前移动端PDF绕过WebView embed，调用attachment_open；iOS没有Android PdfRenderer/系统打开桥接，走opener的Unix分支并实际报错。此为现有能力边界：测试只接受浅深主题错误提示可见、文件名完整、仍保留附件面板及逐层返回原工作区，不能把预期错误反馈当PDF打开/渲染成功。当前提示使用“请确认文件仍存在”，没有直接说明平台不支持，属于已有提示边界，本轮未扩展原生功能。

首轮原生1通过/0失败/0跳过、退出0，但桌面构建与收尾重叠，src-tauri/Cargo.toml暂态变化被源码保护拒绝，整体驱动退出1；[原文](https://github.com/Gczmy/SoloSoul/blob/75f1c1594504dd6e09af57ff6debd25cccce03fa/docs/verification/fe2-ios-pdf-feedback-2026-10-08/source-guard-rejection/report.json)保留，不改写通过。桌面构建恢复后串行复验，[最终](https://github.com/Gczmy/SoloSoul/blob/75f1c1594504dd6e09af57ff6debd25cccce03fa/docs/verification/fe2-ios-pdf-feedback-2026-10-08/final-native/report.json)原生1通过/0失败/0跳过，助手构建/原生/整体驱动均退出0，106份构建/执行/当前来源一致。

[两帧独立检查](https://github.com/Gczmy/SoloSoul/blob/75f1c1594504dd6e09af57ff6debd25cccce03fa/docs/verification/fe2-ios-pdf-feedback-2026-10-08/final-native/feedback-frame-audit.json)确认实际错误文字像素，对比最低10.2588:1；提示在安全区内，原附件名称/上传按钮仍存在，操作44pt。PNG/AX数量分别见两轮archive-scope，完整录屏/事件不归档。终止测试应用3表示测试已自行结束，原文保留。

最终实际99项记录（含根目录）内容/链接/权限和根权限与本次、首轮及最初备份一致；light外观复读恢复、助手卸载、设备实际Shutdown。完整app/xcresult/私有备份仅/tmp。仅该模拟器现有不支持能力的反馈与返回；PDF打开/系统分享、其他iOS/大文件/完整生命周期和实体性能未验，目标仍14项、8完成、6待验证，未提交推送发布。
