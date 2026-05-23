## 16. 附录

### 16.1 Host Functions ABI 规范

| 函数签名 | 命名空间 | 返回值 | 说明 |
|----------|----------|--------|------|
| `request_field(field_ptr, field_len, out_ptr, out_cap) -> i32` | `solosoul` | 0: 成功, -1: 权限不足, -2: 用户拒绝, -3: TTL 过期, -4: 缓冲区不足, -5: 字段路径非法, -7: Vault 已锁定, -8: 频率超限 | 请求用户字段 |
| `post_data(url_ptr, url_len, body_ptr, body_len, out_ptr, out_cap) -> i32` | `solosoul` | 0: 成功, -10: 域名未授权, -6: 网络超时 | 阻塞代理网络请求 |
| `log(level, msg_ptr, msg_len)` | `solosoul` | void | 写审计日志 |
| `get_timestamp() -> i64` | `solosoul` | Unix 时间戳（毫秒） | 获取当前时间 |

### 16.2 错误码总表

| 错误码 | 含义 | 触发场景 |
|--------|------|----------|
| 0 | 成功 | 正常返回 |
| -1 | 权限不足 | 字段不在 manifest 声明范围内 |
| -2 | 用户拒绝 | 用户点击"拒绝" |
| -3 | TTL 过期 | Session 超时或 Store 被销毁 |
| -4 | 缓冲区不足 | `out_cap` 小于返回数据长度 |
| -5 | 字段路径非法 | 不符合 UnifiedObject 路径规范 |
| -6 | 网络超时 | HTTP 请求超过 30 秒 |
| -7 | Vault 已锁定 | 用户未登录或 Vault 被手动锁定 |
| -8 | 频率超限 | 同一字段超过 10 次/分钟 |
| -10 | 域名未授权 | URL 不在 manifest 白名单中 |
| -100 | 燃料耗尽 | Wasm 执行超过 Fuel 限制 |

### 16.3 版本兼容性检查流程

```
Plugin 安装/更新时：
  │
  ▼
 读取 manifest.plugin_api_version
  │
  ▼
 与主软件 PLUGIN_API_VERSION 对比
  │
  ├─ 不匹配 ──▶ 拒绝安装，提示"请升级 SoloSoul"
  │
  ▼ 匹配
 读取 manifest.min_app_version
 读取 manifest.max_app_version
  │
  ▼
 检查 min <= AppVersion <= max
  │
  ├─ 不满足 ──▶ 拒绝安装，提示版本不兼容
  │
  ▼ 满足
 继续 SHA-256 校验
```
