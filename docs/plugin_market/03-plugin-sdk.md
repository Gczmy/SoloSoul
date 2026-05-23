## 4. 插件开发 SDK（Rust）

### 4.1 SDK 接口定义

```rust
//! SoloSoul Plugin SDK
//! 插件开发者只需调用这些高级 API，无需了解 Wasm 底层

// 由 Host 注入的函数（在 Rust Host 侧通过 Linker 定义）
extern "C" {
    fn solosoul_request_field(
        field_id_ptr: *const u8,
        field_id_len: usize,
        out_ptr: *mut u8,
        out_cap: usize,
    ) -> i32;
    fn solosoul_post_data(
        url_ptr: *const u8,
        url_len: usize,
        body_ptr: *const u8,
        body_len: usize,
        out_ptr: *mut u8,
        out_cap: usize,
    ) -> i32;
    fn solosoul_log(level: i32, msg_ptr: *const u8, msg_len: usize);
    fn solosoul_get_timestamp() -> i64;
}

/// 请求用户字段数据（触发 Flutter 授权弹窗）
pub fn get_field(field_id: &str) -> Result<String, PluginError> {
    unsafe {
        let mut buf = vec![0u8; 4096];
        let ret = solosoul_request_field(
            field_id.as_ptr(),
            field_id.len(),
            buf.as_mut_ptr(),
            buf.len(),
        );
        match ret {
            0 => Ok(String::from_utf8_lossy(&buf).to_string()),
            -1 => Err(PluginError::PermissionDenied),
            -2 => Err(PluginError::UserDenied),
            -3 => Err(PluginError::TtlExpired),
            -4 => Err(PluginError::BufferTooSmall),
            -5 => Err(PluginError::InvalidFieldPath),
            -7 => Err(PluginError::VaultLocked),
            -8 => Err(PluginError::RateLimited),
            _ => Err(PluginError::Unknown),
        }
    }
}

/// 发起网络请求（受 manifest.network_policy 白名单限制）
///
/// 实现方式：Host Function 内部通过 tokio::runtime::Handle::block_on
/// 阻塞等待 HTTP 响应，对 Plugin 表现为同步调用。
pub fn post_json(url: &str, body: &str) -> Result<String, PluginError> {
    unsafe {
        let mut buf = vec![0u8; 8192];
        let ret = solosoul_post_data(
            url.as_ptr(),
            url.len(),
            body.as_ptr(),
            body.len(),
            buf.as_mut_ptr(),
            buf.len(),
        );
        match ret {
            0 => Ok(String::from_utf8_lossy(&buf).to_string()),
            -10 => Err(PluginError::DomainNotAllowed),
            -6 => Err(PluginError::NetworkTimeout),
            _ => Err(PluginError::NetworkFailed),
        }
    }
}

#[derive(Debug)]
pub enum PluginError {
    PermissionDenied,   // -1: 字段不在 manifest 声明范围内
    UserDenied,         // -2: 用户点击拒绝
    TtlExpired,         // -3: Session TTL 已过期
    BufferTooSmall,     // -4: out_cap 不足
    InvalidFieldPath,   // -5: 字段路径格式非法
    NetworkTimeout,     // -6: 网络请求超时
    VaultLocked,        // -7: Vault 未解锁
    RateLimited,        // -8: 字段访问频率超限
    DomainNotAllowed,   // -10: 域名不在白名单
    NetworkFailed,      // 其他网络错误
    Unknown,
}
```

### 4.2 插件示例（SlotGo）

```rust
use solosoul_plugin_sdk::{get_field, post_json};

#[no_mangle]
pub extern "C" fn run() -> i32 {
    let name = match get_field("identity.full_name") {
        Ok(v) => v,
        Err(e) => {
            solosoul_plugin_sdk::log_error(&format!("获取姓名失败: {:?}", e));
            return -1;
        }
    };

    let passport = match get_field("travel.primary_passport.number") {
        Ok(v) => v,
        Err(_) => return -2,
    };

    let payload = format!(r#"{"name":"{}","passport":"{}"}"#, name, passport);

    match post_json("https://api.visaservices.com/book", &payload) {
        Ok(response) => {
            solosoul_plugin_sdk::log_info(&format!("预约成功: {}", response));
            0
        }
        Err(e) => {
            solosoul_plugin_sdk::log_error(&format!("网络错误: {:?}", e));
            -3
        }
    }
}
```
