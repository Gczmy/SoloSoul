# RF-312 Windows 认证阶段原生诊断

本轮用于区分旧首页超时发生在输入触发、IPC 返回、前端等待、账户刷新、状态提交还是后端解锁阶段。必要代码检查已完成：默认 Rust workspace 1861 passed/0 failed/3既有ignored，非默认Rust 99项、Core观察器2项，完整Vitest 256文件/2244项、CLI 336 passed/0 failed/2既有ignored、Node最后复验214 passed/0 failed/1既有权限skip（新增真实SDK模式边界回归后）。默认与非默认严格Clippy、类型、格式和新Vite构建均通过。新隔离Release已成功构建，100/5000各三次完整原生行程与独立时钟复算通过；原首页超时本轮未复现，根因未确定。

观测接口只接收固定阶段名，不读取请求/响应正文、密码、账户名或错误消息。fetch返回原Promise，Tauri-Response响应头只说明协议状态；实际invoke Promise完成由authStore在await之后记录。桥缺失或异常时不改变业务结果。

后端阶段包括root-read、maintenance-admission、sync-disable、blocking-queue、worker-vault-read、core-unlock，以及Core内的recovery、precheck、master-config、kdf、verify、account-write-call、kdf-upgrade、vault-open、session-publish、pin-reset-call。audit-call和cleanup-dispatch表示调用返回，不证明best-effort持久化成功或后台清理完成。running没有终结耗时；interrupted只记录到失败的耗时，不作为成功完成时长。

前端使用同一文档performance时钟；后端使用进程内Instant时钟，零点为第一次记录认证命令时创建的观察器，不是应用进程启动时刻。两者不相减。core-unlock包含KDF等内部阶段，嵌套时长不能相加。只有同命令的前后端记录数量一致且各自串行时才按顺序配对；并发时明确记为有歧义。

独立artifact绑定runId、PID、mainFrameId、loaderId和timeOrigin。采集在导航、子frame和进程守卫解除之前完成。未知字段、非法阶段顺序、将来时间、终结attempt之后的stage或未经核对的root均拒绝。正常IPC observer的旧字段不变。

实际采用同一新冻结Release、公开生产KDF 100/5000对象数据，各3次fresh root/profile，完整startup→password-unlock→workspace→search→application-lock→password-reunlock行程。原时限、IPC完整性、真实输入和cleanup要求保持，失败组保留，不拼接成功子集。带观测器的计时仅供阶段诊断，本报告performanceMetrics=null；底层runner输出不作为新的无观测器性能基线。

全量Node首次出现PowerShell mock 15000ms超时，失败记录保留。Rust检查链结束后，同一命令与原预算单独复验通过，符合资源竞争的可能解释，但未证明旧超时的完整根因。默认Rust第一次缺显式PDFium路径，18项PDF测试因DLL加载失败；按现有CI指定本机已核对DLL后整组通过，没有产品加载器/依赖/断言改动。Release最初的输入冻结器混入35个旧Vite产物，文件名变化导致编译前停止；现已分开冻结1314个当前源码/配置输入和35个新dist产物，旧失败脚本与记录保留。

state-set-done表示Zustand同步set调用和同步订阅返回，finished表示随后已安排原有备份提醒；两者不证明React渲染/绘制完成。GUI就绪仍由原有home探针和完整六阶段门禁决定。认证快照采集稍晚于最终GUI探针；各自IPC计数分别保存，对共同前缀核对同一命令序列，不假定后台请求停止。

真实运行还保留两类驱动失败：首次外部包装器给SDK传入不支持的sdk-ui模式，未创建GUI样本；新增调用真实SDK帮助入口的红测复现后修正为sdk-input，目标19项与全量Node214项通过。随后首个GUI样本因临时检查运行器继承单测PDFIUM_LIBRARY_PATH在隔离入口前退出，实际相同EXE明确复现固定拒绝码；只为GUI调用去除该继承，未放松原生守卫。两类旧失败原件保留，旧失败整组统计和cleanupIntegrity仍为失败。

源码冻结1314项中，仅上面两个外部Node包装器/回归文件在Release后变动；Rust、前端及其余1312项保持，新测量冻结另存，原Release冻结不重写。100/5000新组各3/3、每样本61次SDK调用、两条finished认证流通过；3个样本中的账户列表有并发，明确保留配对歧义，不拼接或跨时钟推算。

