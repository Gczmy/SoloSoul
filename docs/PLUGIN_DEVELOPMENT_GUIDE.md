# SoloSoul 插件开发指南

> 本文档面向插件开发者，说明如何为 SoloSoul 开发 Wasm 插件。

---

## 一、插件架构概述

SoloSoul 插件运行在 **Wasmtime 沙盒**中，通过 WASI 接口与主软件通信：

- **插件（WASM）**：执行业务逻辑，只通过 Host Functions 访问数据和输出结果。
- **主软件（Rust Host + Dart UI）**：提供沙盒环境、字段提取、授权管理、UI 渲染。

插件无需关心 UI 实现，只需通过标准化的 Host Functions 输出数据和日志。

---

## 二、Host Functions 参考

插件通过 `extern "C"` 声明调用以下 Host Functions：

### 2.1 字段读取

```rust
extern "C" {
    /// 读取用户数据中的字段值
    /// field_id: 字段路径，如 "address.title"、"identity.full_name"
    /// out_ptr: 输出缓冲区指针
    /// out_cap: 缓冲区容量
    /// 返回值: 0=成功, -1=权限拒绝, -2=用户拒绝, -3=超时, -4=缓冲区不足
    fn solosoul_request_field(field_id: *const u8, field_id_len: usize, out_ptr: *mut u8, out_cap: usize) -> i32;
}
```

### 2.2 日志输出

```rust
extern "C" {
    /// 输出执行日志（用于调试和过程信息）
    /// level: "info" 或 "error"
    fn solosoul_log(level: *const u8, level_len: usize, msg: *const u8, msg_len: usize);
}
```

**注意**：日志内容会展示在"执行日志"区（默认折叠）。插件应只输出有意义的过程信息，避免在辅助函数中打印调试日志。

### 2.3 结构化结果（Phase 2）

```rust
extern "C" {
    /// 发送结构化最终结果（用于结果区卡片展示）
    /// data: JSON 字符串，必须包含 "type" 字段
    /// 返回值: 0=成功, -1=大小超限(64KB), -3=嵌套深度超限(10), -4=非法type, -5=缺少type, -6=非法JSON
    fn solosoul_result(data: *const u8, data_len: usize) -> i32;
}
```

**与 `solosoul_log` 的区别**：
- `solosoul_log` → 执行日志区（纯文本，默认折叠）
- `solosoul_result` → 结果区（结构化卡片，默认展开）

---

## 三、结构化结果格式

### 3.1 `type: "text"` — 纯文本

```json
{
  "type": "text",
  "content": "格式化后的地址：北京市朝阳区长安街1号"
}
```

### 3.2 `type: "key_value"` — 键值对

```json
{
  "type": "key_value",
  "title": "地址解析结果",
  "pairs": [
    {"key": "国家", "value": "中国"},
    {"key": "省/市", "value": "北京市"},
    {"key": "街道", "value": "长安街1号"}
  ]
}
```

### 3.3 `type: "table"` — 表格

```json
{
  "type": "table",
  "headers": ["字段", "值"],
  "rows": [
    ["街道", "长安街1号"],
    ["城市", "北京"]
  ]
}
```

### 3.4 `type: "markdown"` — 富文本

```json
{
  "type": "markdown",
  "content": "**地址**：长安街1号\n*备注*：精确到门牌"
}
```

> Markdown 仅支持粗体、斜体、列表、内联代码。外部链接点击会显示提示而非直接跳转。

### 3.5 `type: "map"` — 地图（需要主软件支持）

```json
{
  "type": "map",
  "latitude": 39.9042,
  "longitude": 116.4074,
  "title": "定位点"
}
```

> 地图卡片依赖主软件是否集成地图 SDK。若未集成，会降级显示坐标文本。

---

## 四、辅助函数（推荐封装）

### Rust 插件 SDK

