# RF-312 当前 Windows 媒体、首次 OCR 与固定 PDF 首屏

绑定源码 `4069e46997bda3e5395a9bc2f124c072ee0ff7d5`、同一非默认native-perf Release和正式setup，1883份运行源、38份前端产物、94份资源与原公开100/5000对象schema2夹具保持；没有重编译或重跑原生样本。严格Clippy和142项原生Rust测试复用同源构建。环境和先前完整行程见[当前GUI基线](RF-312-current-windows-gui-baseline.md)。

## 实际结果

媒体两档各五次、首次OCR和PDF各三次，22次原生驱动及独立合同全部通过。样本量分别n=5/n=3，最近秩p95均等于最大值；不与旧拒绝队列拼接。

| 观测项目（ms，中位 / p95） | 100对象 | 5000对象 |
| --- | ---: | ---: |
| 图片预览 | 173.20 / 192.32 | 185.07 / 202.72 |
| 文本预览 | 189.54 / 191.09 | 187.73 / 188.02 |
| 首次OCR结果 | 1148.47 / 1158.19 | 1120.93 / 1211.55 |
| OCR完整观察区间 | 3001.48 / 3688.02 | 2960.63 / 2968.95 |
| 固定PDF首次像素匹配 | 811.85 / 840.04 | 838.41 / 874.35 |
| 固定PDF稳定首屏 | 1405.84 / 1413.92 | 1341.37 / 1343.18 |

媒体90个业务阶段和111/111串行内存点有效。内存是含根进程/WebView树及观测开销的工作集合计，可能重复计算共享页，不是连续峰值；OCR/PDF本组没有连续内存或PDF整体IPC分布。全部数值样本和原口径保留在[结构化记录](rf312-current-windows-media-2026-10-15.json)。

OCR保持原生owned选择器、完整控件树/目标和父链保护、原25秒预算及200ms结构稳定门禁；固定公开图片完整文本、唯一扫描调用和关闭选择器逐次验证。PDF保持固定schema3视口/ROI、原参考掩码、两次连续匹配/至少250ms间隔、16次/25秒及文档/资源绑定要求。18张PNG均独立检查长度、SHA、CRC和像素结果，保留6张早期未匹配加载帧；12次固定像素匹配成立。六次最终整图SHA相同且已实际查看，公开第一页文字可见，不宣称任意PDF或所有页面完成渲染。

## 数据与验证边界

原始源夹具、准备完成的owned副本、运行后副本是三个不同状态。原有准备后逐文件字节门禁保持；运行后的完整媒体行程会写入初始化状态，不能套用只读OCR/PDF副本的数据库字节不变假设。十份数据库以只读immutable SQLite逐表对照，并按当前代码的Argon2id/AES-GCM格式认证解密比较：四个附件路径准确指向各自隔离根，既有对象/资料值保持；新增偏好仅设备同步、更新源和备份提醒，其他差异限于已核对的登录审计、同步和迁移初始化。源、准备标记和运行后哈希分别绑定，未改应用或原生合同。

离线辅助校验的Windows扩展路径表示、IPC数组/字典结构、运行后数据库写入及源/准备哈希角色错误分别保留原stderr和非零回执；修正消费者及完成更强的语义核验后，重读同一批原件。没有重跑GUI、延长预算、修改原件或以成功子集替换失败样本。

本轮额外34次置前中RF-121使用12次、此处使用22次，每个新窗口仅一次；监督器已停止，系统设置保持。fresh CIM确认此处204个记录身份退出，清理176个精确owned缓存，658份Vault/截图/证明与日志副本清理前后哈希相同，0进程被清理工具终止。原始路径、身份、密钥/资料、日志、截图和程序只保留本地，不进入公开记录。

## 复跑

按[公开媒体夹具合同](RF-312-media-fixture-contract.md)准备schema2夹具及匹配的隔离Release；在tauri目录依次对100/5000对象运行以下命令，输出目录必须全新，保留完整组和全部拒绝。所示路径是应替换的占位符，不能指向正式Vault。

```powershell
node scripts/native-perf-media.mjs --exe <isolated-release-exe> --fixture <public-media-fixture> --output <new-owned-output> --samples 5 --memory-interval-ms 2000
node scripts/native-perf-ocr.mjs --exe <same-release-exe> --fixture <same-public-fixture> --output <another-new-output> --samples 3
node scripts/native-perf-pdf-preview.mjs --exe <same-release-exe> --fixture <same-public-fixture> --output <third-new-output> --samples 3
```

具体统计与边界沿用[媒体](RF-312-native-media-preview.md)、[首次OCR](RF-312-native-first-ocr.md)、[固定PDF首屏](RF-312-native-pdf-first-page.md)的合同，当前数据以本记录为准。样本含SDK、轮询和采集开销，无默认未插桩对照，不作跨版本因果比较；当前数据不足以要求全面分页或全路由懒加载。RF-121辅助显示/macOS、RF-312多端与默认控制及原有其他环境门禁仍未关闭。
