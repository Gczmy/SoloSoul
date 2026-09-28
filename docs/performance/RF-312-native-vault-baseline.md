# RF-312 原生 Vault 后端子基线（2026-09-28）

此记录仅测共享 Rust Vault 的账户目录载入、解锁、对象元数据列表与解密搜索。它不包含 Tauri/WebView 启动、IPC、页面渲染、OCR、附件预览、锁定恢复或内存峰值，不能用来关闭 RF-312 的多端应用验收。

## 复跑口径

- 检出：`519bad7b` 加本项测量程序；Windows 11 Enterprise LTSC build 26100；Intel Core i7-9700（8 核/8 线程）；可见内存约 15.8 GiB；Rust/Cargo 1.96.0。
- 构建：Release、`solosoul-core --no-default-features`。关闭媒体 feature 只减少构建依赖，不改变本项 Vault 路径。Release KDF 实际为 Argon2id 64 MiB、3 次、并行度 4；程序输出实际参数，不以 `SOLOSOUL_SECURE` 环境变量是否设置推断。
- 数据：每次在自动清理的临时目录创建一个合成账户，分别写入 100 或 5,000 个对象。对象 ID、名称和属性由序号确定，每第 20 个对象的属性含 `needle`；搜索结果应分别为 5 或 250。写入时间不计入操作样本。
- 预热：每项先运行一次，再连续测 10 次。解锁每次先锁定再以主密码解锁，包含 KDF 与 Vault 重开；列表走 `list_object_metadata`；搜索走 `search_objects` 的全表解密匹配。所有原始样本、返回条数和失败项均保存在 JSON 中。中位数按偶数样本中间两项均值；P95 按最近秩，10 次样本时等于最大值。

```powershell
cd tauri
cargo run -p solosoul-core --no-default-features --release --example perf_baseline -- --objects 100 --samples 10
cargo run -p solosoul-core --no-default-features --release --example perf_baseline -- --objects 5000 --samples 10
```

每条命令最后一行是 JSON 结果；此前可能有 Vault 文件权限日志。程序使用 `tempfile`，退出时清理合成账户，不读取默认用户数据目录。

## 本机结果

单位为毫秒；各组 10/10 成功。原始样本：[100 个对象](vault-windows-100.json)、[5,000 个对象](vault-windows-5000.json)。

| 操作 | 100 个对象 中位/P95 | 5,000 个对象 中位/P95 |
|---|---:|---:|
| 账户目录载入 | 0.183 / 0.287 | 0.190 / 0.246 |
| 主密码解锁 | 182.576 / 430.911 | 171.965 / 315.107 |
| 元数据列表 | 0.259 / 0.744 | 14.549 / 18.032 |
| 解密搜索 | 0.960 / 1.601 | 44.703 / 46.907 |

这个样本只说明该 Windows 主机上的后端搜索在 5,000 个对象时中位约 45 ms，中位解锁约 172–183 ms；单轮 P95 波动明显，不能证明实际 GUI 搜索响应、启动体验或低配移动端性能。下一步仍需用相同规模的 Vault 在 Windows/macOS/Android 原生应用中测量启动、解锁到可交互、页面搜索、首次 OCR/附件预览、锁定恢复、内存峰值与 IPC 次数，并保留每次原始样本。未得到这些数据前，不据此决定全路由懒加载或分页改造。
