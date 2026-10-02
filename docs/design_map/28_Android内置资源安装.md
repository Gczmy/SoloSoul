# 28 — Android 内置资源安装

> RF-206 的同步安装规范；后台准备与消费者就绪屏障由 RF-207 独立实施。

`npm run build` 中的 `stage-mobile-resources.cjs` 生成精简插件市场，并为 docs 与插件运行文件生成 `resources-mobile/bundled-resource-manifest.json`。Android 配置将此清单打包到 assets 根目录。清单 schema 为 1，列出路径、字节数及 SHA-256；内容版本为按路径排序后 `path\0size\0sha256\n` 的 UTF-8 字节摘要，与应用版本、时间或机器路径无关。资源内容改变就生成新版本。关键帮助索引和插件注册表缺失时构建失败，不产生伪完整资源。

Activity 保持现有同步等待行为，调用 `ResourceInstaller`。APK assets 根路径不存在时允许回退到 `resources/`。完成后的公开读取位置仍是 `dataDir/app_resources`，Rust 帮助和插件消费者的路径不变。

安装器以 `dataDir/resources/.solosoul-resource-install` 存放 stage 与 backup；该位置位于既有应用级 resources 排除范围，不属于账户保险库。暂存目录不会被 Rust 当作完成资源。处理流程是：

1. 上次目录切换中断且公开树缺失时恢复 backup。
2. 读取公开树内 `.bundled-resource-manifest`；版本相同且每个文件摘要匹配时跳过资源写入，返回 `writtenFiles=0`。仍执行读取校验，以修复损坏或丢失的文件。
3. 新建 stage，保留原树中旧清单未拥有的文件；首次升级没有清单时保守保留现有内容。仅移除旧清单明确管理、而新版本已移除的内置文件。
4. 将新清单全部资源写入 stage，逐文件校验长度和摘要，写入并同步完成标记。
5. 原公开树移至 backup，stage 移为公开树；切换失败恢复 backup，成功后清理 backup。中断复制不修改现有公开树，下次从 stage 重新准备。

路径禁止绝对路径、空组件、父级穿越、反斜线及控制分隔符；版本须匹配清单内容；现有资源及暂存路径中的符号链接被拒绝，避免越界读写或删除。安装操作在进程内串行执行。未修改用户账户、已安装插件或下载模型的业务存储路径。

安装异常会向调用方传播，不把部分复制当作成功；本项没有异步重试 UI，也没有更改 Rust 启动时序或资源消费者等待规则。相同版本的读取校验仍占用当前线程，这部分由 RF-207 的后台安装与屏障处理。

验收分别记录纯 Kotlin 的首次/重复安装、升级、摘要错误、复制中断、目录切换中断、损坏修复与非法路径回归；Node 清单稳定性及边界测试；实际 APK 清单/资源一致性、专用模拟器内安装和第二次启动零重写。Debug APK 与模拟器证据不代替 Release 签名或其他平台运行验证。
