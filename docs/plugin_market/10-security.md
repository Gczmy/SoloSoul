## 11. 安全机制清单

| 安全维度 | 机制 | 实现位置 |
|----------|------|----------|
| **完整性校验** | SHA-256 白名单 + registry.json | `PluginInstallerService`, `sandbox.rs` |
| **执行隔离** | Wasmtime 沙盒 + WASI | `sandbox.rs` |
| **防死循环** | Fuel 限制（10,000,000） | `sandbox.rs` |
| **字段级授权** | manifest 声明 + 逐字段确认 | `host.rs`, `plugin_consent_dialog.dart` |
| **会话控制** | Session TTL + 显式撤销 | `host.rs` |
| **Store 级隔离** | TTL 到期后整个 Store drop | `sandbox.rs::execute()` |
| **内存安全** | mlock + Zeroize | `host.rs`, `sandbox.rs` |
| **网络隔离** | 域名白名单 + 阻塞代理 | `host.rs` |
| **速率限制** | 10 次/分钟/字段 | `host.rs::RateLimiter` |
| **审计追溯** | 不可篡改链式日志 | `host.rs` -> `operation_logger.dart` |
| **存储隔离** | 插件目录 0700，与 Vault 同级 | `PluginStore::new()` |
| **卸载安全** | 卸载时强制 Revoke Session + Store drop | `PluginInstallerService::uninstall()` |
| **版本兼容** | `plugin_api_version` + `min/max_app_version` | `PluginInstallerService::_isCompatible` |
| **侧载限制** | Release 模式禁止本地文件安装 | `installFromLocal()` |
| **JIT 解密** | 排队期间仅持 Task ID，提交前才解密 | [13-lifecycle.md](13-lifecycle.md) |
| **熔断机制** | 连续 100 次失败自动熔断 | [13-lifecycle.md](13-lifecycle.md) |

### 11.1 Debug / Release 差异

| 特性 | Debug 模式 | Release 模式 |
|------|-----------|-------------|
| SHA-256 白名单 | 可通过 `installFromLocal` 加载未签名插件 | 强制校验，缺一不可 |
| 侧载（本地安装） | 允许 | **禁止** |
| 网络白名单 | 放宽至 `localhost` | 严格 manifest 白名单 |
| 日志级别 | Verbose（包含 Plugin 内部日志） | Error only |
| Fuel 限制 | 100,000,000（方便调试） | 10,000,000 |
| Session TTL | 30 分钟 | 5 分钟 |
| Rate Limit | 100 次/分钟 | 10 次/分钟 |
