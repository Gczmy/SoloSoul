# RF-312 Android Release 产物准备

2026-10-12，本机完成当前源码的 x86_64 Android Release 库和 APK 准备。
只接受构建产物准备，尚未安装或运行应用；不是实体设备性能基线。

Rust 原日志明确 `Finished release profile [optimized]`，耗时 48m 07s。
实际主应用 rustc 参数为 x86_64-linux-android、opt-level=3、codegen-units=1、strip=symbols；
工作区配置另声明 lto=thin，本次截取的 rustc 参数不单独证明 LTO 模式。
Release ELF 为 59,432,984 字节，ET_DYN / 64 位 / machine62，SHA
`fc37bde0578b82825df7790bf362be41339043a85b938119d72667d382b5071b`。

原 Tauri CLI 在编译完成后因 Windows 无符号链接权限失败，exit1；删除了生成 JNI 文件。
外层恢复守卫另拒绝 Cargo 换行和 tauri.properties 末尾换行变化，finally 读取缺失 JNI 也失败。
原错误和退出码保持。核对纯格式差异后恢复全部1839项输入字节，并从构建前 SHA 验证的副本恢复
原生成 Debug JNI；临时全局 Gradle init 已精确移除，旧主／测试 APK 保持。

将已完成的同一 Release 库以普通文件副本交给 Gradle，未重新编译 Rust。
五次独立打包退出码1/1/1/1/0：前两次缺少既有固定 Runtime 依赖，第三次缺少既有
lint-gradle31.11.0工具，第四次未签名临时设置被项目签名配置覆盖，第五次在AGP finalizeDsl
阶段明确生成内部准备用未签名包后通过。Release Lint、R8、资源压缩和ABI限制保留，生产签名配置不改。

首次依赖辅助任务在内部buildSrc项目选择处失败；修正主项目选择后原五个固定组件解析成功。
显式AAR缓存仍未满足Release配置，随后按真实releaseRuntimeClasspath重新解析相同五项，
整个组件版本图前后完全一致。另按AGP已要求的lint-gradle31.11.0获取正常工具依赖，66份文件核验。
一个JAR路径长272字符，普通Python路径读取失败，但同一Windows扩展路径2199字节/SHA均匹配；
系统长路径策略和缓存未手工改写。三次普通读取失败原件保留；其中一次属于fetch外层，另两次为
独立缓存核验，原元数据计数混合范围由补充说明校正。初次恢复日志读取还因ANSI颜色控制符被拒绝，
仅在读取时移除终端格式后确认原release完成文本，日志原字节保持。

APK包名com.solosoul.app、版本2.13.2/2013002、ABI仅x86_64、minSdk28/targetSdk36、
application-debuggable标记缺失，原未签名验证按预期exit1。
最初包验证工具使用旧字段sdkVersion而拒绝；aapt2实际minSdkVersion，修正后通过，未重打包。
使用已有专用debug/test密钥只签内部验证副本，证书与冻结instrumentation APK一致；未访问发布密钥。
签名前后所有原ZIP成员逐字节一致，仅允许正常签名成员新增。

内部验证APK为78,626,143字节（约75.0MiB），SHA
`20a99626cb9022524beee24d9578d97ee2f97f11ac744e0d368a96e8a15451a5`。
文件保存在本机D盘独占构建目录，未公开APK、库、工具JAR/AAR或keystore。
原件归档核验后仅删除六份冗余库副本，原生成Debug JNI、Release库和两个APK保持。

下一步在有完整备份／恢复和APK身份门禁的专用AVD上补Release原生运行，再建立100/5000对象
实际测量入口；运行后需恢复原Debug APK及完整数据，不能将Debug的UI结果冒充Release结果。
R8记录了proguard-rules.pro:19的字段规则警告，保留待独立复核，当前未为消除警告改变冻结产物。

[完整结果](rf312-android-release-preparation-2026-10-12.json)、
[121份原件索引](rf312-android-release-preparation-2026-10-12/index.json)、
[计数范围补充](rf312-android-release-preparation-2026-10-12/tool-outcome-count-clarification.json)和
[精确清理回执](rf312-android-release-preparation-2026-10-12/cleanup.json)保存全部失败、构建/恢复、
固定依赖/长路径实证和内部签名。RF-312完整要求、RF-121和后续依赖保持未完成，关闭数不增加。
