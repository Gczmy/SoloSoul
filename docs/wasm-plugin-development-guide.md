# SoloSoul Wasm 插件开发指南

本指南面向插件开发者，说明如何为 SoloSoul 编写一个符合 ABI 规范的 WebAssembly 插件。

## 1. 插件生命周期

SoloSoul 插件宿主基于 `wasmtime` + `wasmtime-wasi`（WASI Preview1）运行。每个插件必须：

1. 编译目标：`wasm32-wasip1`。
2. 导出函数：`(module "run") -> i32`，返回值 `0` 表示成功，非零表示插件自定义错误码。
3. 通过 `env` 模块导入 Host Functions 与核心交互（读取字段、发送结果、网络请求等）。

## 2. 推荐开发方式

### 方式 A：使用 Rust SDK（推荐）

`SoloSoul_plugin_market/SDK/rust` 已提供类型安全的封装，开发者只需关注业务逻辑。

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

    // 读取用户字段
    let name = match sdk::get_field("identity.full_name") {
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
      (i32.const 27)  ;; msg_len (UTF-8 字节长度)
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

每个插件必须附带 `manifest.json`，示例如下：

```json
{
  "id": "my-plugin",
  "name": "我的插件",
  "version": "1.0.0",
  "description": "Rust SDK 示例插件",
  "author": "SoloSoul Contributors",
  "homepage": "https://example.com",
  "tier": "p2",
  "category": "productivity",
  "permissions": ["identity.full_name"],
  "network_policy": {
    "allow_hosts": ["api.example.com"]
  },
  "require_user_confirmation": false,
  "data_ttl_seconds": 300,
  "params": [
    {
      "id": "format",
      "label": "输出格式",
      "type": "radio",
      "options": [
        { "id": "text", "label": "纯文本" },
        { "id": "markdown", "label": "Markdown" }
      ],
      "defaultValue": "text"
    }
  ]
}
```

字段说明：

| 字段 | 类型 | 说明 |
|------|------|------|
| `id` | string | 插件唯一标识 |
| `name` | string | 显示名称 |
| `version` | string | SemVer 版本号 |
| `tier` | string | `p0` ~ `p4`，影响默认启用策略 |
| `category` | string | 分类，用于 Dashboard 筛选 |
| `permissions` | string[] | 需要访问的 Vault 字段路径 |
| `network_policy.allow_hosts` | string[] | 允许访问的域名白名单 |
| `require_user_confirmation` | bool | 运行前是否需要用户确认 |
| `data_ttl_seconds` | number | 会话数据有效期（秒） |
| `params` | object[] | 运行参数定义 |

## 4. 调试与测试

### 4.1 使用 SoloSoul 宿主测试

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

启动 SoloSoul 客户端，进入「插件市场」即可看到并运行该插件。

### 4.2 使用 Rust 集成测试

参考 `tauri/src-tauri/tests/plugin_sandbox.rs`，可以：

- 用 `wat::parse_str` 快速构造测试用 Wasm。
- 用 `WasmSandbox::compile` 与 `execute` 直接执行。
- 验证 Host Functions 行为与崩溃隔离。

## 5. 常见错误码

Host Functions 返回的错误码与 SDK 中的 `PluginError` 保持一致：

| 错误码 | 含义 |
|--------|------|
| 0 | 成功 |
| -1 | 权限不足 |
| -2 | 用户拒绝 |
| -3 | TTL 过期或超时 |
| -4 | 缓冲区不足 |
| -5 | 字段路径非法 |
| -6 | 网络超时 |
| -7 | Vault 已锁定 |
| -8 | 频率超限 |
| -10 | 域名未授权 |
| -11 | 非法参数 |

## 6. 安全约定

- 插件只能访问 `manifest.permissions` 中声明的字段，否则 `get_field` 返回 `-1`。
- 网络请求只能发往 `network_policy.allow_hosts` 中的域名。
- 插件运行受 Fuel 限制，死循环会被强制终止。
- 插件 panic 或 Wasm trap 已被 `catch_unwind` 隔离，不会影响宿主与其他插件。

## 7. 参考链接

- `SoloSoul_plugin_market/SDK/rust/src/lib.rs` — Rust SDK 完整实现
- `tauri/src-tauri/src/plugin/host.rs` — Host Functions 注册与行为
- `tauri/src-tauri/tests/plugin_sandbox.rs` — 集成测试示例
