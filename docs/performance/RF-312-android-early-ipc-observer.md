# RF-312 Android 早期 IPC 传输观测入口

仅接受非默认 Android 测量入口及完整生产首页行为门禁。固定 100/5000 数据的性能行程、重复时延分布、内存、物理 ARM 与跨端范围仍须继续，RF-312 不关闭。

默认功能仍为 `custom-protocol`，新增无依赖的 `android-native-perf` 仅支持 Android。入口通过 Tauri 既有 `append_invoke_initialization_script` 在生产页面脚本前安装；保留框架只读 `invoke`、IPC 鉴权键、窗口、认证与生产首页代码。

## 计数口径与保护

统计主文档实际 Java bridge 发送尝试与 channel POST fetch，含重试/通道请求；不是逻辑 invoke 次数或完成数。记录仅包含命令名、传输类别及文档时钟。桥接只扫描 cmd 首字段至结束引号，既不解析请求体、选项/密钥，也不读取响应体。无效命令脱敏为 null，失效原因固定。原参数、原 Java 接收者、同步返回/异常与 fetch Promise 保持；普通 fetch 方法 getter 不额外读取。

Java host 的描述符虽然显示可写，真实方法赋值会被忽略。测量构建仅将可写 `window.ipc` 别名替换为委托对象，保留原 Java host/method，正常调用转发给原接收者；不修改只读 Tauri 内部函数。不可修改的别名或 fetch、额外 frame、包装器替换、文档时钟变化、未知命令或 16384 条溢出均拒绝完整计数。32 位十六进制测量标识绑定一次，保留绑定前的启动发送记录。

## 实际验证

- 默认 Windows workspace fmt、严格 Clippy、全量测试退出 0：1874 通过、0 失败、3 项既有忽略。后续仅改 Android 专属 JS/Node 文件，已检查的 Rust/Cargo/Kotlin 输入保持；不把默认 Windows 检查当作 Android 功能运行。
- Node 12 项通过、0 失败/忽略；包含实际 Tauri 后定义 readonly invoke 的顺序、两项诊断隐私失败、普通 fetch getter 转发以及 Java host 忽略方法赋值的失败回归。
- 独立测试 APK 复用未修改的 `AndroidHomeInstrumentedTest` 完整 12 阶段，实际主应用为非调试优化 x86_64 Release，API31 专用 AVD。增强/回退两组各执行 1 项、0 失败/跳过，传输发送总数分别为 259/266，实际密码登录已计入，原只读 invoke 保持。
- 默认 Release 缺少入口的负向对照：原首页完整通过后发生预期 1 项真实失败，未把失败改成跳过。
- 方法直接包装版本两组各 1 项真实失败，仅计到 2 次 fetch；首页行为各自通过，完整私有目录/包/系统恢复和 22 张 PNG 独立核验通过。原生 Java 委托诊断先因未注册命令失败并保留；换成代码中核实的 `ui_get_preferences` 后 1 项通过，直接方法捕获 0 次、委托捕获并完成 1 次。
- 初始化顺序构建的主动取消、队列前置文件缺失、准备脚本引用次数拒绝、全部构建/签名/运行退出码分别保留。未用观察超时重启运行中的命令；内部 unsigned 校验退出 1 是预期结果，签名只用已有测试证书。
- 两组正向完整原 APK 字节/UID、私有目录所有内容/权限及系统原值恢复通过；自有 AVD 正常退出，无强杀。程序、私有 TAR 和密钥不入库。

[结构化结果](rf312-android-early-observer-2026-10-12.json) 与 [逐字原件索引](rf312-android-early-observer-2026-10-12/index.json) 保留每次来源、失败和真实配置；不能将本页接受范围扩展为全部性能基线。

## 复跑

`node --test src-tauri/src/android_native_perf/observer.node.test.mjs` 在 `tauri/` 执行。真实 Android 库构建需现有 SDK/NDK/JDK/Rust target，使用 `cargo build --locked --offline -p solo_soul --lib --release --target x86_64-linux-android --features android-native-perf`；本轮实际 linker、flags、优化参数及工具 SHA 在原件中，不省略环境前提。源码冻结与原 Debug JNI/旧 APK 校验必须先完成。

库命令实际退出 0 后才能用该库打包、签内部测试证书，再运行独立 `AndroidEarlyIpcObserverInstrumentedTest#productionHomeWithEarlyIpcTransportObserver`。驱动必须绑定专用 AVD/原包/完整私有备份/恢复及唯一 evidenceTag、observerRunId。不能直接用仅适用 Debug run-as 的旧驱动覆盖 Release。

后续固定数据测量保留本入口，另行定义并执行完整启动、解锁、搜索、workspace、OCR/附件预览、锁定恢复、内存与重复样本；不能将此首页门禁的单个公开对象和发送数替代 100/5000 性能行程。
