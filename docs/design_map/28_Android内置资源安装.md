# 28 — Android 内置资源安装

> RF-206 定义可恢复安装器；RF-207 在此基础上实现后台准备与消费者就绪屏障。

`npm run build` 中的 `stage-mobile-resources.cjs` 生成精简插件市场，并为 docs 与插件运行文件生成 `resources-mobile/bundled-resource-manifest.json`。Android 配置将此清单打包到 assets 根目录。清单 schema 为 1，列出路径、字节数及 SHA-256；内容版本为按路径排序后 `path\0size\0sha256\n` 的 UTF-8 字节摘要，与应用版本、时间或机器路径无关。资源内容改变就生成新版本。关键帮助索引和插件注册表缺失时构建失败，不产生伪完整资源。

RF-206 交付时 Activity 同步调用 `ResourceInstaller`；RF-207 已将调用移至后台，见下节。APK assets 根路径不存在时允许回退到 `resources/`。完成后的公开读取位置仍是 `dataDir/app_resources`，Rust 帮助和插件消费者的路径不变。

安装器以 `dataDir/resources/.solosoul-resource-install` 存放 stage 与 backup；该位置位于既有应用级 resources 排除范围，不属于账户保险库。暂存目录不会被 Rust 当作完成资源。处理流程是：

1. 上次目录切换中断且公开树缺失时恢复 backup。
2. 读取公开树内 `.bundled-resource-manifest`；版本相同且每个文件摘要匹配时跳过资源写入，返回 `writtenFiles=0`。仍执行读取校验，以修复损坏或丢失的文件。
3. 新建 stage，保留原树中旧清单未拥有的文件；首次升级没有清单时保守保留现有内容。仅移除旧清单明确管理、而新版本已移除的内置文件。
4. 将新清单全部资源写入 stage，逐文件校验长度和摘要，写入并同步完成标记。
5. 原公开树移至 backup，stage 移为公开树；切换失败恢复 backup，成功后清理 backup。中断复制不修改现有公开树，下次从 stage 重新准备。

路径禁止绝对路径、空组件、父级穿越、反斜线及控制分隔符；版本须匹配清单内容；现有资源及暂存路径中的符号链接被拒绝，避免越界读写或删除。安装操作在进程内串行执行。未修改用户账户、已安装插件或下载模型的业务存储路径。

安装器本身将异常向调用方传播，不把部分复制当作成功；异步重试 UI 和消费者等待由 RF-207 的协调器管理。相同版本读取校验仍在安装器调用线程执行，RF-207 确保该调用线程为后台工作线程。

验收分别记录纯 Kotlin 的首次/重复安装、升级、摘要错误、复制中断、目录切换中断、损坏修复与非法路径回归；Node 清单稳定性及边界测试；实际 APK 清单/资源一致性、专用模拟器内安装和第二次启动零重写。Debug APK 与模拟器证据不代替 Release 签名或其他平台运行验证。

## RF-207 后台准备与就绪

上述同步安装器现在由进程级 `ResourcePreparationCoordinator` 的单线程 executor 调用。Activity 在 `super.onCreate` 前仅提交准备任务；assets 读取、校验、复制及目录切换全部在工作线程执行。Activity 重建只重新观察同一准备状态，不重复安装；准备任务只捕获 applicationContext。准备状态为 idle/preparing/ready/error，等待者只在 ready/error 时收到终态；失败后仅显式重试启动新尝试。

`ResourcePreparationPlugin.waitUntilReady` 是 Rust 内部原生桥接，不增加前端可调用命令或系统权限。等待通过异步回调完成，主线程不做阻塞等待。帮助索引、内容、全文检索、RAG 指南检索/重建，以及插件市场列表、安装、更新和注册表刷新在读取内置资源前等待当前准备结果。安装等待位于已有可取消任务中，取消语义保持。已安装插件操作及其他不依赖内置资源的功能不等待。

Android PluginManager 初始化始终保存 `app_resources/SoloSoul_plugin_market` 的固定路径；构造管理器不读取市场注册表，不会因为后台准备尚未创建目录而永久退到空市场。setup 不再把 Android assets URL 或准备中的目录误报为缺失。桌面目录定位流程保持原有行为。

准备失败时 Activity 显示包含「重试」的原生 Snackbar；等待中的消费者收到稳定失败信息，窗口仍可使用。重试后新的等待者等待当前尝试，成功后正常读取完整树。消费者不读取 stage/backup，也不在 pending 时读取旧版本目录。

原生回归须注入真实文件读取延迟，验证主线程持续绘制、普通系统查询正常、帮助/市场保持 pending、Activity 重建后准备次数为 1，释放复制后消费者恢复；另验证失败提示可见且实际点击重试成功。Trace 使用 `SoloSoul.resources.enqueue` 和 `SoloSoul.resources.install` 区分主线程提交与工作线程安装。首次/重复启动计时单列，不将单次模拟器观测作为性能基准。

Tauri 最后一个 Activity 销毁会影响同进程 instrumentation runner 的收尾。本项目沿用主题回归的外部逐用例执行方式：保留 Activity 至 JUnit 报告结束，设置 `waitForActivitiesToComplete=false`，然后由外部驱动 force-stop 专用应用。Activity 重建仍在用例内执行且保持全部断言，不能用此收尾方式代替重建验收。
