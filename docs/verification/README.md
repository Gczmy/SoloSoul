# 验收资料保留与恢复

本目录保留结论、适用范围、关键回执、代表性原图和自动化测试读取的真实夹具。重复截图、录屏、过程日志、历史输入快照及缓存放在仓库外，避免继续扩大日常检出目录。新增验收先使用仓库外工作目录，只把必要结果纳入版本控制；验收失败和历史来源同样需要保存，不能通过精简资料修改验收结论。

## FE2 资料精简（2026-10-08）

本次仅新增普通清理提交，保留原来的两个提交和全部历史，不进行 amend、rebase、reset 或强制推送。原始资料固定在提交 `75f1c1594504dd6e09af57ff6debd25cccce03fa`，产品代码固定在 `329f1ef0e`。从当前目录移出原件不等于从 Git 历史删除原件，因此完整克隆的历史体积不会随本次清理缩小。

- [完整原件索引](fe2-evidence-index-2026-10-08.json)：6297 个文件各有原路径、字节数、SHA-256 和 Git blob；`retained` 仅表示当前目录是否保持原件字节。
- 当前保留 83 份原件：19 张代表性 PNG、25 份关键 JSON、8 份压缩检查日志，以及 31 份判定测试所需的真实报告/日志夹具。截图没有压缩、缩放或格式转换。
- 50 份阶段 README 保留结论，并增加归档说明；[当前 checkpoint](fe2-frontend-checkpoint-2026-10-07.json)保留 174 项源文件哈希、最新检查和未完成项摘要。它们的原始完整版本也在归档和原提交中。
- 索引中的 `checkout_status: replaced_with_indexed_document` 表示原文已归档，当前同路径是说明或摘要；README 新版本的哈希独立记录在 `replacement_documents` 中，不能与原文哈希混用。
- 从当前目录移出 6163 份文件。产品、测试及原生适配代码保持不变；精简没有重跑或升级产品验收。

完整备份在本机仓库外：

```text
/Users/zzc/SoloSoul/verification-archives/fe2-2026-10-08-75f1c1594/
  fe2-original-evidence.tar.gz
  original-file-manifest.json
```

归档 SHA-256：`b31286f6032d0b19aa1dcf6051f708b60ed9684cc9adb1d1b01224cf61ceecac`。归档约 423.52 MiB，包含完整 6297 份原件，不只包含迁出文件。目录权限为 0700，文件为 0600。外部备份不随 Git 推送；其他机器可通过固定原提交恢复原件。新建归档目录及文件，未覆盖旧备份。

## 只读核对

在仓库根目录执行：

```bash
# 核对当前保留原件、README、源文件哈希及原提交中的迁出原件
PYTHONDONTWRITEBYTECODE=1 python3 docs/verification/scripts/verify_fe2_archive.py

# 在本机额外逐文件核对外部整包，不解压、不写入 Git
PYTHONDONTWRITEBYTECODE=1 python3 docs/verification/scripts/verify_fe2_archive.py \
  --archive /Users/zzc/SoloSoul/verification-archives/fe2-2026-10-08-75f1c1594/fe2-original-evidence.tar.gz

# 验证器拒绝损坏、遗漏、重复、不安全路径和符号链接等情况
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover \
  -s docs/verification/scripts -p 'test_verify_fe2_archive.py'

# 验证精简后原有真实报告夹具仍支持全部判定测试（无需连接设备）
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover \
  -s tauri/scripts -p 'test_android_home_native_regression.py'
```

结果仅说明资料完整性和判定测试能运行，不代表重新完成原生验收。原始构建、APK、截图、失败和各阶段源码范围以原报告为准。

## 恢复原件

先完成上述整包验证，再解压到一个新的仓库外目录：

```bash
fe2_restore_dir="$(mktemp -d /tmp/solosoul-fe2-recovered.XXXXXX)"
tar -xzf /Users/zzc/SoloSoul/verification-archives/fe2-2026-10-08-75f1c1594/fe2-original-evidence.tar.gz \
  -C "$fe2_restore_dir"
```

归档内使用完整仓库相对路径。需要运行依赖完整阶段目录的历史命令时，先在独立工作目录恢复原件；当前精简目录只保留选定资料。原文中的 `/tmp`、模拟器、历史包与进程标识是当时的执行记录，恢复文件本身不会重建这些运行环境。

没有本机备份时，可从原提交恢复单份资料到仓库外；例如恢复原始完整 checkpoint：

```bash
git show 75f1c1594504dd6e09af57ff6debd25cccce03fa:docs/verification/fe2-frontend-checkpoint-2026-10-07.json \
  > /tmp/solosoul-fe2-historical-checkpoint-75f1c1594.json
```

