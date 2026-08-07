# SoloSoul Wasm 插件开发指南

本指南面向插件开发者，说明如何为 SoloSoul 编写一个符合 ABI 规范的 WebAssembly 插件。

> 与当前实现对齐：`SoloSoul_plugin_market/SDK/rust/src/lib.rs`（SDK）、`tauri/crates/solosoul-plugin/src/host.rs`（Host Functions）、`tauri/crates/solosoul-plugin/src/sandbox.rs`（沙盒）。

## 1. 插件生命周期

SoloSoul 插件宿主基于 `wasmtime` + `wasmtime-wasi`（WASI Preview1）运行。每个插件必须：

1. 编译目标：`wasm32-wasip1`。
2. 导出入口函数：`#[no_mangle] pub extern "C" fn run() -> i32`，返回值 `0` 表示成功，非零表示插件自定义错误码。
3. 通过 `env` 模块导入 Host Functions 与核心交互（读取字段、发送结果、网络请求、对话框等），Host 侧全部注册在 `"env"` 命名空间下。

## 2. 推荐开发方式

### 方式 A：使用 Rust SDK（推荐）

`SoloSoul_plugin_market/SDK/rust`（crate 名 `solosoul-plugin-sdk`）已提供类型安全的封装，开发者只需关注业务逻辑。

#### 2.1 创建插件项目

```bash
cargo new --lib my-plugin
cd my-plugin
```

#### 2.2 Cargo.toml

```toml
[package]
name = "my-plugin"
version = "0.1.0"
edition = "2021"

[dependencies]
solosoul-plugin-sdk = { path = "../SoloSoul_plugin_market/SDK/rust" }

[lib]
crate-type = ["cdylib"]

[profile.release]
opt-level = "s"
lto = true
```

#### 2.3 src/lib.rs

```rust
use solosoul_plugin_sdk as sdk;

#[no_mangle]
pub extern "C" fn run() -> i32 {
    sdk::log_info("my-plugin 开始运行");

    // 读取用户字段（字段路径为「类型别名.属性路径」，camelCase，见 §4.1）
    let name = match sdk::get_field("identity.fullName") {
        Ok(v) => v,
        Err(e) => {
            sdk::log_error(&format!("读取字段失败: {:?}", e));
            return 1;
        }
    };

    // 发送文本结果
    sdk::result_text(&format!("你好，{}！", name));

    // 发送键值对结果
    sdk::result_key_value("用户信息", &[
        ("姓名", &name),
        ("来源", "Rust SDK 示例"),
    ]);

    0
}
```

#### 2.4 编译

```bash
cargo build --target wasm32-wasip1 --release
cp target/wasm32-wasip1/release/my-plugin.wasm plugin.wasm
```

### 方式 B：手写 WAT（用于理解 ABI）

以下示例展示一个最小插件：调用 `env.solosoul_log` 记录日志并返回 `0`。

```wat
(module
  ;; 导入日志 Host Function
  ;; 签名: (param i32 i32 i32 i32)  ;; level_ptr, level_len, msg_ptr, msg_len
  (import "env" "solosoul_log" (func $solosoul_log (param i32 i32 i32 i32)))

  ;; 内存必须导出，Host 通过 (export "memory") 读写字符串
  (memory (export "memory") 1)

  ;; 数据段：日志级别与消息
  (data (i32.const 0) "info")
  (data (i32.const 16) "来自 WAT 插件的问候")

  ;; 导出 run 函数
  (func (export "run") (result i32)
    ;; solosoul_log("info", "来自 WAT 插件的问候")
    (call $solosoul_log
      (i32.const 0)   ;; level_ptr
      (i32.const 4)   ;; level_len
      (i32.const 16)  ;; msg_ptr
      (i32.const 26)  ;; msg_len (UTF-8 字节长度：3×8 汉字 + 2 空格 + 3 个 ASCII = 26)
    )
    (i32.const 0)     ;; 成功返回 0
  )
)
```

将上述内容保存为 `plugin.wat`，使用 `wat2wasm` 转换：

```bash
wat2wasm plugin.wat -o plugin.wasm
```

> 提示：Rust 测试集已使用 `wat` crate 直接在代码中解析 WAT，开发者可以参考 `tauri/src-tauri/tests/plugin_sandbox.rs`。

## 3. manifest.json

每个插件根目录（与 `plugin.wasm` 同级）必须包含 `manifest.json`：

