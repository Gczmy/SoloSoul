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

## 可保留的合成 Vault（2026-09-30）

原测量模式使用的临时目录会在进程退出后自动删除，不能直接交给 GUI。示例增加两种数据准备模式：`--fixture-output` 在一个绝对、尚不存在的目录生成100或5,000个对象；`--verify-fixture` 以第二个进程重新解锁并验证落盘数据。生成模式只准备数据，不输出应用性能样本；无这两个参数时继续执行原临时测量。

在 `tauri/` 运行，先创建专用父目录，末级 `vault100`、`vault5000` 必须不存在：

```powershell
$fixtureParent = Join-Path $env:TEMP ('solosoul-rf312-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $fixtureParent | Out-Null
cargo run -p solosoul-core --no-default-features --release --example perf_baseline -- --objects 100 --fixture-output (Join-Path $fixtureParent 'vault100')
cargo run -p solosoul-core --no-default-features --release --example perf_baseline -- --verify-fixture (Join-Path $fixtureParent 'vault100')
cargo run -p solosoul-core --no-default-features --release --example perf_baseline -- --objects 5000 --fixture-output (Join-Path $fixtureParent 'vault5000')
cargo run -p solosoul-core --no-default-features --release --example perf_baseline -- --verify-fixture (Join-Path $fixtureParent 'vault5000')
```

数据使用公开的合成密码 `perf-baseline-only-password`，账户为 `acc_rf312_100`、`acc_rf312_5000`。对象逻辑内容及每第20个对象命中 `needle` 的规则固定，预期命中5/250；加密盐、nonce和创建时间仍会变化，不要求Vault逐字节相同。Profile含空sections/preferences，UI固定英文、浅色/ocean主题并标记引导已完成；未包含附件或OCR图片。正式测量用Release生成，以免首次GUI解锁发生开发KDF到生产KDF迁移；完成标记记录实际KDF参数。

生成使用原子目录创建，已有目录、文件或链接都拒绝。所有数据校验通过后才写入 `rf312-fixture.json`；失败时保留新目录供排查，后续调用仍拒绝覆盖。重开验证先确认账户清单、config、vault.db和UI偏好文件存在，再加载账户、真实解锁并检查对象数、搜索命中、Profile与UI配置，缺文件或校验失败返回非零退出码。程序不读取默认用户Vault，也不自动删除传入目录。

该入口只是原生应用测量的数据前置，不能证明GUI启动、OCR/预览、锁定恢复、内存或IPC指标。后续每次原生样本使用独立数据副本，保留失败样本；先使用独立的Windows测试账户或等价隔离环境。`SOLOSOUL_DATA_DIR`只隔离Vault和UI偏好：桌面[setup](../../tauri/src-tauri/src/setup/mod.rs)的日志/导入暂存清理仍解析固定 `com.solosoul.app` 目录，[插件存储](../../tauri/crates/solosoul-plugin/src/store.rs)仍使用系统用户目录。修改Tauri identifier或子进程的APPDATA/USERPROFILE变量不能视为这些路径已隔离。本阶段不启动GUI，RF-312保持待验证。

本机Release实测：两档生成及第二进程重开全部exit0，对象数100/5000、搜索命中5/250、对象/命中ID、Profile与UI偏好均符合契约；实际KDF为64MiB/3次/并行4。已有目录/文件、相对路径、非法口径、互斥模式、缺失数据库和两类junction负例共8项均exit1，原数据SHA未变、缺失数据库未新建。原临时入口100对象/2样本四项无失败，退出目录已清除；这是兼容性验证，原10样本基线继续保留。源码与Release程序SHA、生成标记和验证原始JSON见[2026-09-30证据](rf312-fixtures-windows-2026-09-30.json)。