```rust
use serde_json::json;

/// 发送文本结果
pub fn result_text(content: &str) {
    let data = json!({"type": "text", "content": content}).to_string();
    unsafe {
        solosoul_result(data.as_ptr(), data.len());
    }
}

/// 发送键值对结果
pub fn result_key_value(title: &str, pairs: &[(&str, &str)]) {
    let pairs: Vec<_> = pairs.iter()
        .map(|(k, v)| json!({"key": k, "value": v}))
        .collect();
    let data = json!({"type": "key_value", "title": title, "pairs": pairs}).to_string();
    unsafe {
        solosoul_result(data.as_ptr(), data.len());
    }
}

/// 发送表格结果
pub fn result_table(headers: &[&str], rows: &[Vec<&str>]) {
    let data = json!({"type": "table", "headers": headers, "rows": rows}).to_string();
    unsafe {
        solosoul_result(data.as_ptr(), data.len());
    }
}

/// 发送 Markdown 结果
pub fn result_markdown(content: &str) {
    let data = json!({"type": "markdown", "content": content}).to_string();
    unsafe {
        solosoul_result(data.as_ptr(), data.len());
    }
}

/// 输出日志
pub fn log_info(msg: &str) {
    let level = "info";
    unsafe {
        solosoul_log(level.as_ptr(), level.len(), msg.as_ptr(), msg.len());
    }
}

/// 读取字段
pub fn read_field(field_id: &str) -> String {
    let mut buf = vec![0u8; 4096];
    let ret = unsafe {
        solosoul_request_field(field_id.as_ptr(), field_id.len(), buf.as_mut_ptr(), buf.len())
    };
    if ret == 0 {
        let len = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
        String::from_utf8_lossy(&buf[..len]).to_string()
    } else {
        String::new()
    }
}
```

---

## 五、manifest.json 格式

```json
{
  "plugin_id": "com.example.my-plugin",
  "name": "My Plugin",
  "version": "1.0.0",
  "plugin_api_version": "1.0",
  "min_app_version": "1.0.0",
  "max_app_version": "2.0.0",
  "description": "插件描述",
  "publisher": "Your Name",
  "required_fields": ["address.title", "address.street"],
  "optional_fields": ["address.country"],
  "network_policy": {
    "block_all_outbound": true
  },
  "require_user_confirmation": false
}
```

### 版本兼容性

- `plugin_api_version`：插件 ABI 版本，必须与主软件严格匹配。主软件当前版本为 `"1.0"`。
- `min_app_version` / `max_app_version`：兼容的 SoloSoul App 版本范围。

---

## 六、降级策略

### 新插件 + 旧主软件

如果主软件不支持 `solosoul_result`，调用会返回错误码。插件应捕获错误并降级使用 `log_info`：

```rust
fn output_result(content: &str) {
    let data = json!({"type": "text", "content": content}).to_string();
    let ret = unsafe {
        solosoul_result(data.as_ptr(), data.len())
    };
    if ret != 0 {
        // 降级：用日志输出
        log_info(content);
    }
}
```

---

## 七、完整示例：地址格式化器

```rust
use serde_json::json;

extern "C" {
    fn solosoul_request_field(field_id: *const u8, field_id_len: usize, out_ptr: *mut u8, out_cap: usize) -> i32;
    fn solosoul_log(level: *const u8, level_len: usize, msg: *const u8, msg_len: usize);
    fn solosoul_result(data: *const u8, data_len: usize) -> i32;
}

fn read_field(field_id: &str) -> String { /* ... */ }
fn log_info(msg: &str) { /* ... */ }

#[no_mangle]
pub extern "C" fn run() -> i32 {
    let count_str = read_field("address.count");
    let count: usize = count_str.parse().unwrap_or(0);
    
    let mut pairs = Vec::new();
    
    for i in 0..count {
        let title = read_field(&format!("address[{}].title", i));
        let street = read_field(&format!("address[{}].street", i));
        let city = read_field(&format!("address[{}].city", i));
        let country = read_field(&format!("address[{}].country", i));
        
        let formatted = format!("{}, {}, {}", street, city, country);
        pairs.push((title.as_str(), formatted.as_str()));
        
        log_info(&format!("地址[{}] 格式化完成", i));
    }
    
    // 输出结构化结果
    let data = json!({
        "type": "key_value",
        "title": "地址格式化结果",
        "pairs": pairs.iter().map(|(k, v)| json!({"key": k, "value": v})).collect::<Vec<_>>()
    }).to_string();
    
    unsafe {
        solosoul_result(data.as_ptr(), data.len());
    }
    
    0
}
```

---

## 八、安全限制

| 限制项 | 值 | 说明 |
|--------|-----|------|
| 结果 JSON 大小 | ≤ 64KB | 超过返回 -1 |
| 嵌套深度 | ≤ 10 | 超过返回 -3 |
| 合法 type | text, key_value, table, map, markdown | 其他返回 -4 |
| Wasm 大小 | ≤ 10MB | 下载时限制 |
| Fuel 限制 | 10M (Release) / 100M (Debug) | 防止死循环 |
| Session TTL | 默认 300s | manifest 中可配置 |