```json
{
  "plugin_id": "com.example.my-plugin",
  "name": "我的插件",
  "version": "1.0.0",
  "plugin_api_version": "1.0",
  "min_app_version": "1.0.0",
  "max_app_version": "999.999.999",
  "description": "Rust SDK 示例插件",
  "publisher": "SoloSoul Contributors",
  "homepage": "https://example.com",
  "required_fields": ["identity.fullName"],
  "optional_fields": [],
  "network_policy": {
    "block_all_outbound": true,
    "allowed_domains": []
  },
  "data_ttl_seconds": 300,
  "require_user_confirmation": false,
  "i18n": {
    "zh": { "name": "我的插件", "description": "Rust SDK 示例插件" },
    "en": { "name": "My Plugin", "description": "Example plugin" }
  },
  "tier": "p2",
  "category": "productivity",
  "params": [
    {
      "id": "format",
      "label": "输出格式",
      "type": "select",
      "required": false,
      "defaultValue": "text",
      "options": [
        { "value": "text", "label": "纯文本" },
        { "value": "markdown", "label": "Markdown" }
      ]
    }
  ]
}
```

字段说明：

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `plugin_id` | string | ✅ | 反向域名格式，全局唯一标识 |
| `name` | string | ✅ | 插件显示名称 |
| `version` | string | ✅ | SemVer 版本号，如 `1.0.0` |
| `plugin_api_version` | string | ✅ | 插件 ABI 版本，与客户端严格匹配（当前官方插件 1.0–2.0） |
| `min_app_version` | string | ✅ | 兼容的最低 SoloSoul 客户端版本 |
| `max_app_version` | string | ✅ | 兼容的最高 SoloSoul 客户端版本 |
| `description` | string | ✅ | 一句话描述插件功能 |
| `publisher` | string | ✅ | 发布者名称 |
| `homepage` | string | ❌ | 项目主页 URL |
| `required_fields` | string[] | ✅ | 必需字段路径列表（camelCase，缺失时客户端提示） |
| `optional_fields` | string[] | ❌ | 可选字段路径列表 |
| `network_policy` | object | ❌ | `block_all_outbound`（默认 `true`）+ `allowed_domains` 域名白名单 |
| `data_ttl_seconds` | number | ❌ | 授权数据内存存活时间（秒），默认 `300` |
| `require_user_confirmation` | boolean | ❌ | 是否要求用户确认，默认 `true` |
| `i18n` | object | ❌ | `zh` / `en` 等语言的名称与描述覆盖 |
| `tier` | string | ❌ | 分批启用层级：`p0`–`p4`（默认 `p3`，客户端默认启用 p0/p1/p2） |
| `category` | string | ❌ | 分类，用于看板筛选 |
| `contracts` | array | ❌ | 类型契约：`typeId` / `typeIdAliases` / `roles` / `bindings`（语义角色绑定） |
| `params` | array | ❌ | 运行参数定义（`id` / `label` / `type` / `required` / `description` / `defaultValue` / `options[{value,label}]`），`type` 仅支持 `string` / `number` / `boolean` / `select` |

> 完整 JSON Schema 见 `SoloSoul_plugin_market/SDK/schema/manifest.schema.json`。

## 4. SDK API 一览

### 4.1 字段路径约定

`get_field` 等数据访问 API 使用 **「类型别名.属性路径」**（camelCase）的 typed field 格式，例如：

- `idCard.number`、`passport.expiryDate`、`contact.email`、`address.street`
- 类型别名经「用户模板反查」或 manifest `contracts.typeIdAliases` 绑定到 Vault 实际对象

### 4.2 函数分组

| 分组 | 函数 | 说明 |
|------|------|------|
| 数据访问 | `list_objects(type_id)` | 列出指定契约类型的所有对象（JSON 数组） |
| | `get_field(field_id)` | 请求字段数据（受 manifest 声明范围 + Consent 约束） |
| | `get_data_structure_tree()` | 获取 Vault 数据契约结构树（元数据，不含值） |
| | `list_attachments()` | 列出可用于处理的附件（图片/PDF，按页面→对象分组） |
| | `prepare_attachment_copy(object_id, attachment_id)` | 将附件复制到插件临时工作区，返回副本路径 |
| 网络 | `post_json(url, json_body)` | 经宿主代理的 HTTP 请求（受 `network_policy` 白名单约束）。宿主侧底层为异步 API `solosoul_http_request` / `http_poll` / `http_read` / `http_close`，SDK 的 `post_json` 封装了同步兼容入口 `solosoul_post_data` |
| 交互 | `show_dialog(config_json)` | 弹出用户交互对话框。`config_json` 形如 `{"title":"选择","type":"radio_list","items":[{"id":"a","label":"A"}]}`，返回用户选择的 JSON；用户取消 → `UserDenied`，超时 → `TtlExpired` |
| 参数 | `get_param(key)` / `get_locale()` | 读取运行参数 / 当前系统 locale（如 `zh-CN`） |
| 结果 | `send_result_json(json)` | 发送结构化结果（`type` 取 `text` / `key_value` / `table` / `markdown` 等） |
| | `result_text(content)` / `result_key_value(title, pairs)` / `result_table(headers, rows)` / `result_markdown(content)` | 便捷结果构造 |
| | `write_output_file(file_name, bytes)` / `copy_output_file(src_path, file_name)` | 将文件写入运行参数 `outputDir` 指定目录（返回绝对路径） |
| 水印 | `image_watermark(input, output, config)` / `pdf_watermark(input, output, config)` | 附件水印（配置经 `WatermarkConfig` / `WatermarkPosition` 构造） |
| 工具 | `log` / `log_info` / `log_warn` / `log_error` / `log_debug` | 结构化日志（进入审计流） |
| | `get_timestamp()` / `sleep(ms)` | 时间与休眠（宿主限制单次 ≤ 1000ms） |
| | `escape_json` / `truncate` / `parse_date_yyyymmdd_or_iso` / `days_until_ymd` | 常用工具函数 |

