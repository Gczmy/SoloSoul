# RF-312 公开媒体数据集合同

本入口为首次 OCR／附件预览的原生测量准备独立数据集。它不测量 GUI，不替代原生 OCR／预览行程，也不改变已经验收的 v1 启动、解锁和搜索数据集。

## 数据与验证

- 保留 100／5000 个固定对象、每 20 个对象一次 `needle` 命中、相同账户与 Profile 内容。
- 仅 `obj_perf_00000000` 添加四个公开附件：仓库 OCR 测试 PNG、文本 PDF、扫描 PDF，以及固定的 `preview.txt`。前三个素材以 `include_bytes!` 固定在示例程序中；第四个是固定公开文本。
- 通过 `solosoul_core::objects::add_attachments` 和真实会话派生密钥写入 SOLC 密文；附件 UUID、随机盐、nonce、操作时间和密文字节会变化。公开素材字节和对象基础内容固定。
- 完成标记为 `rf312-media-fixture.json`（schemaVersion 2），明确包含附件与 OCR 样例。旧原生启动工具继续只接纳 v1，不会误将本数据集算入旧基线。完成标记的 `baseFixture.kdf` 从新建账户 `config.json` 读取实际参数，不因创建后的环境变化而改写；[RF-1096 红绿验证](../verification/rf1096-fixture-kdf-2026-10-06.json)覆盖两个变化方向及独立重开。
- 验证先检查固定标记、路径白名单、普通文件及密文 SHA，再只解锁临时副本，核对全部对象内容、账户、Profile、搜索结果和四个附件的解密字节。源目录所有已允许文件（包括标记和备份）在验证前后保持 SHA 相同。
- 复制只写入新目录，将附件 `vaultPath` 重定位到复制后的 Vault，并清除 `srcPath`；复制结束后再通过另一个临时副本独立重开验证。源目录不会被打开为 Vault。
- 不接纳额外文件、reparse point、重复或未知选项、非固定素材、明文附件、错误对象内容。Release 数据集强制生产级 Argon2id：64 MiB／3 iterations／4 parallelism。

## Windows 复跑

在 `tauri/` 中构建；数据输出的父目录必须已存在，每个输出目录必须尚不存在：

```powershell
$env:SOLOSOUL_SECURE = '1'
cargo build --locked --release -p solosoul-core --example perf_baseline

$cargoMetadata = cargo metadata --locked --no-deps --format-version 1 | ConvertFrom-Json
$fixtureExe = Join-Path $cargoMetadata.target_directory 'release/examples/perf_baseline.exe'
& $fixtureExe --media-fixture-output 'C:\TEMP\rf312-media\vault100' --objects 100
& $fixtureExe --media-fixture-output 'C:\TEMP\rf312-media\vault5000' --objects 5000
& $fixtureExe --verify-media-fixture 'C:\TEMP\rf312-media\vault100'
& $fixtureExe --verify-media-fixture 'C:\TEMP\rf312-media\vault5000'
& $fixtureExe --copy-media-fixture 'C:\TEMP\rf312-media\vault100' --media-fixture-output 'C:\TEMP\rf312-media\sample100-01'
& $fixtureExe --verify-media-fixture 'C:\TEMP\rf312-media\sample100-01'
```

`$fixtureExe` 通过 Cargo metadata 指向实际构建的 `perf_baseline.exe`；本轮验收另外保存了该 EXE 的冻结副本和 SHA。以上命令必须等待程序退出，并保存每次 stdout、stderr 和实际退出码。失败目录和失败输出保留，完成标记仅在全部检查通过后发布。

## 后续原生行程

先让 Windows `native-perf` 接纳本合同并独立验证、重定位 owned 副本，再通过 SDK 的真实输入打开附件管理器和预览。图片结束条件包括 `complete`、预期自然尺寸和实际绘制帧；文本结束条件包括固定公开内容和实际绘制帧。PDF 需要确认原生 PDF 渲染结束条件，不能只以 `<embed>` 存在计成功。

OCR 的真实文件选择使用系统对话框；SDK 的 WebView 输入不能直接控制该对话框。需要把对话框输入绑定到本次 owned GUI 进程和公开样例路径，不能注入路由参数、业务 IPC 或替换为浏览器 mock 冒充原生行程。后端 OCR 分段测量可以作为性能归因证据，但不替代完整界面测量。

这些行程、重复样本、内存／IPC 采样及性能归因仍由 RF-312 承接，本数据集验收不关闭 RF-312。
