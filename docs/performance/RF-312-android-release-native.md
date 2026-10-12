# RF-312 Android Release 原生首页功能门禁

2026-10-12，基线 `a664c0730`。接受同一份未修改的优化、非调试 x86_64 Release APK 在专用 API31 模拟器上的首页功能基线；**未接受 100/5000 对象性能、真机性能或完整 RF-312/RF-121**。

| 项目 | 实际结果 |
| --- | --- |
| Release APK | 2.13.2 / 2013002；78,626,143 字节；SHA `20a99626cb9022524beee24d9578d97ee2f97f11ac744e0d368a96e8a15451a5` |
| 设备 | 专用 `SoloSoul_RF121_API31_20261010`；API31、x86_64、WebView 91.0.4472.114；不连接真实手机 |
| Release 原生用例 | 支持玻璃、回退各 1 项，2 通过、0 失败、0 跳过 |
| 同条件 Debug 对照 | 原 Debug APK、相同独立测试包和原样首页方法，1 通过、0 失败、0 跳过 |
| 每个成功用例 | 12 阶段：引导及公开临时账户、浅深色首页、增强玻璃、锁定/密码解锁、真实加密偏好、对象编辑/列表/详情/菜单、系统返回和有内容首页 |
| 图像 | 33 PNG 解码/尺寸/非均匀图像及 SHA 回读；人工查看 Release 浅色支持模式、深色回退解锁后 2 张，未称全部人工验收 |
| 恢复 | 每轮原主包/测试包字节、UID、debug 标记一致；216 项私有内容/链接/权限及私有根权限一致；6 项系统设置/有效配置一致；独立 probe 包已移除 |
| 来源 | 1,839 项当前源码及原 APK 哈希保持；首页 Kotlin 方法原字节不变；生产源码、Release 库和 APK 没有修改或重新编译 |

## 与 Release 匹配的测试方式

原 Debug instrumentation APK 依赖主包提供 `androidx.tracing.Trace`。实际 DEX 定义回读：原 Debug 主包有这个原类名；原测试包和优化 Release 主包都没有。首次两轮在运行器创建阶段 `NoClassDefFoundError`，随后按原 240 秒命令上限结束，执行设备测试 0 项。不能将其算作产品首页用例失败，更不能通过把 Release 改成 Debug 验收。

D 盘独立 Gradle 项目使用原样 `AndroidHomeInstrumentedTest.kt`，将测试运行器依赖作为自己的实现依赖。生产 `MainActivity` 和 Tauri 的真实已编译类仅作 compileOnly；DEX 复核未打包这些生产类或原生库。仅用已有测试密钥签名，证书与目标相同；未使用生产签名密钥。

第一版独立 probe APK 虽能运行 JUnit，但 Release 两轮与 Debug 一轮均在原 45 秒引导预算下失败，设备测试合计 3 项失败、0 跳过。对照发现它比原 instrumentation 多出自动初始化 Provider、Startup/ProfileInstaller 运行时和不同 manifest 设置。仅对齐测试包环境后，同一首页源码、相同断言/预算、相同 Release 字节的两轮及 Debug 对照均通过。该组对照证明测试环境影响结果；没有把多项同时对齐的结果唯一归因某一个 Provider 或设置，也没有登记产品启动缺陷。

独立测试 APK 的 `applicationDebuggable=true` 属于 probe 包；逐次读取目标主包确认 Release 的 `applicationDebuggable=false`。测试通过生产 UI 创建公开临时账户，不替换认证 store 或直接注入解锁状态。保留真实 IPC、加密偏好、最终截图文字像素和原断言。测试配置背景可参照 [Android 官方高级测试配置](https://developer.android.com/studio/test/advanced-test-setup)；本轮实际方式及配置以归档源码和设备结果为准。

## 安装、恢复及失败保留

原私有目录先在已确认空账户的原 Debug 包上完整备份。Release 不支持 `run-as`；收尾先重装并按字节/UID/debug 标记核验原 Debug 包，然后恢复完整 TAR，再逐项核验私有根、所有内容/链接/权限和系统状态。原测试包不替换；新增独立 probe 包在核验确为本次构建后卸载。备份不入 Git。

- 首次安装前路径预检拒绝 1 次，实际安装/设备测试 0；修正 Android `~~` 路径及 canonical `pkgFlags` 读取，原拒绝保留。
- 原运行器两次失败及第一版 probe 三项设备失败完整保留；未作为成功样本，未放宽任何原预算或断言。
- 独立 probe Gradle 六轮真实退出码 `[1,1,1,1,0,0]`：AndroidX 开关、runtime 缓存、compile 缓存、原用例 WebKit 引用四次失败；两轮构建成功。正常仓库补全固定版本缓存，分别 37/55 个依赖文件长度/SHA 回读；原生产依赖没有改动。
- 第六轮 Gradle 已成功，外层校验遇到既有日志文件名并退出 1；新名称独立完成包/签名/DEX/manifest 校验，保留外层失败，不将其改为 0。
- 八对备份/恢复逐项核验后，自有模拟器按 PID、出生时间、路径和命令核验并正常退出；端口释放，无强制终止。证据归档核验后精确删除 16 份已无恢复用途的私有 TAR 副本；两次清理预检均在删除前拒绝，修正数组/原生 DirectoryInfo 遍历后清理成功，所有失败源码/记录保留。

## 复跑与下一步

[完整结果](rf312-android-release-native-2026-10-12.json)、[335 份逐字原件索引](rf312-android-release-native-2026-10-12/index.json)、[原件归档](rf312-android-release-native-2026-10-12/raw.tar.gz)、[精确清理回执](rf312-android-release-native-2026-10-12/cleanup.json)包含所有实际成功/失败、构建/来源、运行器、完整驱动和恢复合同、独立复核及图像。归档排除 APK/JAR/原生库/密钥和私有 TAR；这些程序产物在 D 盘内部验证目录保留，完整产物准备见 [上一阶段](RF-312-android-release-preparation.md)。归档创建时 `taskClosed=false`、`commitCreated=false` 是时点事实；提交由后续 Git 回读证明。

复跑时使用新的结果目录及已命名专用 AVD，先核对原安装包、无既有账户、源码/产物/测试密钥证书。从归档中按顺序复用启动、已对齐 probe 构建与校验、原 Debug 对照、Release 驱动、独立复核及完整恢复；驱动中的旧目录不可重复使用。不得直接将 Release 交给仅恢复数据的原 Debug 驱动，或复用未经对齐的旧 probe。已删除 TAR 不再用于事后重放；后续运行必须创建新的完整备份。

该门禁包含公开临时账户与一个对象，**不是** 100/5000 合成 Vault 采样。下一步建立 Android Release 的固定数据测量入口，执行重复启动、解锁、搜索、OCR/预览、锁定恢复、内存和 IPC 指标。Windows 成对启动仅已有部分证据；Windows 原生材质/恢复、API35 WebGL、macOS/iOS 和真机范围保持原状态。关闭数仍 310/322，6 待验证、6 待执行，goal active；本阶段独立本地提交，不推送。