> SDK 还导出 `PluginMain` 类型别名（`extern "C" fn() -> i32`），用于声明入口函数签名。

## 5. 调试与测试

### 5.1 使用 SoloSoul 宿主测试

将 `plugin.wasm` 与 `manifest.json` 放入本地插件市场目录：

```
SoloSoul_plugin_market/plugins/my-plugin/
  ├── manifest.json
  └── plugin.wasm
```

然后重新生成注册表：

```bash
cd SoloSoul_plugin_market
python3 scripts/generate_registry.py
```

启动 SoloSoul 客户端（开发模式会直接读取该源码目录），进入「插件市场」即可看到并运行该插件。

### 5.2 使用 Rust 集成测试

参考 `tauri/src-tauri/tests/plugin_sandbox.rs`，可以：

- 用 `wat::parse_str` 快速构造测试用 Wasm。
- 用 `WasmSandbox::compile` 与 `execute` 直接执行。
- 验证 Host Functions 行为与崩溃隔离。

## 6. 常见错误码

Host Functions 返回的错误码（`host.rs::code` 模块）与 SDK `PluginError` 保持一致：

| 错误码 | 含义 |
|--------|------|
| 0 | 成功 |
| 1 | HTTP 请求仍在进行中（非错误，用于轮询） |
| -1 | 权限不足（字段不在 manifest 声明范围内） |
| -2 | 用户拒绝 |
| -3 | TTL 过期或 Session 被撤销 |
| -4 | 缓冲区不足 |
| -5 | 字段路径非法 |
| -6 | 网络超时 |
| -7 | Vault 已锁定 |
| -8 | 频率超限 |
| -9 | 未实现 |
| -10 | 域名未授权 |
| -11 | 非法参数 |
| -12 | Wasm Trap |
| -13 | 文件不存在 |
| -14 | 文件过大 |
| -15 | 处理失败 |

> SDK `PluginError` 枚举覆盖其中 `-1`–`-8`、`-10`，其余码由插件侧映射为 `Unknown`（`-99`）。

## 7. 安全约定

- 插件只能访问 `required_fields` / `optional_fields` 中声明的字段，否则 `get_field` 返回 `-1`。
- 网络请求只能发往 `network_policy.allowed_domains` 中的域名（`block_all_outbound: true` 时默认全部阻断）。
- 插件运行受燃料限制（单次 `10_000_000_000`），死循环会被强制终止。
- 插件 panic 或 Wasm trap 已被 `catch_unwind` 隔离，不会影响宿主与其他插件。
- 插件 stdout/stderr 静默丢弃（不继承宿主终端），日志只能通过 `log*` Host Functions 输出。
- 单次运行限制：`sleep` 最多 1000ms；Host 单次读取字符串上限 64 KiB；`send_result_json` 上限 64 KiB、JSON 嵌套 ≤ 10 层。

## 8. 参考链接

- `SoloSoul_plugin_market/SDK/rust/src/lib.rs` — Rust SDK 完整实现（Host Functions 声明与错误码）
- `SoloSoul_plugin_market/SDK/schema/manifest.schema.json` — manifest.json JSON Schema
- `tauri/crates/solosoul-plugin/src/host.rs` — Host Functions 注册与行为（`env` 命名空间）
- `tauri/crates/solosoul-plugin/src/sandbox.rs` — Wasmtime 沙盒执行
- `tauri/crates/solosoul-plugin/src/field.rs` — 字段路径解析（typed field + contracts）
- `tauri/src-tauri/tests/plugin_sandbox.rs` — 集成测试示例
- 客户端插件架构（模块地图 / 安全机制 / 前端）见 [docs/plugin_market/](plugin_market/)
