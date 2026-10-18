# RF-312：观察器构建回执与运行入口对齐

## 问题与修复

公开的`build_observer.py`实际编译成功，却只写出`apkSha256`，缺少既有`runtime_inputs.py`和`observer_session.py`要求的`sha256`、`instrumentationTarget`。将真实生成的回执直接交给原准备器，会以`Verified separate observer artifact required`拒绝，不能进入完整队列。

构建端现在写出实际APK的`sha256`和`bytes`，保留同值`apkSha256`。`package`、`instrumentationName`及`instrumentationTarget`在源码与编译后Manifest身份一致后写出：根包必须为`org.solosoul.rf312observer`，唯一直接子插桩为`.Observer`且目标为自身。原字符串包含检查可能被根包中的自身名称误导；新检查将属性绑定到实际节点，拒绝不同目标、多个／嵌套插桩、错误命名空间、重复属性及歧义字符串。

运行端、读取器和观察器Java／Manifest源码不变。编译、签名、签名前后内容比对、源件哈希、每步120秒以及新目录要求保持。

## 使用

按[构建命令](rf312-default-observer/README.md)在全新目录生成APK和`package.json`。在[显式运行配置](RF-312-runtime-inputs.md)中，将`observerPackage`指向该JSON、`observerPackageSha256`设为其实际SHA-256，并把`artifactPaths.observer`指向回执的`apk`。其余产物、源件和夹具仍按原配置完整绑定。

`runtime_inputs.py --config ... --output ...`可直接读取这份原样回执；成功准备后再按[完整环境队列](RF-312-portable-owner.md)执行原生验收。回执的`productInstalled`及`nativeRuntimeAccepted`仍为false，不能凭构建结果宣称原生通过。

## 本轮验证（2026-10-17）

- 原构建端先真实生成APK，旧回执被原准备器拒绝，非通过结果与原始日志保留。修正后在含空格的新目录真实重建；签名、编译后Manifest和APK内容检查通过。
- 新回执不加转换进入原准备器，1,881项产品输入完整保留并复验，分别为工作区1,843项和独立前端38项；二十份运行源件哈希与此前原生队列完全相同。再次用原`load_inputs`复验通过。原会话的产物断言也通过，测试在第一次SDK读取边界主动停止，没有开始会话或安装应用。
- 新旧观察器Java／Manifest和APK内DEX相同，DEX SHA-256为`c841d6dcb3a7931ab7055ccc7b308b5eee36327e6470fad2fca812997ac172b4`；内部新测试签名使APK摘要发生预期变化。
- 另用真实`aapt2`编译一个根包正确、插桩目标为其他包的反例APK：旧字符串条件会放行，新编译后Manifest检查拒绝。错误源码目标在任何SDK工具执行前拒绝；重复输出目录拒绝且已有回执字节保持。
- 完整同库258项唯一回归通过、零跳过，包括本次14项。首次本机复现包装器误把预期退出码写成2而非1；首次新单元夹具将恢复包与待篡改观察器共用文件，导致错误断言。两份失败记录保留，分别由新运行和独立夹具修正，不改写旧证据。

去标识摘要见[机器可读结果](rf312-observer-contract-2026-10-17.json)。APK、测试签名、原始日志、输入清单及反例包只保留本机。

## 未完成范围

本轮没有设备命令、界面输入或新的性能样本。产物生成链其余部分、同源重复分布、全行程峰值／观察器开销、实际Tauri IPC、启动两次提交帧和多端验证仍待完成；不关闭RF-312。