成功组fresh CIM核验54个有记录的PID/birth身份退出；旧启动失败组另核验两个缺少birth的PID已不存在，不冒充完整身份。只清理56个精确owned缓存目录、解除六个已核对链接，六份应用日志副本保留，61个完整Vault文件与全部证明/原失败状态清理前后逐字节保持，缓存工具没有终止任何进程。

账户写入调用在12次登录/再次解锁中耗时1803.06～5258.70ms，当前源代码含清单原子写和两次Windows权限命令；该范围是调用归因，尚不能把具体ACL调用认定为全部耗时或旧首页超时根因。KDF为127.61～198.92ms；状态写入为1.20～4.50ms，但不是React绘制耗时。下一本地步骤为账户写入内部归因和Windows RF-121完整窗口合成；RF-312及多端、睡眠与RF-112/121/122～127前置未关闭。

## 冻结与实际分布

新Release SHA-256 `85ce46809a74b4aa6d87045d0b646e1ba4845c80a08963cbf22b8d4700ecd492`，103510528字节。1314源码/配置、35新dist、94资源、185公开源和十个旧EXE保持。默认前端包含桥缺失时不记录的helper，不能声称默认前端字节完全不变。两档各三个全新owned root/profile，生产Argon2id 64MiB/3 iter/4 parallelism。

以下为中位 / P95，单位ms，每格n=3，P95为这三次最大值，包含观测开销。首次登录与再次解锁分开统计，带观测器的结果只用于归因，不构成新的无观测器性能基线。

| 阶段                  |      100 首次登录 |     5000 首次登录 |      100 再次解锁 |     5000 再次解锁 |
| --------------------- | ----------------: | ----------------: | ----------------: | ----------------: |
| 前端实际login await   | 2509.10 / 2617.20 | 3037.40 / 3105.70 | 2076.80 / 2470.90 | 2754.40 / 5405.20 |
| 后端login全调用       | 2490.97 / 2586.54 | 3008.81 / 3084.30 | 2050.91 / 2447.67 | 2709.52 / 5401.54 |
| 账户写入调用          | 2314.94 / 2452.40 | 2872.61 / 2936.15 | 1917.30 / 2256.06 | 2505.17 / 5258.70 |
| KDF                   |   143.06 / 161.49 |   130.90 / 137.60 |   132.65 / 186.21 |   138.10 / 198.92 |
| 登录后账户列表await   |     15.80 / 19.30 |     16.70 / 25.90 |     12.70 / 15.80 |     10.20 / 15.10 |
| 响应头到实际await完成 |       0.40 / 0.70 |       0.40 / 0.50 |       0.20 / 0.30 |       0.30 / 3.00 |
| 同步状态写入          |       2.20 / 2.40 |       2.20 / 4.50 |       1.30 / 1.40 |       1.40 / 1.90 |

## 复跑、原件与未完成范围

先准备[公开合成Vault](RF-312-native-vault-baseline.md)，构建 `cargo build --locked --release -p solo_soul --features native-perf --bin solo_soul` 并冻结当前源码、dist、资源和EXE。GUI测量环境必须没有 `PDFIUM_LIBRARY_PATH` 覆盖，输出使用新绝对目录，不重用consumed marker，也不并发运行内存采样。下列 `C:\rf312` 是另一份已准备并核验测试包的路径示例：

```powershell
cd C:\Users\40299571\SoloSoul\tauri
node scripts/native-perf-auth-attribution.mjs --exe C:\rf312\bin\solo_soul.exe --fixture C:\rf312\vault100 --output C:\rf312\new-auth100 --samples 3
node scripts/native-perf-auth-attribution.mjs --exe C:\rf312\bin\solo_soul.exe --fixture C:\rf312\vault5000 --output C:\rf312\new-auth5000 --samples 3
```

[结构化验收](rf312-windows-auth-attribution-2026-10-06.json)及[287份逐字节原件](rf312-windows-auth-attribution-2026-10-06/index.json)包含所有成功、失败、红绿回归、冻结和清理证据；不含EXE/模型/DLL、Vault物理文件或缓存树。旧首页超时、系统睡眠和完整多端同口径验收未完成；RF-312继续待验证，RF-112/121和RF-122～127前置保持。按本阶段独立本地提交，不推送。


## 2026-10-06 · 账户清单写入内部归因补证

五个固定子阶段的两档完整样本、独立时钟复算及失败/检查/保全原件已归档，见[当前归因记录](RF-312-native-account-write-attribution.md)。该记录补充本阶段，不覆盖旧失败或替代无观测器基线；首页超时根因、受控优化效果、睡眠、多端与材质前置仍未关闭。
