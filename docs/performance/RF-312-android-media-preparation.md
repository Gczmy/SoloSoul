# RF-312 Android媒体夹具路径准备（部分验收）

接受当前源码的独立夹具导出/重开校验；100/5000对象、全部固定对象内容、四个加密素材及Android路径均已验证。尚未注入设备或执行OCR/预览，不关闭RF-312、RF-121、RF-1109，也不将本项计作原生性能样本。

## 接入范围

仅修改`tauri/crates/solosoul-core/examples/perf_baseline/media.rs`。普通Core接口、主APK、默认功能、依赖和锁文件不改。原Windows媒体v2合同与消费者保持，新增独立Android包装清单，普通Windows/native_manifest入口必须拒绝它。

目标私有根固定为`/data/user/0/com.solosoul.app`，已在本轮专用模拟器通过真实`run-as pwd`确认。此准备入口适用于这个包和user0根，不能据此宣称其他用户/包路径也已验证。

导出先复用完整原夹具复制与校验，再仅在新副本中改写四条加密对象附件元数据路径；移除副本原完成标记，直到全部对象/加密素材/源未变检查通过才发布Android标记。路径用Unix分隔符构造，文件名和UUID仍由原严格合同验证。原目录仅读，拒绝已存在输出、输出位于源内、未知文件、reparse、错误摘要、明文附件、改变对象内容。

Android标记仍名为`rf312-media-fixture.json`，外层`schemaVersion=1`、`scope=synthetic-android-media-vault-fixture`、`metadataVaultRoot`及`media`，内层媒体v2含账户/KDF/对象/素材/封闭文件证明。Android验证只接纳此封闭包装；旧主机标记或篡改根/未知字段均拒绝。

## 实际验证

- workspace fmt退出0；例子定向9项通过、0失败/忽略；严格Clippy退出0；当前源码Release生成器实际构建退出0。
- 同一模块在Windows非默认native-perf库消费者中严格Clippy退出0，相关10项回归通过、0失败/忽略；不是新的Windows GUI运行，未重跑未改动的材质矩阵。
- 全新一整组100/5000各导出、独立重开验证退出0，真实生产KDF65536/3/4、全部对象内容/搜索结果ID及四个素材解密字节/类型/路径完整检查通过；输出验证前后字节保持，原两份主机源逐文件摘要保持。
- 两种数量的主机验证Android标记、Android验证旧主机标记分别真实退出1，共4个负向门禁；未作为跳过或失败样本过滤。
- 首轮100导出/验证真实退出0，但汇总器因Windows ACL日志位于JSON之前而退出1，5000未执行。源码/输出/日志保留；修正为唯一结果JSON并严格核对scope/success/nativeUiMeasured后，在新目录重做整组，不拼接旧结果。
- 增量缓存硬链接失败警告原样保留，编译器回退为复制；未修改系统权限或为此重启编译。

[结构化结果](rf312-android-media-preparation-2026-10-12.json)和[87份逐字原件索引](rf312-android-media-preparation-2026-10-12/index.json)绑定当前源码、真实构建/命令/退出码和两份清单。所有成员SHA/长度回读通过；程序、数据库、缓存和密钥不入库，公开加密副本保留本机供下一步验证。

## 复跑

在`tauri/`使用现有已配置工具链和缓存运行：

```text
cargo test --locked --offline -p solosoul-core --example perf_baseline media::tests -- --test-threads=1
cargo clippy --locked --offline -p solosoul-core --example perf_baseline -- -D warnings
cargo build --locked --offline -p solosoul-core --release --example perf_baseline
```

当前构建结束后，设置`SOLOSOUL_SECURE=1`，使用该生成器执行：

```text
perf_baseline --copy-android-media-fixture <已验证主机媒体源绝对路径> --media-fixture-output <全新主机输出绝对路径>
perf_baseline --verify-android-media-fixture <输出绝对路径>
```

具体D盘缓存/临时目录/运行程序SHA和实际命令在原件中。stdout含正常Windows ACL日志，不能直接把整个stdout当JSON；应提取唯一结果对象并核对严格scope和完成标记。

下一步使用当前RF-1134共享恢复helper、专用模拟器/原包字节UID/完整私有备份、唯一标识和真实原生输入，将Android副本接入设备；先核验图像/文本/PDF预览，再验证图片首OCR（Android扫描不支持PDF）。需要真实绘制/内容/返回/锁定恢复证据，不能只看文件读取成功或embed元素存在。renderer/全进程内存、进程冷启动、物理硬件和其他平台范围继续保留。当前313/324关闭、5待验证、6待执行，goal active；此项独立本地提交、不推送。