以上方法不切换当前分支，不覆盖当前摘要，不修改或删除既有提交。索引保留所有原路径，迁出的 Markdown 链接固定指向原提交。

## 代表性资料与待验证边界

| 资料 | 当前目录中的代表 |
| --- | --- |
| macOS 浅深首页与普通登录卡片 | [分区说明](fe2-macos-shell-contrast-2026-10-08/README.md)，截图来自浏览器，原生目测单独记录 |
| 滚动区域高亮 | [悬停补验](fe2-scroll-region-native-hover-2026-10-08/README.md)，保留四张 WebKit 浅深区域内外原图及像素回执 |
| Android 安全区、增强首页与键盘操作 | [移动端补验](fe2-current-mobile-followup-2026-10-08/README.md)，保留关键原生原图及报告 |
| Android 非空文件/照片预览 | [当前文件阶段](fe2-android-current-file-stage-2026-10-08/README.md)，保留文本及照片代表原图 |
| iOS 系统主题 | [移动端补验](fe2-current-mobile-followup-2026-10-08/README.md)，保留浅深合成帧和原生回执 |
| API31 仍失败的预览颜色诊断 | [诊断说明](fe2-android-api31-transition-alpha-2026-10-08/README.md)，保留失败原图和报告，未计作通过 |
| 提交前完整检查与环境失败记录 | [检查回执](fe2-push-checks-2026-10-08/receipt.json)，保留成功及失败压缩日志 |

FE2 当前为 **21 项：15 完成、0 进行中、6 待验证**。[归档后原生续验](fe2-macos-native-continuation-2026-10-08/README.md)已取得用户确认：当前 macOS 第二包浅色纯移动时正文内高亮，离开到侧栏和顶部后变灰。Mac 已解锁并恢复验收，侧栏展开与工具菜单命中通过；随后用户确认深色纯移动与普通顶栏拖拽正常，FE2-020已关闭。Android API31 暗色预览的严格像素检查仍失败。完整 macOS 材质/恢复、其他移动端矩阵、Windows 原生及实体性能等范围继续按[主执行报告](../REFACTOR_FRONTEND_EXECUTION_REPORT_2026-10-07.md)登记。

同一冻结隔离包的新公开文本及照片100%预览已观察；旧附件的路径引用了旧测试目录，原失败保留。新照片120%后原生采集失败，稳定画面及当前窗口状态未确认，没有因此重启应用或修改产品代码。详情仍见[续验回执](fe2-macos-native-continuation-2026-10-08/receipt.json)。

后续同句柄采集已恢复，无重启；新附件120%/144%/复位及照片查看器100%/120%/返回稳定视图已补证。当前为深色首页、工具展开；完整恢复、交通灯及纯移动/拖拽待验，旧失败证据保留。

用户随后确认深色纯移动区域切换和顶部拖拽正常，FE2-020已关闭；其他六项完整原生验收保持待验证，不能将此次反馈扩为全矩阵通过。

当前浅色矮窗口工具滚动、后台返回无闪黑及顶部/圆角正常已获用户确认；矮窗口主密码登录实际补验通过。全屏操作中黑色顶部层与后续正常视图均保留，原因及退出复验待确认，详见续验回执；不升级为完整macOS矩阵通过。

2026-10-09全屏黑栏修复采用原生工具栏生命周期切换，专项两次0/52pt通过；用户手动解锁后，最终完整客户端深浅色全屏、侧栏/指南实际点击、退出顶栏对齐的稳定视图已通过，纯指针移动与普通拖拽交还用户确认。旧命中失败及32pt候选失败保留，见[本次续验](fe2-macos-native-continuation-2026-10-08/README.md)。大日志、包与构建清单仍在仓库外，仅回执保留来源及SHA256。

用户确认v3仍有退出栏高和进入短暂黑条问题后，已实现v4：动画前移除空工具栏，退出完成及下一轮主运行循环恢复并通知网页，桌面AppBar至少52px。26项前端/15项Rust窗口/4项浏览器过渡回归通过，真实AppKit两次0/52pt，完整客户端深浅色网页栏高稳定；瞬间黑条仍待人工反馈。旧v3证据未替换，见[续验回执](fe2-macos-native-continuation-2026-10-08/receipt.json)。

用户随后明确反馈 **“修复成功，提交推送”**。本次v4的进入黑条与退出顶栏高度缺陷按人工验收关闭，1370项生产源码复核与已测构建一致；提交推送已获授权。其余完整FE2原生矩阵及旧命中断言失败仍保留，不随本次两项缺陷的关闭提升为全量通过。
