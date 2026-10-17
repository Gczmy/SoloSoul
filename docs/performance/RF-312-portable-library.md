# RF-312：从已绑定前端生成默认 Android Release 库

本阶段消费[隔离前端](RF-312-portable-frontend.md)的完整回执和物化源码，在新的Cargo目标目录编译`x86_64-linux-android`默认Release库。源码见[构建入口](rf312-default-observer/library_build.py)与[阶段交接验证](rf312-default-observer/build_payload.py)。它不安装应用，也不把编译成功当作原生或性能验收。

## 前置输入

在Windows准备Python 3.11以上、已安装Android目标标准库的Rust MSVC工具链、NDK以及锁文件所需的本地Cargo缓存。当前构建配置使用`custom-protocol`默认功能、Release、API28链接器，原生观察器等非默认功能不能加入。

前端回执中的物化源码、生成资源和38份前端文件必须保持原字节。产品／插件快照逐份验证，完整资源版本重新计算；后来仅修改文档的提交不会使这个独立副本失效。交接不读取已经移动HEAD的原工作区或原Node依赖路径。

## 配置与运行

保存本机配置，所有摘要和字节数来自实际输入，不使用示例值：

```json
{
  "schemaVersion": 1,
  "frontendResult": "<已通过的frontend-result.json>",
  "frontendResultSha256": "<该回执SHA-256>",
  "sdkRoot": "<Android SDK>",
  "ndkVersion": "27.0.12077973",
  "toolchainRoot": "<rustupHome/toolchains/工具链目录>",
  "rustupHome": "<Rustup根>",
  "cargoHome": "<含锁定依赖源和.crate的Cargo根>",
  "toolSha256": {
    "cargo": "<cargo.exe SHA-256>",
    "rustc": "<rustc.exe SHA-256>",
    "rustcDriver": "<唯一rustc_driver*.dll SHA-256>",
    "linker": "<x86_64-linux-android28-clang.cmd SHA-256>",
    "clang": "<clang.exe SHA-256>",
    "ar": "<llvm-ar.exe SHA-256>",
    "ndkProperties": "<NDK source.properties SHA-256>"
  },
  "targetStdFiles": {
    "<每一个目标标准库文件名>": { "bytes": 0, "sha256": "<实际SHA-256>" }
  }
}
```

Rust程序位于工具链`bin`，Android工具位于`ndk/<版本>/toolchains/llvm/prebuilt/windows-x86_64/bin`。`targetStdFiles`须完整覆盖工具链`lib/rustlib/x86_64-linux-android/lib`，每一项记录实际大小和摘要；0和占位符不能作为真实输入。SDK文件应先完整本地化，再捕获其稳定大小／SHA；读取期间发生身份变化仍会拒绝，不屏蔽属性位。

```powershell
python -B docs/performance/rf312-default-observer/library_build.py `
  --config '<本机配置.json>' --output '<新的普通输出目录>'
```

入口固定执行`cargo build --locked --offline -p solo_soul --lib --release --target x86_64-linux-android`，另加JSON诊断格式。目标、临时及ORT缓存均在新输出内，构建并发固定一；Cargo/Rustup输入缓存显式提供。继承的编译包装器、特性／profile覆盖和Kotlin代码注入变量被清除，配置只覆盖相对`frontendDist`。

## 实际产物门禁

- Cargo必须正常结束，且报告唯一、实际新编译的产品库，源路径与隔离副本一致；实际功能集合为`default`及`custom-protocol`，优化等级3、非测试且无debug断言。
- ELF必须是x86_64小端共享库，不含原非默认Android观察器标记。实际生产HTML中的脚本、预加载和样式入口必须出现在新库中；不能用旧库替代。
- Kotlin、ProGuard和Gradle元数据由本次Cargo正式生成。缓存`.crate`整包SHA须等于`Cargo.lock`，所用生成器源码／模板逐项对照包内原文件；不依赖可选的`.cargo-checksum.json`。生成桥接全文摘要与官方模板替换结果匹配，Gradle依赖路径及implementation集合与锁定库一致。
- 结束后重新核对完整前端物化输入、工具、目标标准库、生成器和祖先Cargo配置；复制出的`.so`也须逐字核对。失败保存为非通过回执和原始输出，不覆盖旧目录或旧产物。

`library.json`记录编译器原始功能／profile、输入及生成桥接的摘要。后续Android打包仍须核验APK的资源、原生库、非debug属性、签名及完整原生行程；本阶段不关闭RF-312。

## 本轮结果：编译成功，结束后输入复核拒绝

301项唯一回归（新增20项、零跳过）通过。真实离线Cargo在新目标目录编译完成，退出0、用时2929.75秒；结束后的完整物化输入复核报`Input changed while hashing`，入口整体退出1，`library.json.passed=false`。**默认库生成尚未验收，不把Cargo退出0等同于整体通过。** 本次没有写出入口的最终复制库，实际产物保留在原Cargo目标目录；失败回执及原日志不改写。Cargo计时不是应用性能数据。

独立取证检查确认实际产品库新编译、功能为`default/custom-protocol`、优化等级3、非测试且无debug断言；ELF59,454,952字节，摘要`ce850c34438306d15eeab407eb2d146b45c1796c9240083b0124057f7354edfe`。38项生产前端资源键均实际出现，10份官方桥接、6个依赖映射、完整输入内容、55项标准库及7项工具匹配。取证读取显式使用独立本机句柄草案，**只用于定位与核对字节，不替代原入口或取得生成验收**；原20份运行时源码保持。

原严格读取器的追加诊断仍拒绝：打开一份模型归档时，属性由`0x40020`变为`0x20`，设备／文件身份、长度、写入时间及mode保持。公开PDF逐步诊断同样定位到打开阶段；提供方原因未证明，也未屏蔽属性位。原三个SDK准备拒绝及两次CLI负向检查记录保留；缺项标准库CLI被更早的属性保护拒绝，不计专项CLI通过，缺项／额外项／内容变化和工具变化的configure回归独立通过。

后续将单独修正输入冻结时点：打开普通文件后绑定路径与句柄，分别冻结完整mode和属性，哈希期间仍拒绝变化。首个本机草案因批处理文件两种视图的mode差异拒绝；修订草案8项定向回归及1801项原源码对照通过。依赖诊断完成22687项Node文件逐项对照后，汇总在插件物理树多出Git控制文件处拒绝，整次诊断不算通过；108项跟踪插件源码另行对照通过。上述草案均未改动本次正式生产者。生产者变更后须新建前端回执并重跑库生成，不能重标本次失败。

本轮未构建新APK、未安装应用或发送设备命令，不追加界面／性能样本。后续包应从当前89项资源及清单生成90份应用assets，隔离复制6个Android依赖并验证两份项目补丁；旧冻结包多出的8份文档和旧保全读取器原样保留。完整新生成链、原生候选验收、重复分布、全程内存／同源观察开销、实际IPC、启动两帧和多端仍待，不关闭RF-312。见[去标识摘要](rf312-portable-library-2026-10-17.json)。
