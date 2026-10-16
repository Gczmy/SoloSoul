# RF-312 独立原生观察器

本目录公开当前观察器的原样Java／Manifest源码及参数化离线构建入口。观察器插桩自己的独立包，读取指定公开夹具应用与系统文档选择器；构建命令只生成本地测试产物。

本轮实际编译、签名和Manifest核验通过，生成DEX与冻结观察器完全一致；含空格输出路径通过，已有输出目录与缺失SDK均拒绝。原生完整行程仍待验收，见[执行记录](../RF-312-complete-default-entry.md)。

## 构建

准备Python 3、包含`java`／`javac`／`keytool`的JDK，以及Android SDK的API36平台和build-tools 35.0.0。替换下面的本机目录参数，在新输出目录执行：

```sh
python build_observer.py --sdk "<SDK目录>" --jdk "<JDK目录>" --output "<新的本地输出目录>"
```

可通过`--compile-sdk`和`--build-tools`指定已安装版本；当前实测为API36／35.0.0。脚本完成Java11编译、DEX、APK链接／对齐、生成内部测试签名、签名和内容回读；不会下载SDK或安装应用。产物包括APK、`package.json`和各步骤日志，测试签名材料保留在本地输出目录。

`package.json`记录实际源件、DEX、APK和签名摘要。不同测试签名会改变APK字节；源码和DEX摘要用于核对同一程序。应用的当前默认Release、恢复用Debug／测试包及公开合成Vault由完整行程入口另行准备。

## 协议与边界

观察器包名与instrumentation目标均为`org.solosoul.rf312observer`。只在专用公开测试AVD使用；调用方须先核验设备、APK、包缺席及公开数据，运行后按完整清单恢复。当前完整行程编排仍为本地冻结配方，尚需参数化公开。

启动参数`token`为新的32位小写十六进制命名空间。调用方在该包自己的`files/<token>/`目录按序原子发布`request-001.json`等文件，读取相应响应；`ready`表示连接就绪，`stop`／`stopped`完成正常停止。

| 请求 | 原生条件 |
| --- | --- |
| `snapshot` | 层级仅来自指定应用或文档选择器；其他根返回空层级，截图拒绝 |
| `inspect-target` | 只读核对唯一目标、焦点／IME及滚动或公开文件卡片条件 |
| `tap` | 精确唯一、可见、启用、可操作且在屏幕内的节点；单手指DOWN／UP |
| `type-public` | 仅空白聚焦输入框与两种固定公开合成文本，密码语义匹配 |
| `scroll` | 精确唯一可滚动节点，原生屏幕裁剪后的有界单手指手势 |
| `pick-public-file` | 指定公开文件命名空间、可聚焦原生卡片、唯一独立预览控件且触点避开该控件 |
| `back` | 仅指定应用或选择器的前台根 |

输入目标逐项匹配`package`、`class`、`text`、`content-desc`、`password`和`bounds`，执行前重新查询。观察器保持240秒／512请求及5000节点／200层深度上限；调用方还须保留45秒阶段期限、15秒就绪和10秒停止期限。构建成功不代表设备交互或性能采样通过。

## ADB 调用记录

`adb_control.py`提供`AdbRecorder(executable, serial, output)`。调用方先完成专用模拟器、包身份及公开夹具检查，再通过`run(*command, timeout=...)`执行原命令；它保留固定`127.0.0.1:5057`连接与显式设备，不重试、不重启服务，也不改变客户端启动方式。设备、输入、恢复及完整行程的门禁继续由调用方负责。

超时会先记录本次命令、原请求期限、实际耗时、`exitCode=null`和可取得的原始部分输出，再抛出同一个`TimeoutExpired`。成功及非零退出维持原结果语义；若保存证据失败，原超时仍是主异常。命令及部分输出可能带有本机信息，只保存在调用方指定的本地目录，不能直接作为公开摘要。

五项回归覆盖部分输出、异常保留、成功／非零退出、无界期限拒绝和证据写入失败。将当前进程的临时目录指向专用测试盘后，可在本目录运行：

```sh
python -B -m unittest discover -s . -p test_adb_control.py -v
```

## 正文与底部导航边界

`body_navigation.py`仅识别当前英文公开夹具的原生`Main navigation`容器，核对归属、可见／启用状态、容器类型、四个唯一可点击导航项及其边界。页面标题中的Home／Objects／Tools／Settings不参与判断；缺失、重复或不匹配时返回未就绪，调用方继续受原阶段期限约束，没有坐标或标题回退。

八项回归覆盖实际标题碰撞、容器／按钮歧义、缺失、错误归属／状态和越界几何。留存层级的只读复现已从0个恢复为1个正文候选，不派发输入；原生命中、焦点、手指事件及原预算由原入口继续验证。

```sh
python -B -m unittest discover -s . -p test_body_navigation.py -v
```
