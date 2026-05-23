## 3. 插件市场目录结构（SoloSoul_plugin_market/）

```
SoloSoul_plugin_market/
├── README.md                    # 插件开发指南
├── SDK/                         # 插件开发 SDK
│   ├── rust/
│   │   ├── Cargo.toml           # solosoul-plugin-sdk
│   │   └── src/
│   │       └── lib.rs           # Host Functions 绑定 + 宏
│   ├── typescript/              # 未来：AssemblyScript SDK
│   └── schema/
│       └── manifest.schema.json # manifest.json JSON Schema
├── registry.json                # 官方插件白名单注册表
├── plugins/                     # 官方插件仓库
│   └── com.solosoul.slotgo/
│       ├── manifest.json
│       ├── plugin.wasm          # 编译产物（Release 构建）
│       ├── plugin.debug.wasm    # 调试用（含符号表）
│       └── src/                 # 源码（可选开源）
│           └── lib.rs
└── examples/                    # 示例插件
    └── hello_world/
        ├── manifest.json
        └── src/
            └── lib.rs
```

### 3.1 registry.json（插件白名单注册表）

```json
{
  "version": "1",
  "updated_at": "2026-05-22T00:00:00Z",
  "plugins": {
    "com.solosoul.slotgo": {
      "name": "SlotGo - UK Visa Booking",
      "publisher": "SoloSoul Team",
      "latest_version": "1.0.0",
      "versions": {
        "1.0.0": {
          "sha256": "a3b5c8d7e9f0123456789abcdef0123456789abcdef0123456789abcdef0123",
          "min_app_version": "1.0.0",
          "max_app_version": "2.0.0",
          "plugin_api_version": "1.0",
          "download_url": "https://plugins.solosoul.dev/com.solosoul.slotgo/1.0.0/",
          "released_at": "2026-05-20T00:00:00Z"
        }
      }
    }
  }
}
```

### 3.2 manifest.json（单个插件清单）

Rust Host 与 Go PluginManager 共用统一 Schema：

```json
{
  "plugin_id": "com.solosoul.slotgo",
  "name": "SlotGo - UK Visa Booking",
  "version": "1.0.0",
  "plugin_api_version": "1.0",
  "min_app_version": "1.0.0",
  "max_app_version": "2.0.0",
  "description": "自动监控并预约 UK Visa 面签时间",
  "publisher": "SoloSoul Team",
  "homepage": "https://github.com/Gczmy/SoloSoul_plugin_market",
  "signature": "base64-encoded-ed25519-signature",
  "required_fields": [
    "identity.full_name",
    "travel.primary_passport.number"
  ],
  "optional_fields": [
    "identity.contact.emails",
    "identity.contact.phones"
  ],
  "network_policy": {
    "allowed_domains": [
      "*.visaservices.com",
      "api.booking.com"
    ],
    "block_all_outbound": true
  },
  "data_ttl_seconds": 300,
  "require_user_confirmation": true,
  "consent_validity_hours": 24
}
```

> **Schema 统一说明**：
> - `plugin_api_version` + `max_app_version` 为 v1.2 新增，用于版本兼容性检查。
> - `data_ttl_seconds` 和 `network_policy` 在 Go 的 `PluginManifest` 中当前缺失，实施时需同步补充。
